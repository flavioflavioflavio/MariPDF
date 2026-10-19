// ui/viewer_helpers.rs — Funções puras de renderização do viewer
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: upload de textura, fit de página, desenho e spinner.

use crate::pdf::engine::RenderedPage;
use crate::ui::viewer::SearchHighlight;
use eframe::egui;

/// Converte `[left, bottom, right, top]` em pontos PDF para a tela.
pub(super) fn pdf_rect_to_screen(
    page_rect: egui::Rect,
    page_size: (f32, f32),
    [left, bottom, right, top]: [f32; 4],
) -> egui::Rect {
    let (width, height) = page_size;
    let x = |pdf_x: f32| page_rect.left() + pdf_x / width * page_rect.width();
    let y = |pdf_y: f32| page_rect.bottom() - pdf_y / height * page_rect.height();
    egui::Rect::from_min_max(egui::pos2(x(left), y(top)), egui::pos2(x(right), y(bottom)))
}

/// Pinta a ocorrência ativa da busca se ela pertence à página desenhada.
pub(crate) fn paint_search_highlight(
    ui: &egui::Ui,
    page_rect: egui::Rect,
    highlight: Option<&SearchHighlight>,
    page_index: Option<usize>,
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
        let screen = pdf_rect_to_screen(page_rect, highlight.page_size, *rect).expand(1.5);
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

/// Inversão suave (dark mode PDF): branco → grafite, preto → off-white.
/// Fórmula: `38 + ((255 - x) * 205 / 255)`. Alpha intacto.
pub fn invert_rgba(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[0] = 38 + ((255u16 - pixel[0] as u16) * 205 / 255) as u8;
        pixel[1] = 38 + ((255u16 - pixel[1] as u16) * 205 / 255) as u8;
        pixel[2] = 38 + ((255u16 - pixel[2] as u16) * 205 / 255) as u8;
        // pixel[3] = Alpha — não altera
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_rect_maps_with_bottom_left_origin() {
        let page = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(300.0, 400.0));
        // Página 600x800pt exibida em 300x400px: escala 0,5.
        let screen = pdf_rect_to_screen(page, (600.0, 800.0), [100.0, 700.0, 200.0, 760.0]);
        assert_eq!(screen.min, egui::pos2(150.0, 70.0));
        assert_eq!(screen.max, egui::pos2(200.0, 100.0));
    }
}
