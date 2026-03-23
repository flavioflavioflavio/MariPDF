// ui/viewer_helpers.rs — Funções puras de renderização do viewer
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: upload de textura, fit de página, desenho e spinner.

use eframe::egui;
use crate::pdf::engine::RenderedPage;

/// Faz upload de um RenderedPage para a GPU como TextureHandle.
pub(super) fn upload_texture(ctx: &egui::Context, page: &RenderedPage, idx: usize) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [page.width as usize, page.height as usize],
        &page.rgba,
    );
    ctx.load_texture(
        format!("pdf_{idx}"),
        image,
        egui::TextureOptions::LINEAR,
    )
}

/// Calcula (display_w, display_h) respeitando max_w, mantendo aspect ratio.
pub(super) fn fit_page(tex_size: egui::Vec2, max_w: f32) -> (f32, f32) {
    let w = tex_size.x.min(max_w.max(100.0));
    let h = tex_size.y * (w / tex_size.x);
    (w, h)
}

/// Desenha uma página com sombra, tint e borda tema-aware.
/// Grava o rect alocado em `viewer` para hit-test de links pelo caller.
pub(super) fn draw_page_tex(
    viewer: &mut egui::Rect,
    ui:     &mut egui::Ui,
    tex:    &egui::TextureHandle,
    size:   egui::Vec2,
    tint:   egui::Color32,
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
        egui::Stroke::new(0.5, border),
        egui::StrokeKind::Outside,
    );
}

/// Spinner centralizado verticalmente para página ainda não renderizada.
pub(super) fn show_spinner(ui: &mut egui::Ui, available_h: f32) {
    ui.vertical_centered(|ui| {
        ui.add_space(available_h / 2.0 - 24.0);
        ui.spinner();
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Renderizando...")
            .size(13.0).color(egui::Color32::from_rgb(90, 90, 110)));
    });
}

/// Inversão suave (dark mode PDF): branco → grafite, preto → off-white.
/// Fórmula: `38 + ((255 - x) * 205 / 255)`. Alpha intacto.
pub fn invert_rgba(rgba: &mut Vec<u8>) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = 38 + ((255u16 - pixel[0] as u16) * 205 / 255) as u8;
        pixel[1] = 38 + ((255u16 - pixel[1] as u16) * 205 / 255) as u8;
        pixel[2] = 38 + ((255u16 - pixel[2] as u16) * 205 / 255) as u8;
        // pixel[3] = Alpha — não altera
    }
}
