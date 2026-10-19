// app_overlays.rs — Overlays flutuantes: GoTo Page e Search Bar
// Extraído de app.rs (L1971-2138). Funções chamadas diretamente em app::MariApp.

use crate::app::{MariApp, ACCENT_PRIMARY, BG_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SUBTLE};
use crate::pdf::text_search::SearchHit;
use crate::t;
use crate::ui::icons;
use crate::ui::viewer::{SearchHighlight, ViewMode};
use eframe::egui;

impl MariApp {
    // ── Overlay Ir para Página (Ctrl+G) ──────────────────────────────────────

    pub(crate) fn ui_goto_overlay(&mut self, ctx: &egui::Context) {
        let pc = self.page_count;

        egui::Area::new(egui::Id::new("goto_overlay"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG_SURFACE)
                    .stroke(egui::Stroke::new(1.5_f32, ACCENT_PRIMARY))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 120),
                        spread: 8,
                        blur: 12,
                        offset: [0, 4],
                    })
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(t!("overlay-goto-label"))
                                    .size(13.0)
                                    .color(TEXT_PRIMARY),
                            );
                            ui.add_space(8.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.goto_input)
                                    .desired_width(80.0)
                                    .hint_text(format!("1–{pc}"))
                                    .char_limit(6),
                            );
                            resp.request_focus();

                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));

                            if enter {
                                if let Ok(n) = self.goto_input.trim().parse::<usize>() {
                                    let page = n.saturating_sub(1).min(pc.saturating_sub(1));
                                    let c = ctx.clone();
                                    self.go_to_page(&c, page);
                                }
                                self.goto_open = false;
                            }
                            if esc {
                                self.goto_open = false;
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(t!("overlay-goto-hint"))
                                .size(10.0)
                                .color(TEXT_SUBTLE),
                        );
                    });
            });
    }

    // ── Search Bar (Ctrl+F) ───────────────────────────────────────────────────

    pub(crate) fn ui_search_bar(&mut self, ctx: &egui::Context) {
        let first_hits_arrived = self.search.poll();
        if self.search.is_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        if first_hits_arrived {
            self.reveal_current_hit(ctx);
        }

        let total_hits = self.search.hits.len();
        let current_idx = self.search.current_hit;
        let is_running = self.search.is_running;
        let query_snap = self.search.query.clone();
        let current_hit = self.search.current().cloned();

        let mut new_query = query_snap.clone();
        let mut do_search = false;
        let mut go_next = false;
        let mut go_prev = false;
        let mut do_close = false;

        egui::Area::new(egui::Id::new("search_bar"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG_SURFACE)
                    .stroke(egui::Stroke::new(1.5_f32, ACCENT_PRIMARY))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 140),
                        spread: 8,
                        blur: 14,
                        offset: [0, 4],
                    })
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔍").size(13.0));
                            ui.add_space(4.0);

                            let edit_resp = ui.add(
                                egui::TextEdit::singleline(&mut new_query)
                                    .desired_width(180.0)
                                    .hint_text(t!("overlay-search-placeholder"))
                                    .font(egui::TextStyle::Body),
                            );
                            if self.search_focused {
                                edit_resp.request_focus();
                                self.search_focused = false;
                            }

                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let shift = ui.input(|i| i.modifiers.shift);
                            if enter && !shift {
                                go_next = true;
                                do_search = new_query != query_snap || total_hits == 0;
                            }
                            if enter && shift {
                                go_prev = true;
                            }

                            if total_hits > 0 {
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}/{}",
                                        current_idx + 1,
                                        total_hits
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                            } else if is_running {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("…").size(11.0).color(TEXT_MUTED));
                            } else if !query_snap.is_empty() {
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(t!("overlay-search-no-results"))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(200, 80, 80)),
                                );
                            }

                            ui.add_space(8.0);
                            if ui
                                .small_button(icons::ph(icons::CARET_LEFT, 13.0, TEXT_PRIMARY))
                                .clicked()
                            {
                                go_prev = true;
                            }
                            if ui
                                .small_button(icons::ph(icons::CARET_RIGHT, 13.0, TEXT_PRIMARY))
                                .clicked()
                            {
                                go_next = true;
                            }
                            ui.add_space(4.0);
                            if ui
                                .small_button(icons::ph(icons::X, 13.0, TEXT_PRIMARY))
                                .clicked()
                            {
                                do_close = true;
                            }
                        });
                        if let Some(hit) = &current_hit {
                            ui.add_space(4.0);
                            ui.label(context_snippet(hit));
                        }
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(t!("overlay-search-hint"))
                                .size(9.0)
                                .color(TEXT_SUBTLE),
                        );
                    });
            });

        if new_query != query_snap {
            self.search.query = new_query.clone();
        }

        if do_search || (go_next && self.search.hits.is_empty() && !new_query.is_empty()) {
            self.search.start(&new_query);
        }

        if go_next && !self.search.hits.is_empty() {
            self.search.go_next();
        }
        if go_prev && !self.search.hits.is_empty() {
            self.search.go_prev();
        }

        if (go_next || go_prev) && !self.search.hits.is_empty() {
            self.reveal_current_hit(ctx);
        }

        if do_close {
            self.search_open = false;
            self.search.clear();
        }
    }
}

