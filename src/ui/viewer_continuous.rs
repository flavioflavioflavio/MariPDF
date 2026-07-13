// ui/viewer_continuous.rs — Modo de rolagem contínua do viewer PDF
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: show_continuous() com prefetch, skeleton e virtual scrolling.

use crate::pdf::engine::PdfEngine;
use crate::ui::viewer::PdfViewer;
use crate::ui::viewer_helpers::{draw_page_tex, fit_page, invert_rgba, upload_texture};
use eframe::egui;

/// Quantas páginas adjacentes pré-renderizar no modo Contínuo
const PREFETCH_AHEAD: usize = 3;
const PREFETCH_BEHIND: usize = 1;
impl PdfViewer {
    /// Renderiza todas as páginas em scroll contínuo com prefetch assíncrono.
    pub(super) fn show_continuous(
        &mut self,
        ui: &mut egui::Ui,
        engine: &PdfEngine,
        page_count: usize,
        page_tint: egui::Color32,
        page_border: egui::Color32,
    ) {
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

        // Converte RenderedPage disponibilizados pelo cache em TextureHandle
        for idx in 0..page_count {
            if !self.tex_cache.contains_key(&idx) {
                if let Some(bitmap) = self.page_cache.get(idx) {
                    if self.invert_colors {
                        let mut inverted = (*bitmap).clone();
                        invert_rgba(&mut inverted.rgba);
                        let tex = upload_texture(&ctx, &inverted, idx);
                        self.tex_cache.insert(idx, tex);
                    } else {
                        let tex = upload_texture(&ctx, bitmap, idx);
                        self.tex_cache.insert(idx, tex);
                    }
                }
            }
        }

        egui::ScrollArea::vertical()
            .id_salt("continuous_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(self.page_margin);

                let est_page_h = self.estimate_page_height(available_w);
                let scroll_top = ui.min_rect().top();
                let vp_h = ctx.screen_rect().height();

                let first_visible = ((scroll_top.abs() / (est_page_h + self.page_margin)) as usize)
                    .saturating_sub(PREFETCH_BEHIND + 1);
                let last_visible = (first_visible
                    + (vp_h / (est_page_h + self.page_margin)).ceil() as usize
                    + PREFETCH_AHEAD
                    + PREFETCH_BEHIND
                    + 2)
                .min(page_count.saturating_sub(1));

                // Evicta texturas fora da janela estendida do viewport
                let cache_first = first_visible.saturating_sub(PREFETCH_BEHIND);
                let cache_last = last_visible;
                self.tex_cache
                    .retain(|&idx, _| idx >= cache_first && idx <= cache_last);
                self.page_cache.evict_range(cache_first, cache_last);

                // Solicita pré-render das páginas vizinhas
                for idx in first_visible..=last_visible {
                    if idx < page_count {
                        self.page_cache.prefetch(idx, scale);
                    }
                }

                for idx in 0..page_count {
                    let is_visible = idx >= first_visible && idx <= last_visible;

                    // Fallback síncrono se visível mas ainda não chegou do background
                    if is_visible && !self.tex_cache.contains_key(&idx) && self.pdf_path.is_none() {
                        if let Ok(mut rendered) = engine.render_page(idx, scale) {
                            if self.invert_colors {
                                invert_rgba(&mut rendered.rgba);
                            }
                            let tex = upload_texture(ui.ctx(), &rendered, idx);
                            self.tex_cache.insert(idx, tex);
                        }
                    }

                    ui.vertical_centered(|ui| {
                        if let Some(tex) = self.tex_cache.get(&idx) {
                            let (w, h) =
                                fit_page(tex.size_vec2(), available_w - self.page_margin * 2.0);
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
                            draw_skeleton(
                                ui,
                                est_page_h,
                                available_w,
                                self.page_margin,
                                idx,
                                is_visible,
                            );
                        }
                    });

                    if idx + 1 < page_count {
                        ui.add_space(self.page_margin);
                    }
                }
                ui.add_space(self.page_margin);
            });
    }
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
