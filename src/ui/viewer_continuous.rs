// ui/viewer_continuous.rs — Modo de rolagem contínua do viewer PDF
// Extraído de viewer.rs para respeitar o hardcap de 250L.
// Responsabilidade: show_continuous() com prefetch, skeleton e virtual scrolling.

use crate::pdf::document_worker::JobPriority;
use crate::ui::page_layout::LayoutParams;
use crate::ui::prefetch::PrefetchPlan;
use crate::ui::scroll_motion::{instant_wheel_offset, NavGoal, ScrollTween};
use crate::ui::viewer::{page_raster_scale, PdfViewer};
use crate::ui::viewer_helpers::{
    draw_page_tex, page_ribbon, paint_search_highlight, upload_texture,
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

        // Ctrl+Scroll → zoom (só com o mouse sobre o leitor)
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let over_reader = ui.rect_contains_pointer(ui.max_rect());
        let scroll_y = ui.input(|i| {
            if self.smooth_scroll {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y
            }
        });
        if ctrl && over_reader && scroll_y != 0.0 {
            self.zoom_delta = scroll_y.signum() * 0.15;
        }

        let available_w = ui.available_width();
        self.usable_width = (available_w - self.page_margin * 2.0).max(0.0);
        let (zoom, usable_width, density) = (self.zoom, self.usable_width, self.render_scale);
        let ctx = ui.ctx().clone();

        // Drena bitmaps prontos do scheduler global
        if self.page_cache.drain_ready() {
            ctx.request_repaint();
        }

        self.page_layout.ensure_page_count(page_count);
        let layout_changed = self.page_layout.rebuild(LayoutParams {
            available_width: available_w,
            zoom: self.zoom,
            margin: self.page_margin,
        });
        let forced_offset = self.continuous_forced_offset(ui, ctrl, layout_changed);
        let total_h = self.page_layout.total_height();
        self.last_page_rect = egui::Rect::NOTHING;
        self.is_loading = false;

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("continuous_scroll")
            .auto_shrink(false);
        if let Some(offset) = forced_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        let scroll_output = scroll_area.show_viewport(ui, |ui, viewport| {
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
                if !self.page_errors.contains_key(&page) {
                    let scale =
                        page_raster_scale(&self.page_layout, page, zoom, usable_width, density);
                    // Bitmap despejado do cache de CPU mas com textura nítida
                    // na GPU: re-renderizar só gastaria PDFium em loop.
                    if !self.tex_cache.is_sharp_enough(page, scale) {
                        self.page_cache.prefetch(page, scale, priority);
                    }
                }
            });

            for idx in first_visible..=last_visible {
                // Upload para GPU quando a página chega ao viewport ou quando um
                // bitmap mais nítido substitui o atual (janela ampliada).
                let uploaded_width = self.tex_cache.get(idx).map(|tex| tex.size()[0] as u32);
                let cached_width = self.page_cache.get(idx).map(|bitmap| bitmap.width);
                let needs_upload = match (uploaded_width, cached_width) {
                    (None, _) => true,
                    (Some(uploaded), Some(cached)) => cached > uploaded + 1,
                    (Some(_), None) => false,
                };
                if needs_upload {
                    if let Some(bitmap) = self.page_cache.get(idx) {
                        let tex = if self.page_look.filters_pixels() {
                            let mut filtered = (*bitmap).clone();
                            crate::reader_profile::apply_filter(&mut filtered.rgba, self.page_look);
                            upload_texture(&ctx, &filtered, idx)
                        } else {
                            upload_texture(&ctx, bitmap, idx)
                        };
                        let learned_size = self.page_layout.observe_bitmap(
                            idx,
                            bitmap.width,
                            bitmap.height,
                            bitmap.scale,
                        );
                        let (width, height, scale) = (bitmap.width, bitmap.height, bitmap.scale);
                        self.tex_cache.insert(idx, tex, width, height, scale);
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
                        tracing::warn!("continuous: página {} falhou: {error}", idx + 1);
                        self.page_errors.insert(idx, error);
                        self.continuous_failures += 1;
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
                            // O layout já decidiu a altura pelo zoom; a
                            // largura segue a proporção da página.
                            let aspect = tex.size_vec2().x / tex.size_vec2().y.max(1.0);
                            let (w, h) = (page_height * aspect, page_height);
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
                                self.rotation,
                            );
                            let marked = self.marks.binary_search(&idx).is_ok();
                            if page_ribbon(ui, page_rect, idx, marked, self.mark_color) {
                                self.mark_toggle = Some(idx);
                            }
                            if self.last_page_rect == egui::Rect::NOTHING {
                                self.last_page_rect = page_rect;
                            }
                        } else if let Some(error) = self.page_errors.get(&idx) {
                            draw_page_error(
                                ui,
                                page_height,
                                available_w,
                                self.page_margin,
                                idx,
                                error,
                            );
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
        self.paint_mark_rail(ui, scroll_output.inner_rect);
    }
}

/// Placeholder de uma página que não renderizou; o resto do documento segue
/// navegável.
fn draw_page_error(
    ui: &mut egui::Ui,
    height: f32,
    available_w: f32,
    page_margin: f32,
    idx: usize,
    error: &str,
) {
    let width = (available_w - page_margin * 2.0).min(800.0);
    let (_, rect) = ui.allocate_space(egui::vec2(width, height));
    let danger = egui::Color32::from_rgb(220, 90, 80);
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        egui::Color32::from_rgba_unmultiplied(220, 90, 80, 18),
        egui::Stroke::new(1.0_f32, danger.gamma_multiply(0.6)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!(
            "⚠ {}\n{error}",
            crate::t!("page-render-failed", page => idx + 1)
        ),
        egui::FontId::proportional(13.0),
        danger,
    );
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

impl PdfViewer {
    /// Decide o offset imposto ao ScrollArea neste frame: navegação
    /// programática (animada ou não), preservação de âncora após mudança de
    /// layout e, com Scroll Suave desligado, o passo instantâneo da roda.
    fn continuous_forced_offset(
        &mut self,
        ui: &egui::Ui,
        ctrl: bool,
        layout_changed: bool,
    ) -> Option<f32> {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let current = self.continuous_scroll_offset;

        let goal = match self.pending_scroll_anchor.take() {
            Some(anchor) => Some(NavGoal::Anchor(anchor)),
            None => self.pending_scroll_page.take().map(NavGoal::Page),
        };

        let mut forced = None;
        if let Some(goal) = goal {
            let target = self.goal_offset(goal);
            tracing::debug!(
                target,
                from = ?current,
                smooth = self.smooth_scroll,
                "continuous: navegação programática"
            );
            match current {
                Some(from) if self.smooth_scroll && (target - from).abs() > 1.0 => {
                    self.scroll_tween = Some(ScrollTween::new(from, target, now));
                    self.scroll_goal = Some(goal);
                }
                _ => {
                    self.scroll_tween = None;
                    self.scroll_goal = None;
                    forced = Some(target);
                }
            }
        } else if layout_changed {
            match self.scroll_goal {
                // Durante o salto, páginas que terminam de renderizar mudam o
                // layout: o destino acompanha em vez de a animação morrer no
                // meio do caminho.
                Some(goal) if self.scroll_tween.is_some() => {
                    let target = self.goal_offset(goal);
                    if let Some(tween) = self.scroll_tween.as_mut() {
                        tween.retarget(target);
                    }
                }
                // Zoom/redimensionamento fora de salto: a âncora vale na hora.
                _ => {
                    self.scroll_tween = None;
                    self.scroll_goal = None;
                    forced = self.continuous_anchor.map(|anchor| {
                        self.page_layout
                            .scroll_offset_for_anchor(anchor, self.continuous_viewport_height)
                    });
                }
            }
        }

        let hovered = ui.rect_contains_pointer(ui.max_rect());
        let raw_y = ctx.input(|i| i.raw_scroll_delta.y);
        let wheel = hovered && !ctrl && raw_y != 0.0;
        if wheel {
            // A roda do usuário interrompe um salto animado.
            self.scroll_tween = None;
            self.scroll_goal = None;
        }
        if let Some(tween) = self.scroll_tween {
            let (offset, done) = tween.sample(now);
            forced = Some(offset);
            if done {
                self.scroll_tween = None;
                self.scroll_goal = None;
            } else {
                ctx.request_repaint();
            }
        }

        if !self.smooth_scroll && hovered && !ctrl {
            if wheel && forced.is_none() {
                forced = Some(instant_wheel_offset(current.unwrap_or(0.0), raw_y));
            }
            // O ScrollArea não deve reaplicar a cauda suavizada do egui.
            ctx.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
        }
        forced
    }

    fn goal_offset(&self, goal: NavGoal) -> f32 {
        match goal {
            NavGoal::Page(page) => self.page_layout.scroll_offset_for_page(page),
            NavGoal::Anchor(anchor) => self
                .page_layout
                .scroll_offset_for_anchor(anchor, self.continuous_viewport_height),
        }
    }

    /// Trilho de marcas: um risco por página marcada na lateral do leitor,
    /// na altura proporcional da página no documento. Clicar leva até ela.
    fn paint_mark_rail(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let total = self.page_layout.total_height();
        if self.marks.is_empty() || total <= 0.0 || !rect.is_positive() {
            return;
        }
        let x = rect.right() - 14.0;
        let marks = self.marks.clone();
        for page in marks {
            let fraction = (self.page_layout.page_top(page) / total).clamp(0.0, 1.0);
            let y = (rect.top() + fraction * rect.height())
                .clamp(rect.top() + 4.0, rect.bottom() - 4.0);
            let tick = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(10.0, 4.0));
            let response = ui
                .interact(
                    tick.expand(4.0),
                    egui::Id::new(("mark_rail", page)),
                    egui::Sense::click(),
                )
                .on_hover_text(crate::t!("side-mark-page", page => page + 1));
            let color = if response.hovered() {
                self.mark_color
            } else {
                self.mark_color.gamma_multiply(0.8)
            };
            ui.painter()
                .rect_filled(tick, egui::CornerRadius::same(2), color);
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                self.mark_jump = Some(page);
            }
        }
    }
}
