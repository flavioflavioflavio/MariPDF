// ui/viewer_helpers.rs — Funções puras de renderização do viewer
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: upload de textura, fit de página, desenho e spinner.

use crate::pdf::engine::RenderedPage;
use crate::ui::page_rotation::{PageMapping, Rotation};
use crate::ui::viewer::SearchHighlight;
use eframe::egui;

/// Converte `[left, bottom, right, top]` em pontos PDF para a tela.
pub(super) fn pdf_rect_to_screen(
    page_rect: egui::Rect,
    page_size: (f32, f32),
    rect: [f32; 4],
    rotation: Rotation,
) -> egui::Rect {
    PageMapping::new(page_rect, page_size, rotation).rect_to_screen(rect)
}

/// Desenha a fita de marcador (retângulo com o corte em V embaixo) em
/// `rect`. Cheia quando marcada; só contorno quando é uma prévia.
pub(crate) fn paint_ribbon(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    filled: bool,
) {
    let notch = egui::pos2(rect.center().x, rect.bottom() - rect.height() * 0.28);
    if filled {
        // Polígono côncavo dividido em duas partes convexas.
        painter.add(egui::Shape::convex_polygon(
            vec![
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                notch,
            ],
            color,
            egui::Stroke::NONE,
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![rect.left_top(), notch, rect.left_bottom()],
            color,
            egui::Stroke::NONE,
        ));
    } else {
        painter.add(egui::Shape::closed_line(
            vec![
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                notch,
                rect.left_bottom(),
            ],
            egui::Stroke::new(1.5_f32, color),
        ));
    }
}

/// Desenho da fita no canto superior direito da página.
fn ribbon_shape(page_rect: egui::Rect) -> egui::Rect {
    let width = (page_rect.width() * 0.035).clamp(12.0, 22.0);
    let height = width * 1.6;
    let right = page_rect.right() - width * 0.9;
    egui::Rect::from_min_max(
        egui::pos2(right - width, page_rect.top()),
        egui::pos2(right, page_rect.top() + height),
    )
}

/// Área clicável da fita. Também usada para que um clique nela nunca siga
/// um link do PDF que esteja embaixo.
pub(crate) fn ribbon_hot_rect(page_rect: egui::Rect) -> egui::Rect {
    ribbon_shape(page_rect).expand2(egui::vec2(10.0, 8.0))
}

/// Fita de marcador no canto superior direito da página: cheia quando a
/// página está marcada, contorno discreto com o mouse sobre a página. Devolve
/// true quando clicada (marcar/desmarcar).
pub(crate) fn page_ribbon(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    page: usize,
    marked: bool,
    color: egui::Color32,
) -> bool {
    if !page_rect.is_positive() {
        return false;
    }
    let shape = ribbon_shape(page_rect);
    let hot = ribbon_hot_rect(page_rect);
    let response = ui.interact(
        hot,
        egui::Id::new(("page_ribbon", page)),
        egui::Sense::click(),
    );
    let over_page = ui.rect_contains_pointer(page_rect);
    let painter = ui.painter();
    if marked {
        let fill = if response.hovered() {
            color.gamma_multiply(0.85)
        } else {
            color
        };
        paint_ribbon(painter, shape, fill, true);
    } else if response.hovered() {
        paint_ribbon(painter, shape, color.gamma_multiply(0.45), true);
    } else if over_page {
        paint_ribbon(painter, shape, color.gamma_multiply(0.55), false);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hint = if marked {
        crate::t!("ribbon-remove-hint")
    } else {
        crate::t!("ribbon-add-hint")
    };
    response.on_hover_text(hint).clicked()
}

/// Pinta a ocorrência ativa da busca se ela pertence à página desenhada.
pub(crate) fn paint_search_highlight(
    ui: &egui::Ui,
    page_rect: egui::Rect,
    highlight: Option<&SearchHighlight>,
    page_index: Option<usize>,
    rotation: Rotation,
) {
    let Some(highlight) = highlight.filter(|h| Some(h.page_index) == page_index) else {
        return;
    };
    let (width, height) = highlight.page_size;
    if !page_rect.is_positive() || width <= 0.0 || height <= 0.0 {
        return;
    }
    let fill = egui::Color32::from_rgba_unmultiplied(255, 196, 0, 90);
    let stroke = egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 160, 0));
    for rect in &highlight.rects {
        let screen =
            pdf_rect_to_screen(page_rect, highlight.page_size, *rect, rotation).expand(1.5);
        ui.painter().rect(
            screen,
            egui::CornerRadius::same(2),
            fill,
            stroke,
            egui::StrokeKind::Outside,
        );
    }
}

/// Faz upload de um RenderedPage para a GPU como TextureHandle.
pub(super) fn upload_texture(
    ctx: &egui::Context,
    page: &RenderedPage,
    idx: usize,
) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [page.width as usize, page.height as usize],
        &page.rgba,
    );
    ctx.load_texture(format!("pdf_{idx}"), image, egui::TextureOptions::LINEAR)
}

/// Desenha uma página com sombra, tint e borda tema-aware.
/// Grava o rect alocado em `viewer` para hit-test de links pelo caller.
pub(super) fn draw_page_tex(
    viewer: &mut egui::Rect,
    ui: &mut egui::Ui,
    tex: &egui::TextureHandle,
    size: egui::Vec2,
    tint: egui::Color32,
    border: egui::Color32,
) {
    let (_, rect) = ui.allocate_space(size);
    *viewer = rect;

    // Cursor I-beam sobre a área da página
    if let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
        if rect.contains(pos) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }

    // Sombra suave (3 camadas crescentes de baixa opacidade)
    for (offset, alpha) in [(3.0_f32, 18_u8), (6.0, 12), (10.0, 6)] {
        ui.painter().rect_filled(
            rect.translate(egui::vec2(offset * 0.4, offset)),
            egui::CornerRadius::same(2),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, alpha),
        );
    }

    // Página com tint multiplicado (WHITE = sem filtro, Sepia = amarelo quente)
    ui.painter().image(
        tex.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        tint,
    );

    // Borda fina adaptada ao tema
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(0.5_f32, border),
        egui::StrokeKind::Outside,
    );
}

/// Spinner centralizado verticalmente para página ainda não renderizada.
pub(super) fn show_spinner(ui: &mut egui::Ui, available_h: f32) {
    ui.vertical_centered(|ui| {
        ui.add_space(available_h / 2.0 - 24.0);
        ui.spinner();
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(crate::t!("status-rendering"))
                .size(13.0)
                .color(egui::Color32::from_rgb(90, 90, 110)),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_rect_maps_with_bottom_left_origin() {
        let page = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(300.0, 400.0));
        // Página 600x800pt exibida em 300x400px: escala 0,5.
        let screen = pdf_rect_to_screen(
            page,
            (600.0, 800.0),
            [100.0, 700.0, 200.0, 760.0],
            Rotation::R0,
        );
        assert_eq!(screen.min, egui::pos2(150.0, 70.0));
        assert_eq!(screen.max, egui::pos2(200.0, 100.0));
    }
}