impl MariApp {
    /// Ocorrência que o viewer deve destacar neste frame.
    pub(crate) fn active_search_highlight(&self) -> Option<SearchHighlight> {
        if !self.search_open {
            return None;
        }
        let hit = self.search.current()?;
        Some(SearchHighlight {
            page_index: hit.page_index,
            page_size: hit.page_size,
            rects: hit.rects.clone(),
        })
    }

    /// Navega até a ocorrência atual; no modo contínuo centraliza a própria linha.
    fn reveal_current_hit(&mut self, ctx: &egui::Context) {
        let Some(hit) = self.search.current() else {
            return;
        };
        let page = hit.page_index;
        let fraction = hit_page_fraction(hit);
        self.go_to_page(ctx, page);
        if self.viewer.mode == ViewMode::Continuous {
            if let Some(fraction) = fraction {
                self.viewer.reveal_continuous(page, fraction);
            }
        }
    }
}

/// Posição vertical (0 = topo) do primeiro retângulo da ocorrência na página.
fn hit_page_fraction(hit: &SearchHit) -> Option<f32> {
    let [_, bottom, _, top] = *hit.rects.first()?;
    let (_, height) = hit.page_size;
    (height > 0.0).then(|| 1.0 - (top + bottom) * 0.5 / height)
}

/// Trecho de contexto com a ocorrência em destaque.
fn context_snippet(hit: &SearchHit) -> egui::text::LayoutJob {
    let font = egui::FontId::proportional(11.0);
    let plain = egui::TextFormat {
        font_id: font.clone(),
        color: TEXT_MUTED,
        ..Default::default()
    };
    let strong = egui::TextFormat {
        font_id: font,
        color: TEXT_PRIMARY,
        background: ACCENT_PRIMARY.gamma_multiply(0.45),
        ..Default::default()
    };
    let text = &hit.context;
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = 320.0;
    job.wrap.max_rows = 2;
    job.append(&text[..hit.match_start], 0.0, plain.clone());
    job.append(&text[hit.match_start..hit.match_end], 0.0, strong);
    job.append(&text[hit.match_end..], 0.0, plain);
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_fraction_points_at_the_line_center() {
        let hit = SearchHit {
            page_index: 0,
            context: String::new(),
            match_start: 0,
            match_end: 0,
            rects: vec![[0.0, 590.0, 50.0, 610.0]],
            page_size: (600.0, 800.0),
        };
        assert_eq!(hit_page_fraction(&hit), Some(0.25));
    }
}
