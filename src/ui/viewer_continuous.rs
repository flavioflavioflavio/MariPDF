// ui/viewer_continuous.rs — Modo de rolagem contínua do viewer PDF
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: show_continuous() com prefetch, skeleton e virtual scrolling.

use crate::pdf::document_worker::JobPriority;
use crate::ui::page_layout::LayoutParams;
use crate::ui::prefetch::PrefetchPlan;
use crate::ui::viewer::PdfViewer;
use crate::ui::viewer_helpers::{
    draw_page_tex, fit_page, invert_rgba, paint_search_highlight, upload_texture,
};
use eframe::egui;

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

        // Drena bitmaps prontos do scheduler global
        if self.page_cache.drain_ready() {
            ctx.request_repaint();
        }

        self.page_layout.ensure_page_count(page_count);
        let layout_changed = self.page_layout.rebuild(LayoutParams {
            available_width: available_w,
            zoom: self.zoom,
            render_scale: self.render_scale,
            margin: self.page_margin,
        });
        let forced_offset = if let Some(anchor) = self.pending_scroll_anchor.take() {
            Some(
                self.page_layout
                    .scroll_offset_for_anchor(anchor, self.continuous_viewport_height),
            )
        } else if let Some(page) = self.pending_scroll_page.take() {
            Some(self.page_layout.scroll_offset_for_page(page))
        } else if layout_changed {
            self.continuous_anchor.map(|anchor| {
                self.page_layout
                    .scroll_offset_for_anchor(anchor, self.continuous_viewport_height)
            })
        } else {
            None
        };
        let total_h = self.page_layout.total_height();
        self.last_page_rect = egui::Rect::NOTHING;
        self.is_loading = false;

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("continuous_scroll")
            .auto_shrink(false);
        if let Some(offset) = forced_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        scroll_area.show_viewport(ui, |ui, viewport| {
            ui.set_min_height(total_h);

            let (first_visible, last_visible) = self
                .page_layout
                .visible_range(viewport.min.y, viewport.max.y);
            self.visible_page = Some(
                self.page_layout
                    .dominant_page(viewport.min.y, viewport.max.y),
            );
            self.continuous_anchor = Some(self.page_layout.anchor_at(viewport.center().y));
            self.continuous_viewport_height = viewport.height();
            let scroll_delta = self
                .continuous_scroll_offset
                .map_or(0.0, |previous| viewport.min.y - previous);
            self.continuous_scroll_offset = Some(viewport.min.y);
            let prefetch = PrefetchPlan::new(
                first_visible,
                last_visible,
                page_count,
                scroll_delta,
                viewport.height(),
            );

            // Evicta texturas fora da janela estendida do viewport
            self.tex_cache.retain_range(prefetch.first, prefetch.last);
            self.page_cache.evict_range(prefetch.first, prefetch.last);

            prefetch.schedule(|page| {
                let priority = if (first_visible..=last_visible).contains(&page) {
                    JobPriority::Immediate
                } else {
                    JobPriority::Visible
                };
                self.page_cache.prefetch(page, scale, priority);
            });

            for idx in first_visible..=last_visible {
                // Upload para GPU só quando a página chega ao viewport.
                if !self.tex_cache.contains(idx) {
                    if let Some(bitmap) = self.page_cache.get(idx) {
                        let tex = if self.invert_colors {
                            let mut inverted = (*bitmap).clone();
                            invert_rgba(&mut inverted.rgba);
                            upload_texture(&ctx, &inverted, idx)
                        } else {
                            upload_texture(&ctx, bitmap, idx)
                        };
                        let learned_size = self.page_layout.observe_bitmap(
                            idx,
                            bitmap.width,
                            bitmap.height,
                            scale,
                        );
                        self.tex_cache.insert(idx, tex, bitmap.width, bitmap.height);
                        if learned_size {
                            ctx.request_repaint();
                        }
                        let cpu = self.page_cache.metrics();
                        let gpu = self.tex_cache.metrics();
                        tracing::debug!(
                            cpu_bytes = cpu.resident_bytes,
                            cpu_budget = cpu.budget_bytes,
                            cpu_hits = cpu.hits,
                            cpu_misses = cpu.misses,
                            cpu_evictions = cpu.evictions,
                            gpu_bytes = gpu.resident_bytes,
                            gpu_budget = gpu.budget_bytes,
                            gpu_hits = gpu.hits,
                            gpu_misses = gpu.misses,
                            gpu_evictions = gpu.evictions,
                            "viewer cache metrics"
                        );
                    } else if let Some(error) = self.page_cache.take_failure(idx) {
                        self.last_error = Some(error);
                    }
                }

                let page_height = self.page_layout.page_height(idx);
                let y_top = ui.max_rect().top() + self.page_layout.page_top(idx);
                let slot_rect = egui::Rect::from_min_size(
                    egui::pos2(ui.max_rect().left(), y_top),
                    egui::vec2(available_w, page_height),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(slot_rect), |ui| {
                    ui.vertical_centered(|ui| {
                        if let Some(tex) = self.tex_cache.get(idx) {
                            let (mut w, mut h) =
                                fit_page(tex.size_vec2(), available_w - self.page_margin * 2.0);
                            if h > page_height {
                                let ratio = page_height / h;
                                w *= ratio;
                                h = page_height;
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
                            paint_search_highlight(
                                ui,
                                page_rect,
                                self.search_highlight.as_ref(),
                                Some(idx),
                            );
                            if self.last_page_rect == egui::Rect::NOTHING {
                                self.last_page_rect = page_rect;
                            }
                        } else {
                            self.is_loading = true;
                            draw_skeleton(
                                ui,
                                page_height,
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
