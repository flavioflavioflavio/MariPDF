// app_overlays.rs — Overlays flutuantes: GoTo Page e Search Bar
// Extraído de app.rs (L1971-2138). Funções chamadas diretamente em app::MariApp.

use crate::app::{MariApp, ACCENT_PRIMARY, BG_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SUBTLE};
use crate::pdf::text_search::search_all_pages;
use crate::t;
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
                    .stroke(egui::Stroke::new(1.5, ACCENT_PRIMARY))
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
        let total_hits = self.search.hits.len();
        let current_idx = self.search.current_hit;
        let is_running = self.search.is_running;
        let query_snap = self.search.query.clone();

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
                    .stroke(egui::Stroke::new(1.5, ACCENT_PRIMARY))
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
                                    .hint_text("Buscar no PDF…")
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
                            if ui.small_button("◀").clicked() {
                                go_prev = true;
                            }
                            if ui.small_button("▶").clicked() {
                                go_next = true;
                            }
                            ui.add_space(4.0);
                            if ui.small_button("✕").clicked() {
                                do_close = true;
                            }
                        });
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
            if let Some(engine) = &self.engine {
                self.search.hits = search_all_pages(engine, &new_query);
                self.search.current_hit = 0;
                self.search.is_running = false;
            }
        }

        if go_next && !self.search.hits.is_empty() {
            self.search.go_next();
        }
        if go_prev && !self.search.hits.is_empty() {
            self.search.go_prev();
        }

        if (go_next || go_prev) && !self.search.hits.is_empty() {
            if let Some(hit) = self.search.current() {
                let pg = hit.page_index;
                let c = ctx.clone();
                self.go_to_page(&c, pg);
            }
        }

        if do_close {
            self.search_open = false;
            self.search.clear();
        }
    }
}
