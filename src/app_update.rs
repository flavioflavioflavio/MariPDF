// app_update.rs — Loop principal de atualização do eframe (antigo update() em app.rs)
// Extraído para manter app.rs abaixo de 250 linhas.

use crate::app::{MariApp, SidePanelKind, Toast};
use crate::pdf::document_worker::{
    DocumentJobOutput, DocumentJobTarget, JobCompletion, JobOutcome,
};
use crate::pdf::engine::PdfLinkInfo;
use crate::ui::page_rotation::PageMapping;
use crate::ui::viewer::ViewMode;
use eframe::egui;

/// Chamado dentro de `eframe::App::update`.
pub(crate) fn run(app: &mut MariApp, ctx: &egui::Context) {
    // Fechar documento pendente
    if app.pending_close {
        app.pending_close = false;
        app.exit_frame_mode(ctx);
        app.current_file = None;
        app.document_worker = None;
        app.engine = None;
        app.viewer.reset();
        app.render_error = None;
        app.search.detach_document();
        app.search_open = false;
        app.goto_open = false;
        app.page_links = Vec::new();
        app.links_page = usize::MAX;
        app.bookmarks = Vec::new();
        app.text_chars = Vec::new();
        app.text_chars_page = usize::MAX;
        app.sel_anchor = None;
        app.sel_focus = None;
        app.pending_select_all = false;
        app.thumb_cache = None;
        app.side_panel = SidePanelKind::None;
        // Diálogos e estado ligados ao documento fechado.
        app.properties = None;
        app.page_copy = None;
        if app.password_prompt.as_ref().is_some_and(|p| p.reload) {
            app.password_prompt = None;
        }
        app.doc_password = None;
        app.marks.clear();
        app.nav.clear();
    }

    // Drena bitmaps de thumbnails prontos
    if let Some(ref mut tc) = app.thumb_cache {
        if tc.drain_ready() {
            ctx.request_repaint();
        }
    }

    // Avisos de persistência/integração vindos de fora da UI: um por vez,
    // o próximo só depois que o toast atual expirar (nenhum se perde).
    let now = ctx.input(|i| i.time);
    if app.toast.as_ref().is_none_or(|toast| !toast.is_alive(now)) {
        if let Some(notice) = crate::notices::pop() {
            app.toast = Some(match notice.kind {
                crate::notices::NoticeKind::Info => Toast::info(notice.message, 4.0, now),
                crate::notices::NoticeKind::Error => Toast::error(notice.message, 5.0, now),
            });
        }
    }
    if crate::notices::has_pending() {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    // Arquivo pendente
    if let Some(path) = app.pending_open.take() {
        let c = ctx.clone();
        app.open_file(&c, path);
    }

    let document_results = app
        .document_worker
        .as_mut()
        .map(|worker| worker.drain_results())
        .unwrap_or_default();
    for completion in document_results {
        apply_document_completion(app, ctx, completion);
    }

    // Rerender pendente (troca de tema)
    if app.pending_rerender && app.engine.is_some() {
        app.pending_rerender = false;
        let c = ctx.clone();
        app.render_current_page(&c);
    }

    // Drag & drop
    ctx.input(|i| {
        app.drop_zone_hovered = !i.raw.hovered_files.is_empty();
        if let Some(f) = i.raw.dropped_files.first() {
            if let Some(p) = f.path.clone() {
                if p.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
                {
                    app.pending_open = Some(p);
                }
            }
        }
    });

    // ── Atalhos globais ───────────────────────────────────────────────────────
    let (mut go_next, mut go_prev, mut zoom_in, mut zoom_out) = (false, false, false, false);
    let (mut go_home, mut go_end, mut toggle_fs, mut fit_w) = (false, false, false, false);
    let (mut fit_p, mut open_goto, mut toggle_mode, mut toggle_frame) =
        (false, false, false, false);
    let (mut open_search, mut search_next, mut search_prev) = (false, false, false);
    let (mut escape, mut select_all, mut copy_text, mut open_about) = (false, false, false, false);
    let (mut nav_back, mut nav_forward, mut rotate_cw, mut rotate_ccw) =
        (false, false, false, false);
    let (mut toggle_mark, mut copy_page, mut open_properties) = (false, false, false);
    // Com o campo de senha aberto, setas/Home/End editam o texto: nenhum
    // atalho do leitor pode mexer no documento por baixo.
    let modal = app.password_prompt.is_some();

    ctx.input(|i| {
        if modal {
            return;
        }
        let ctrl = i.modifiers.ctrl;
        let shift = i.modifiers.shift;
        let alt = i.modifiers.alt;
        go_next =
            (i.key_pressed(egui::Key::ArrowRight) && !alt) || i.key_pressed(egui::Key::PageDown);
        go_prev = (i.key_pressed(egui::Key::ArrowLeft) && !alt) || i.key_pressed(egui::Key::PageUp);
        // Alt+← / Alt+→ e os botões laterais do mouse navegam no histórico.
        nav_back = (alt && i.key_pressed(egui::Key::ArrowLeft))
            || i.pointer.button_pressed(egui::PointerButton::Extra1);
        nav_forward = (alt && i.key_pressed(egui::Key::ArrowRight))
            || i.pointer.button_pressed(egui::PointerButton::Extra2);
        rotate_cw = ctrl && !shift && i.key_pressed(egui::Key::R);
        rotate_ccw = ctrl && shift && i.key_pressed(egui::Key::R);
        toggle_mark = ctrl && i.key_pressed(egui::Key::D);
        copy_page = ctrl && shift && i.key_pressed(egui::Key::C);
        open_properties = alt && i.key_pressed(egui::Key::Enter);
        go_home = i.key_pressed(egui::Key::Home);
        go_end = i.key_pressed(egui::Key::End);
        toggle_fs = i.key_pressed(egui::Key::F11);
        zoom_in = ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
        zoom_out = ctrl && i.key_pressed(egui::Key::Minus);
        fit_w = i.key_pressed(egui::Key::W) && !ctrl;
        fit_p = ctrl && i.key_pressed(egui::Key::Num0);
        open_goto = ctrl && i.key_pressed(egui::Key::G);
        toggle_mode = ctrl && shift && i.key_pressed(egui::Key::M);
        open_search = ctrl && !shift && i.key_pressed(egui::Key::F);
        toggle_frame = ctrl && shift && i.key_pressed(egui::Key::F);
        search_next = i.key_pressed(egui::Key::F3) && !shift;
        search_prev = i.key_pressed(egui::Key::F3) && shift;
        escape = i.key_pressed(egui::Key::Escape);
        select_all = ctrl && i.key_pressed(egui::Key::A);
        copy_text = ctrl && !shift && i.key_pressed(egui::Key::C);
        open_about = ctrl && shift && i.key_pressed(egui::Key::I);
    });

    if toggle_frame && app.engine.is_some() {
        app.toggle_frame_mode(ctx);
    }
    // Esc sai do modo Flutuante quando nenhum overlay está aberto.
    if escape && app.frame.active && !app.goto_open && !app.search_open {
        app.exit_frame_mode(ctx);
    }

    if toggle_fs && !app.frame.active {
        app.fullscreen = !app.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.fullscreen));
    }

    let zoom_delta = std::mem::replace(&mut app.viewer.zoom_delta, 0.0);
    let page_delta = std::mem::replace(&mut app.viewer.page_delta, 0);
    if zoom_delta != 0.0 {
        app.apply_zoom(ctx, zoom_delta);
    }
    if zoom_in {
        app.apply_zoom(ctx, 0.15);
    }
    if zoom_out {
        app.apply_zoom(ctx, -0.15);
    }

    if open_goto {
        if app.search_open {
            app.search_open = false;
        }
        app.goto_open = !app.goto_open;
        app.goto_input.clear();
    }
    if open_search && app.engine.is_some() {
        if app.goto_open {
            app.goto_open = false;
        }
        app.search_open = !app.search_open;
        app.search_focused = app.search_open;
        if !app.search_open {
            app.search.clear();
        }
    }
    if search_next && app.search_open {
        app.search.go_next();
    }
    if search_prev && app.search_open {
        app.search.go_prev();
    }
    if escape {
        app.goto_open = false;
        app.search_open = false;
        app.search.clear();
        app.clear_selection();
    }
    if (search_next || search_prev) && app.search_open {
        if let Some(hit) = app.search.current() {
            let pg = hit.page_index;
            let c = ctx.clone();
            app.go_to_page(&c, pg);
        }
    }

    if fit_w {
        let c = ctx.clone();
        app.fit_width(&c);
    }
    if fit_p {
        let c = ctx.clone();
        app.fit_page(&c);
    }

    if app.engine.is_some() {
        if page_delta != 0 && app.viewer.mode == ViewMode::SinglePage {
            let p = (app.current_page as i64 + page_delta as i64)
                .clamp(0, app.page_count as i64 - 1) as usize;
            let c = ctx.clone();
            app.turn_page(&c, p);
        }
        if go_next && app.current_page + 1 < app.page_count {
            let p = app.current_page + 1;
            let c = ctx.clone();
            app.turn_page(&c, p);
        }
        if go_prev && app.current_page > 0 {
            let p = app.current_page - 1;
            let c = ctx.clone();
            app.turn_page(&c, p);
        }
        if go_home {
            let c = ctx.clone();
            app.go_to_page(&c, 0);
        }
        if go_end {
            let last = app.page_count.saturating_sub(1);
            let c = ctx.clone();
            app.go_to_page(&c, last);
        }
        if toggle_mode && !app.frame.active {
            let c = ctx.clone();
            app.do_toggle_mode(&c);
        }
        if nav_back {
            app.nav_back(ctx);
        }
        if nav_forward {
            app.nav_forward(ctx);
        }
        if rotate_cw || rotate_ccw {
            app.rotate_view(ctx, rotate_cw);
        }
        if toggle_mark {
            app.toggle_mark(ctx);
        }
        if copy_page {
            app.copy_page_image(ctx);
        }
        if open_properties {
            app.open_properties();
        }
    }
    app.poll_file_changes(ctx);
    app.poll_page_copy(ctx);

    if select_all && app.engine.is_some() && !app.search_open && !app.goto_open {
        app.select_all_text(ctx);
    }
    if copy_text && !app.search_open && !app.goto_open {
        app.copy_selection(ctx);
    }

    // Cache de links
    if app.engine.is_some() && app.links_page != app.current_page {
        app.page_links.clear();
        if let Some(worker) = app.document_worker.as_mut() {
            worker.request_links(app.current_page);
        }
    }
    let current_texture_is_visible =
        app.viewer.mode == ViewMode::SinglePage && app.viewer.loaded_page == Some(app.current_page);
    if current_texture_is_visible && !app.page_links.is_empty() {
        let maybe_click = ctx.input(|i| {
            if i.pointer.button_clicked(egui::PointerButton::Primary) {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        if let Some(click_pos) = maybe_click {
            let page_rect = app.viewer.last_page_rect;
            if let Some(engine) = &app.engine {
                let page_size = engine.page_size_pts(app.current_page);
                let mapping = PageMapping::new(page_rect, page_size, app.viewer.rotation);
                if let Some(link) = hit_test_link(&app.page_links, click_pos, &mapping) {
                    match link.page {
                        // Link interno: salto registrado no histórico.
                        Some(page) => {
                            let c = ctx.clone();
                            app.go_to_page(&c, page);
                        }
                        None => {
                            let _ = open::that(&link.uri);
                        }
                    }
                }
            }
        }
    }

    // ── Renderiza UI ──────────────────────────────────────────────────────────
    if app.current_file.is_none() {
        app.ui_home(ctx);
    } else {
        app.ui_reader(ctx);
    }
    if app.goto_open && app.engine.is_some() {
        app.ui_goto_overlay(ctx);
    }
    app.ui_properties(ctx);
    app.ui_password_prompt(ctx);
    app.ui_toast(ctx);
    if app.search_open && app.engine.is_some() {
        app.ui_search_bar(ctx);
    }
    if open_about {
        app.settings_open = true;
        crate::ui::settings_panel::select_about_tab(ctx);
    }
    if app.settings_open {
        let effects = crate::ui::settings_panel::show_settings_panel(
            ctx,
            &mut app.settings_open,
            &mut app.settings,
            &mut app.persistence,
            &mut app.color_mode,
            app.system_prefers_dark,
            &mut app.about,
        );
        if effects.font_reload {
            crate::apply_font_settings(ctx, app.settings.font);
            app.settings.save();
        }
        if effects.render_reload {
            app.invalidate_render(ctx);
            app.settings.save();
        }
    }
    persist_theme_change(app);
}

/// Toolbar, Home e Configurações alteram `color_mode` diretamente; aqui a
/// mudança vira preferência persistida, preservando "Sistema" quando possível.
fn persist_theme_change(app: &mut MariApp) {
    let theme = app
        .settings
        .theme
        .after_mode_change(app.color_mode, app.system_prefers_dark);
    if theme != app.settings.theme {
        app.settings.theme = theme;
        app.settings.save();
    }
}

fn hit_test_link<'a>(
    links: &'a [PdfLinkInfo],
    click: egui::Pos2,
    mapping: &PageMapping,
) -> Option<&'a PdfLinkInfo> {
    if !mapping.is_valid() || !mapping.rect.contains(click) {
        return None;
    }
    let (pdf_x, pdf_y) = mapping.pdf_point(click);
    links.iter().find(|link| {
        let [left, bottom, right, top] = link.rect;
        pdf_x >= left && pdf_x <= right && pdf_y >= bottom && pdf_y <= top
    })
}

