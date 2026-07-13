// ui/viewer_continuous.rs — Modo de rolagem contínua do viewer PDF
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: show_continuous() com prefetch, skeleton e virtual scrolling.

use crate::ui::viewer::PdfViewer;
use crate::ui::viewer_helpers::{draw_page_tex, fit_page, invert_rgba, upload_texture};
use eframe::egui;

/// Quantas páginas adjacentes pré-renderizar no modo Contínuo
const PREFETCH_AHEAD: usize = 3;
const PREFETCH_BEHIND: usize = 1;
impl PdfViewer {
    /// Renderiza somente a janela visível do scroll contínuo, com prefetch
    /// assíncrono. Páginas fora do viewport ocupam apenas espaço virtual.
    pub(super) fn show_continuous(
        &mut self,
        ui: &mut egui::Ui,
        page_count: usize,
        page_tint: egui::Color32,
        page_border: egui::Color32,
    ) {
        if page_count == 0 {
            return;
        }

        // Ctrl+Scroll → zoom
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let scroll_y = ui.input(|i| {
            if self.smooth_scroll {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y
            }
        });
        if ctrl && scroll_y != 0.0 {
            self.zoom_delta = scroll_y.signum() * 0.15;
        }

        let scale = self.zoom * self.render_scale;
        let available_w = ui.available_width();
        let ctx = ui.ctx().clone();

        // Drena bitmaps prontos da thread de background
        if self.page_cache.drain_ready() {
            ctx.request_repaint();
        }

        let est_page_h = self.estimate_page_height(available_w);
        let stride = est_page_h + self.page_margin;
        let total_h = self.page_margin + stride * page_count as f32;
        self.last_page_rect = egui::Rect::NOTHING;
        self.is_loading = false;

        egui::ScrollArea::vertical()
            .id_salt("continuous_scroll")
            .auto_shrink(false)
            .show_viewport(ui, |ui, viewport| {
                ui.set_min_height(total_h);

                let (first_visible, last_visible) =
                    visible_page_range(viewport.min.y, viewport.max.y, stride, page_count);
                let render_first = first_visible.saturating_sub(PREFETCH_BEHIND);
                let render_last = (last_visible + PREFETCH_AHEAD).min(page_count.saturating_sub(1));

                // Evicta texturas fora da janela estendida do viewport
                self.tex_cache
                    .retain(|&idx, _| idx >= render_first && idx <= render_last);
                self.page_cache.evict_range(render_first, render_last);

                // Solicita pré-render das páginas vizinhas
                for idx in render_first..=render_last {
                    self.page_cache.prefetch(idx, scale);
                }

                for idx in first_visible..=last_visible {
                    // Upload para GPU só quando a página chega ao viewport.
                    if !self.tex_cache.contains_key(&idx) {
                        if let Some(bitmap) = self.page_cache.get(idx) {
                            let tex = if self.invert_colors {
                                let mut inverted = (*bitmap).clone();
                                invert_rgba(&mut inverted.rgba);
                                upload_texture(&ctx, &inverted, idx)
                            } else {
                                upload_texture(&ctx, bitmap, idx)
                            };
                            self.tex_cache.insert(idx, tex);
                        } else if let Some(error) = self.page_cache.take_failure(idx) {
                            self.last_error = Some(error);
                        }
                    }

                    let y_top = ui.max_rect().top() + self.page_margin + idx as f32 * stride;
                    let slot_rect = egui::Rect::from_min_size(
                        egui::pos2(ui.max_rect().left(), y_top),
                        egui::vec2(available_w, est_page_h),
                    );
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(slot_rect), |ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(tex) = self.tex_cache.get(&idx) {
                                let (mut w, mut h) =
                                    fit_page(tex.size_vec2(), available_w - self.page_margin * 2.0);
                                if h > est_page_h {
                                    let ratio = est_page_h / h;
                                    w *= ratio;
                                    h = est_page_h;
                                }
                                let mut page_rect = egui::Rect::NOTHING;
                                draw_page_tex(
                                    &mut page_rect,
                                    ui,
                                    tex,
                                    egui::vec2(w, h),
                                    page_tint,
                                    page_border,
                                );
                                if self.last_page_rect == egui::Rect::NOTHING {
                                    self.last_page_rect = page_rect;
                                }
                            } else {
                                self.is_loading = true;
                                draw_skeleton(
                                    ui,
                                    est_page_h,
                                    available_w,
                                    self.page_margin,
                                    idx,
                                    true,
                                );
                            }
                        });
                    });
                }
            });
    }
}

fn visible_page_range(
    viewport_min: f32,
    viewport_max: f32,
    stride: f32,
    page_count: usize,
) -> (usize, usize) {
    let last_page = page_count.saturating_sub(1);
    let first = (viewport_min.max(0.0) / stride).floor() as usize;
    let last = (viewport_max.max(0.0) / stride).ceil() as usize;
    (first.min(last_page), last.max(first).min(last_page))
}

/// Skeleton animado (shimmer) exibido enquanto a página ainda não renderizou.
fn draw_skeleton(
    ui: &mut egui::Ui,
    ph: f32,
    available_w: f32,
    page_margin: f32,
    idx: usize,
    is_visible: bool,
) {
    let pw = (available_w - page_margin * 2.0).min(800.0);
    let (_, rect) = ui.allocate_space(egui::vec2(pw, ph));

    let t = ui.ctx().input(|i| i.time);
    // Pulso com fase diferente por página para não piscarem juntas
    let phase = (idx as f64) * 0.4;
    let alpha = ((t + phase).sin() * 0.15 + 0.70) as f32; // 0.55..0.85

    let base = egui::Color32::from_rgba_premultiplied(
        (38.0 * alpha) as u8,
        (38.0 * alpha) as u8,
        (50.0 * alpha) as u8,
        255,
    );
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), base);

    // Shimmer: 3 linhas fictícias de texto
    let stripe_color = egui::Color32::from_rgba_premultiplied(
        (60.0 * alpha) as u8,
        (60.0 * alpha) as u8,
        (80.0 * alpha) as u8,
        255,
    );
    for i in 0..3 {
        let y = rect.top() + 32.0 + (i as f32) * 18.0;
        let width = pw * if i == 2 { 0.5 } else { 0.72 };
        let sr =
            egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, y), egui::vec2(width, 10.0));
        ui.painter()
            .rect_filled(sr, egui::CornerRadius::same(4), stripe_color);
    }

    if is_visible {
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::visible_page_range;

    #[test]
    fn visible_range_is_limited_to_viewport() {
        assert_eq!(visible_page_range(2_000.0, 3_100.0, 1_000.0, 100), (2, 4));
    }

    #[test]
    fn visible_range_clamps_to_document_end() {
        assert_eq!(visible_page_range(9_000.0, 12_000.0, 1_000.0, 10), (9, 9));
    }
}
