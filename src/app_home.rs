// app_home.rs — Tela inicial (Home) do MariPDF
// Extraído de app.rs (L917-1296).

use eframe::egui;
use crate::app::{MariApp, ACCENT_PRIMARY, ACCENT_LIGHT};
use crate::app_color_mode::ColorMode;
use crate::ui::icons;

impl MariApp {
    pub(crate) fn ui_home(&mut self, ctx: &egui::Context) {
        let cm    = self.color_mode;
        let pal   = self.settings.palette.0;
        let bg    = cm.bg_for(pal);
        let surf  = cm.surface_for(pal);
        let bord  = cm.border();
        let text  = cm.text_primary();
        let muted = cm.text_muted();
        let dw    = cm.divider_width();

        let recents = self.persistence.recent_files(8);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg))
            .show(ctx, |ui| {
                let total_w   = ui.available_width();
                let total_h   = ui.available_height();
                let content_w = (total_w * 0.80).min(640.0).max(360.0);
                let side_pad  = (total_w - content_w) / 2.0;

                let top_space = if recents.is_empty() { (total_h * 0.25).max(80.0) } else { (total_h * 0.14).max(40.0) };
                ui.add_space(top_space);

                // ── Logo ────────────────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("MariPDF")
                        .size(46.0).color(ACCENT_PRIMARY)
                        .family(egui::FontFamily::Name("Title".into())));
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Leitor PDF ultra-leve para Windows")
                        .size(13.0).color(muted));
                });

                // ── Botão CTA ────────────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    let btn_w     = (content_w * 0.55).min(280.0).max(200.0);
                    ui.set_max_width(btn_w);

                    let cta_id     = egui::Id::new("home_open_btn");
                    let cta_hov_id = cta_id.with("hov");
                    let was_h: bool = ctx.data(|d| d.get_temp(cta_hov_id).unwrap_or(false));
                    let t_cta = ctx.animate_bool_with_time(cta_id, was_h, 0.12);
                    let darken_cta = (t_cta * 24.0) as u8;
                    let cta_bg = egui::Color32::from_rgb(
                        ACCENT_PRIMARY.r().saturating_sub(darken_cta),
                        ACCENT_PRIMARY.g().saturating_sub(darken_cta),
                        ACCENT_PRIMARY.b().saturating_sub(darken_cta),
                    );

                    let cta_resp = ui.add_sized(
                        [btn_w, 44.0],
                        egui::Button::new(egui::RichText::new("Abrir PDF…")
                            .size(15.0).color(egui::Color32::WHITE).strong())
                            .fill(cta_bg)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(12)),
                    );
                    let is_h_cta = cta_resp.hovered();
                    ctx.data_mut(|d| d.insert_temp(cta_hov_id, is_h_cta));
                    if is_h_cta { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); ctx.request_repaint(); }
                    if cta_resp.clicked() { self.open_file_dialog(); }

                    ui.add_space(10.0);

                    let drag_color = if self.drop_zone_hovered { ACCENT_LIGHT } else { cm.text_subtle() };
                    let drag_text  = if self.drop_zone_hovered { "▼  Solte o PDF aqui" } else { "ou arraste um PDF para qualquer lugar" };
                    ui.label(egui::RichText::new(drag_text).size(11.5).color(drag_color));
                });

                // ── Erros ────────────────────────────────────────────────────
                if let Some(ref err) = self.render_error.clone() {
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(format!("⚠ {err}"))
                            .size(12.0).color(egui::Color32::from_rgb(239, 68, 68)));
                    });
                }

                // ── Recentes ─────────────────────────────────────────────────
                if !recents.is_empty() {
                    ui.add_space(32.0);
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        ui.label(egui::RichText::new("RECENTES").size(10.0).color(muted).strong());
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        ui.add(egui::Separator::default().horizontal().grow(-(side_pad * 2.0)).spacing(dw));
                    });
                    ui.add_space(8.0);

                    let mut open_path:   Option<std::path::PathBuf> = None;
                    let mut remove_path: Option<std::path::PathBuf> = None;

                    for (path, session) in &recents {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("PDF")
                            .trim_end_matches(".pdf").trim_end_matches(".PDF");
                        let dir = path.parent().and_then(|p| p.to_str()).unwrap_or("");
                        let dir_display = if dir.len() > 55 { format!("…{}", &dir[dir.len() - 52..]) } else { dir.to_string() };
                        let pg_label = format!("p.{}", session.last_page + 1);

                        let mut x_btn_rect = egui::Rect::NOTHING;
                        ui.horizontal(|ui| {
                            ui.add_space(side_pad);
                            let card = egui::Frame::new()
                                .fill(surf).stroke(egui::Stroke::new(dw, bord))
                                .corner_radius(egui::CornerRadius::same(10))
                                .inner_margin(egui::Margin::symmetric(14, 10));

                            let ir = card.show(ui, |ui| {
                                ui.set_min_width(content_w - 2.0);
                                ui.set_max_width(content_w - 2.0);
                                ui.horizontal(|ui| {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(ui.cursor().min + egui::vec2(0.0, 6.0), egui::vec2(3.0, 26.0)),
                                        egui::CornerRadius::same(2), egui::Color32::from_rgb(220, 80, 60),
                                    );
                                    ui.add_space(10.0);
                                    let pills_w = 70.0_f32;
                                    let text_w  = (content_w - 2.0 - 3.0 - 10.0 - pills_w - 16.0).max(60.0);
                                    ui.allocate_ui_with_layout(egui::vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                        ui.set_max_width(text_w);
                                        ui.label(egui::RichText::new(name).size(13.5).color(text).strong());
                                        ui.add_space(1.0);
                                        ui.label(egui::RichText::new(&dir_display).size(10.5).color(muted));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let x_id  = ui.id().with(("xbtn", path.as_path()));
                                        let x_hv  = x_id.with("h");
                                        let x_was: bool = ctx.data(|d| d.get_temp(x_hv).unwrap_or(false));
                                        let xt    = ctx.animate_bool_with_time(x_id, x_was, 0.10);
                                        let x_bg  = egui::Color32::from_rgba_unmultiplied(200, 60, 50, (xt * 55.0) as u8);
                                        let x_fg  = if xt > 0.1 { egui::Color32::from_rgb(255, 130, 110) } else { egui::Color32::from_rgb(200, 80, 60) };
                                        let x_btn = ui.add(
                                            egui::Button::new(egui::RichText::new("\u{E4F8}").size(18.0).color(x_fg).family(egui::FontFamily::Name("Icons".into())))
                                                .fill(x_bg).stroke(egui::Stroke::NONE)
                                                .corner_radius(egui::CornerRadius::same(6))
                                                .min_size(egui::vec2(34.0, 34.0)),
                                        );
                                        x_btn_rect = x_btn.rect;
                                        let x_is = x_btn.hovered();
                                        ctx.data_mut(|d| d.insert_temp(x_hv, x_is));
                                        if x_is { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); ctx.request_repaint(); }
                                        if x_btn.clicked() { remove_path = Some(path.clone()); }
                                        ui.add_space(6.0);
                                        ui.label(egui::RichText::new(&pg_label).size(10.5).color(muted).monospace());
                                    });
                                })
                            });

                            let card_rect = ir.response.rect;
                            let (hovered, clicked, click_pos) = ctx.input(|i| {
                                let pos     = i.pointer.hover_pos().unwrap_or_default();
                                let interact = i.pointer.interact_pos().unwrap_or_default();
                                (card_rect.contains(pos), i.pointer.any_released() && card_rect.contains(interact), interact)
                            });
                            if hovered && !x_btn_rect.contains(ctx.input(|i| i.pointer.hover_pos().unwrap_or_default())) {
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if clicked && !x_btn_rect.contains(click_pos) && remove_path.is_none() {
                                open_path = Some(path.clone());
                            }
                        });
                        ui.add_space(5.0);
                    }

                    if let Some(p) = open_path   { self.pending_open = Some(p); }
                    if let Some(p) = remove_path  { self.persistence.remove_recent(&p); }
                }

                // ── Rodapé ────────────────────────────────────────────────────
                let remaining = ui.available_height();
                ui.add_space(remaining.max(16.0) - 16.0);
                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    ui.label(egui::RichText::new(format!(
                        "v{}  ·  Ctrl+F: Buscar  ·  Ctrl+G: Ir à página  ·  W: Fit Width  ·  F11: Tela cheia",
                        self.version,
                    )).size(10.0).color(cm.text_subtle()));
                });
            });

        // ── Botão tema (canto superior direito) ──────────────────────────────
        {
            let screen_rect = ctx.screen_rect();
            let area_pos = egui::pos2(screen_rect.right() - 52.0, screen_rect.top() + 12.0);
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("home_area_pos"), area_pos));
            let btn_surf = self.color_mode.surface();
            let btn_bord = match self.color_mode {
                ColorMode::Dark  => egui::Color32::from_rgb(55, 55, 65),
                ColorMode::Light => egui::Color32::from_rgb(200, 195, 185),
                ColorMode::Sepia => egui::Color32::from_rgb(170, 140, 100),
            };
            let icon_color = match self.color_mode {
                ColorMode::Dark  => ACCENT_LIGHT,
                ColorMode::Light => egui::Color32::from_rgb(80, 60, 20),
                ColorMode::Sepia => egui::Color32::from_rgb(90, 55, 10),
            };
            let icon_text = match self.color_mode {
                ColorMode::Dark  => icons::ph(icons::MOON,          20.0, icon_color),
                ColorMode::Light => icons::ph(icons::SUN,           20.0, icon_color),
                ColorMode::Sepia => icons::ph(icons::LAMP_PENDANT,  20.0, icon_color),
            };

            egui::Area::new(egui::Id::new("home_theme_btn"))
                .fixed_pos(area_pos).order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(btn_surf)
                        .stroke(egui::Stroke::new(1.4, btn_bord))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            let resp = ui.add(
                                egui::Button::new(icon_text)
                                    .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(28.0, 28.0)),
                            ).on_hover_text(self.color_mode.label());
                            if resp.clicked() {
                                self.color_mode = self.color_mode.next();
                                self.viewer.tex_cache.clear();
                                self.viewer.texture    = None;
                                self.viewer.loaded_page = None;
                                self.pending_rerender  = true;
                            }
                            if resp.hovered() { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                        });
                });
        }

        // ── Botão Settings ────────────────────────────────────────────────────
        if let Some(ap) = ctx.data(|d| d.get_temp::<egui::Pos2>(egui::Id::new("home_area_pos"))) {
            let cm_snap = self.color_mode;
            egui::Area::new(egui::Id::new("home_settings_btn"))
                .fixed_pos(egui::pos2(ap.x - 44.0, ap.y)).order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let hid = egui::Id::new("home_settings_hover");
                    let hov = ui.ctx().data(|d| d.get_temp::<bool>(hid).unwrap_or(false));
                    egui::Frame::new()
                        .fill(if hov { cm_snap.surface() } else { egui::Color32::TRANSPARENT })
                        .stroke(egui::Stroke::new(1.1, if hov { cm_snap.border() } else { egui::Color32::TRANSPARENT }))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            let r = ui.add(
                                egui::Button::new(icons::ph(icons::GEAR, 20.0, cm_snap.text_muted()))
                                    .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(28.0, 28.0)),
                            ).on_hover_text("Configurações");
                            ui.ctx().data_mut(|d| d.insert_temp(hid, r.hovered()));
                            if r.hovered() { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                            if r.clicked() { self.settings_open = !self.settings_open; }
                        });
                });
        }
    }
}