fn apply_document_completion(app: &mut MariApp, ctx: &egui::Context, completion: JobCompletion) {
    match completion.outcome {
        JobOutcome::Ready(DocumentJobOutput::Bookmarks(bookmarks)) => {
            app.bookmarks = bookmarks;
        }
        JobOutcome::Ready(DocumentJobOutput::Links { page_index, links }) => {
            if page_index == app.current_page {
                app.page_links = links;
                app.links_page = page_index;
            }
        }
        JobOutcome::Ready(DocumentJobOutput::Chars { page_index, chars }) => {
            if page_index == app.current_page {
                app.text_chars = chars;
                app.text_chars_page = page_index;
                app.clear_selection();
                if app.pending_select_all {
                    app.select_all_text(ctx);
                }
            }
        }
        JobOutcome::Ready(
            DocumentJobOutput::Render { .. }
            | DocumentJobOutput::Thumbnail { .. }
            | DocumentJobOutput::SearchText { .. },
        ) => {
            tracing::warn!("document_job: resultado entregue ao receptor incorreto");
        }
        JobOutcome::Failed(error) => {
            tracing::warn!(
                job_id = completion.id,
                target = ?completion.target,
                error,
                "document_job: falha"
            );
            if matches!(completion.target, DocumentJobTarget::Chars(page) if page == app.current_page)
                && app.pending_select_all
            {
                app.pending_select_all = false;
                let now = ctx.input(|input| input.time);
                app.toast = Some(Toast::error(crate::t!("toast-no-text"), 2.5, now));
            }
        }
        JobOutcome::Cancelled => {}
    }
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::page_rotation::Rotation;

    fn link(uri: &str, rect: [f32; 4]) -> PdfLinkInfo {
        PdfLinkInfo {
            uri: uri.to_string(),
            page: None,
            rect,
        }
    }

    #[test]
    fn hit_test_maps_screen_to_pdf_coordinates_for_multiple_links() {
        let links = vec![
            link("https://first.test", [10.0, 10.0, 30.0, 30.0]),
            link("https://second.test", [60.0, 150.0, 90.0, 190.0]),
        ];
        let page = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 400.0));

        let first = hit_test_link(
            &links,
            egui::pos2(40.0, 360.0),
            &PageMapping::new(page, (100.0, 200.0), Rotation::R0),
        );
        let second = hit_test_link(
            &links,
            egui::pos2(150.0, 60.0),
            &PageMapping::new(page, (100.0, 200.0), Rotation::R0),
        );

        assert_eq!(
            first.map(|link| link.uri.as_str()),
            Some("https://first.test")
        );
        assert_eq!(
            second.map(|link| link.uri.as_str()),
            Some("https://second.test")
        );
    }

    #[test]
    fn hit_test_follows_the_page_rotation() {
        let links = vec![link("https://corner.test", [0.0, 180.0, 20.0, 200.0])];
        // Página 100×200 girada 90°: na tela fica 400×200 (escala 2) e o canto
        // superior esquerdo do PDF vai para o canto superior direito.
        let page = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 200.0));
        let mapping = PageMapping::new(page, (100.0, 200.0), Rotation::R90);
        assert!(hit_test_link(&links, egui::pos2(395.0, 5.0), &mapping).is_some());
        assert!(hit_test_link(&links, egui::pos2(5.0, 5.0), &mapping).is_none());
    }

    #[test]
    fn hit_test_rejects_clicks_outside_page_or_links() {
        let links = vec![link("https://only.test", [10.0, 10.0, 30.0, 30.0])];
        let page = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(210.0, 410.0));

        assert!(hit_test_link(
            &links,
            egui::pos2(5.0, 5.0),
            &PageMapping::new(page, (100.0, 200.0), Rotation::R0)
        )
        .is_none());
        assert!(hit_test_link(
            &links,
            egui::pos2(150.0, 200.0),
            &PageMapping::new(page, (100.0, 200.0), Rotation::R0)
        )
        .is_none());
    }
}
