// app_update.rs — Loop principal de atualização do eframe (antigo update() em app.rs)
// Extraído para manter app.rs abaixo de 250 linhas.

use crate::app::{MariApp, SidePanelKind, Toast};
use crate::ui::viewer::ViewMode;
use eframe::egui;

/// Chamado dentro de `eframe::App::update`.
pub(crate) fn run(app: &mut MariApp, ctx: &egui::Context) {
    // Fechar documento pendente
    if app.pending_close {
        app.pending_close = false;
        app.current_file = None;
        app.engine = None;
        app.viewer.reset();
        app.render_error = None;
        app.search.detach_document();
        app.search_open = false;
        app.goto_open = false;
        app.page_links = Vec::new();
        app.bookmarks = Vec::new();
        app.thumb_cache = None;
        app.side_panel = SidePanelKind::None;
    }

    // Drena bitmaps de thumbnails prontos
    if let Some(ref mut tc) = app.thumb_cache {
        if tc.drain_ready() {
            ctx.request_repaint();
        }
    }

    // Arquivo pendente
    if let Some(path) = app.pending_open.take() {
        let c = ctx.clone();
        app.open_file(&c, path);
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
                if p.extension().map(|e| e == "pdf").unwrap_or(false) {
                    app.pending_open = Some(p);
                }
            }
        }
    });

    // ── Atalhos globais ───────────────────────────────────────────────────────
    let (mut go_next, mut go_prev, mut zoom_in, mut zoom_out) = (false, false, false, false);
    let (mut go_home, mut go_end, mut toggle_fs, mut fit_w) = (false, false, false, false);
    let (mut fit_p, mut open_goto, mut toggle_mode) = (false, false, false);
    let (mut open_search, mut search_next, mut search_prev) = (false, false, false);
    let (mut escape, mut select_all, mut copy_text, mut open_about) = (false, false, false, false);

    ctx.input(|i| {
        let ctrl = i.modifiers.ctrl;
        let shift = i.modifiers.shift;
        go_next = i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown);
        go_prev = i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp);
        go_home = i.key_pressed(egui::Key::Home);
        go_end = i.key_pressed(egui::Key::End);
        toggle_fs = i.key_pressed(egui::Key::F11);
        zoom_in = ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
        zoom_out = ctrl && i.key_pressed(egui::Key::Minus);
        fit_w = i.key_pressed(egui::Key::W) && !ctrl;
        fit_p = ctrl && i.key_pressed(egui::Key::Num0);
        open_goto = ctrl && i.key_pressed(egui::Key::G);
        toggle_mode = ctrl && shift && i.key_pressed(egui::Key::M);
        open_search = ctrl && i.key_pressed(egui::Key::F);
        search_next = i.key_pressed(egui::Key::F3) && !shift;
        search_prev = i.key_pressed(egui::Key::F3) && shift;
        escape = i.key_pressed(egui::Key::Escape);
        select_all = ctrl && i.key_pressed(egui::Key::A);
        copy_text = ctrl && i.key_pressed(egui::Key::C);
        open_about = ctrl && shift && i.key_pressed(egui::Key::I);
    });

    if toggle_fs {
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
            app.go_to_page(&c, p);
        }
        if go_next && app.current_page + 1 < app.page_count {
            let p = app.current_page + 1;
            let c = ctx.clone();
            app.go_to_page(&c, p);
        }
        if go_prev && app.current_page > 0 {
            let p = app.current_page - 1;
            let c = ctx.clone();
            app.go_to_page(&c, p);
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
        if toggle_mode {
            let c = ctx.clone();
            app.do_toggle_mode(&c);
        }
    }

    if select_all && app.engine.is_some() {
        let now = ctx.input(|i| i.time);
        if let Some(engine) = &app.engine {
            match engine.extract_text(app.current_page) {
                Ok(text) if !text.trim().is_empty() => {
                    app.selected_text = Some(text);
                    app.toast = Some(Toast::info(
                        "Texto selecionado — Ctrl+C para copiar",
                        3.0,
                        now,
                    ));
                }
                Ok(_) => {
                    app.toast = Some(Toast::error("Página sem texto extraível", 2.5, now));
                }
                Err(_) => {
                    app.toast = Some(Toast::error("Falha ao extrair texto", 2.5, now));
                }
            }
        }
    }
    if copy_text {
        if let Some(ref text) = app.selected_text.clone() {
            let now = ctx.input(|i| i.time);
            ctx.copy_text(text.clone());
            app.toast = Some(Toast::info("Texto copiado!", 2.0, now));
        }
    }

    // Cache de links
    if app.engine.is_some() && app.links_page != app.current_page {
        app.page_links = app
            .engine
            .as_ref()
            .map(|e| e.extract_page_links(app.current_page))
            .unwrap_or_default();
        app.links_page = app.current_page;
    }
    if !app.page_links.is_empty() {
        let maybe_click = ctx.input(|i| {
            if i.pointer.button_clicked(egui::PointerButton::Primary) {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        if let Some(click_pos) = maybe_click {
            let page_rect = app.viewer.last_page_rect;
            if page_rect.is_positive() && page_rect.contains(click_pos) && app.page_links.len() == 1
            {
                let uri = app.page_links[0].uri.clone();
                let _ = open::that(uri);
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
    app.ui_toast(ctx);
    if app.search_open && app.engine.is_some() {
        app.ui_search_bar(ctx);
    }
    if open_about {
        app.about.open = !app.about.open;
    }
    app.about.show(ctx);
    if app.settings_open {
        let mut needs_font = false;
        let mut needs_render = false;
        crate::ui::settings_panel::show_settings_panel(
            ctx,
            &mut app.settings_open,
            &mut app.settings,
            &mut app.persistence,
            &mut app.color_mode,
            &mut needs_font,
            &mut needs_render,
        );
        if needs_font {
            crate::apply_font_settings(ctx, app.settings.font);
            app.settings.save();
        }
        if needs_render {
            app.invalidate_render(ctx);
            app.settings.save();
        }
    }
}
