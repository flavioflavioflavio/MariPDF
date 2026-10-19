// ui/page_scale.rs — Tamanho na tela × resolução do bitmap (BF-402).
//
// Tamanho exibido depende só do zoom: 100% = tamanho físico do documento
// (1 pt PDF = 1/72" e 1 ponto de UI = 1/96" em escala 100% do Windows).
// A resolução do bitmap depende do zoom, da densidade da tela e da qualidade
// escolhida, limitada para não estourar a textura da GPU nem a memória: se o
// limite atuar, a página mantém o tamanho e só perde nitidez.

use eframe::egui;

/// Pontos de UI por ponto PDF em 100%.
pub const UI_POINTS_PER_PDF_POINT: f32 = 96.0 / 72.0;
/// Maior lado de bitmap aceito (limite comum de textura 2D no wgpu).
pub const MAX_RASTER_SIDE: f32 = 8192.0;
/// Teto de pixels por bitmap (~160 MB em RGBA).
pub const MAX_RASTER_PIXELS: f32 = 40_000_000.0;

/// Pontos de UI por ponto PDF no zoom informado.
pub fn display_scale(zoom: f32) -> f32 {
    zoom * UI_POINTS_PER_PDF_POINT
}

/// Pixels de bitmap por ponto PDF: zoom × densidade da tela × qualidade.
pub fn raster_scale(zoom: f32, pixels_per_ui_point: f32) -> f32 {
    display_scale(zoom) * pixels_per_ui_point
}

/// Tamanho exibido da página em pontos de UI, limitado à largura útil.
pub fn display_size(page_pts: egui::Vec2, zoom: f32, max_width: f32) -> egui::Vec2 {
    let natural = page_pts * display_scale(zoom);
    if max_width > 0.0 && natural.x > max_width {
        natural * (max_width / natural.x)
    } else {
        natural
    }
}

/// Fração mínima da resolução desejada aceita no cache antes de re-renderizar
/// (ex.: depois de maximizar a janela).
pub const UPGRADE_RATIO: f32 = 0.85;

/// Zoom que de fato chega à tela: a página nunca fica mais larga que a área
/// útil, então rasterizar acima disso só gasta memória e tempo (BF-3: 4× menos
/// pixels e 3-4× mais rápido a 400%, ver doc/benchmarks).
pub fn effective_zoom(zoom: f32, page_width_pts: Option<f32>, usable_width: f32) -> f32 {
    match page_width_pts {
        Some(width) if width > 0.0 && usable_width > 0.0 => {
            zoom.min(usable_width / display_scale(width))
        }
        _ => zoom,
    }
}

/// Escala efetiva de rasterização respeitando os limites de textura e memória.
pub fn capped_raster_scale(width_pts: f32, height_pts: f32, requested: f32) -> f32 {
    let longest = width_pts.max(height_pts).max(1.0);
    let area = (width_pts * height_pts).max(1.0);
    requested
        .min(MAX_RASTER_SIDE / longest)
        .min((MAX_RASTER_PIXELS / area).sqrt())
        .max(f32::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: egui::Vec2 = egui::vec2(595.0, 842.0);

    #[test]
    fn hundred_percent_is_physical_size() {
        let size = display_size(A4, 1.0, f32::INFINITY);
        // 595pt = 8,26" → 793 pontos de UI a 96/polegada.
        assert!((size.x - 793.33).abs() < 0.01, "{size:?}");
    }

    #[test]
    fn display_size_does_not_depend_on_raster_density() {
        // A mesma chamada vale para qualquer qualidade/DPI: só o zoom importa.
        assert_eq!(display_size(A4, 1.5, 2_000.0), A4 * display_scale(1.5));
    }

    #[test]
    fn display_size_is_limited_to_available_width_keeping_aspect() {
        let size = display_size(A4, 3.0, 600.0);
        assert!((size.x - 600.0).abs() < 1e-3, "{size:?}");
        assert!((size.y / size.x - 842.0 / 595.0).abs() < 1e-4);
    }

    #[test]
    fn raster_scale_combines_zoom_density_and_quality() {
        // 125% de escala do Windows e qualidade 1,5×.
        let density = 1.25 * 1.5;
        assert!((raster_scale(1.0, density) - UI_POINTS_PER_PDF_POINT * density).abs() < 1e-6);
    }

    #[test]
    fn zoom_beyond_the_window_width_does_not_raise_resolution() {
        // A4 numa área útil de 1200 pontos: cabe até ~151% de zoom.
        let fit = 1200.0 / display_scale(595.0);
        assert!((effective_zoom(4.0, Some(595.0), 1200.0) - fit).abs() < 1e-5);
        assert_eq!(effective_zoom(1.0, Some(595.0), 1200.0), 1.0);
        // Sem largura útil conhecida, não limita (o viewer estima a da página).
        assert_eq!(effective_zoom(4.0, None, 1200.0), 4.0);
        assert_eq!(effective_zoom(4.0, Some(595.0), 0.0), 4.0);
    }

    #[test]
    fn huge_requests_are_capped_to_texture_and_memory_limits() {
        let capped = capped_raster_scale(A4.x, A4.y, 20.0);
        assert!(A4.y * capped <= MAX_RASTER_SIDE + 0.5);
        assert!(A4.x * capped * A4.y * capped <= MAX_RASTER_PIXELS + 1.0);
        // Pedidos normais passam intactos.
        assert_eq!(capped_raster_scale(A4.x, A4.y, 2.0), 2.0);
    }
}
