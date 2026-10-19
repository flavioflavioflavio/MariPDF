// app_home_btns.rs — Botões flutuantes da home (tema e configurações)
// Extraído de app_home.rs para manter abaixo de 250 linhas.

use crate::app::MariApp;
use crate::app_color_mode::ColorMode;
use crate::app_constants::ACCENT_LIGHT;
use crate::ui::icons;
use eframe::egui;

impl MariApp {
    pub(crate) fn ui_home_buttons(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        let area_pos = egui::pos2(screen_rect.right() - 52.0, screen_rect.top() + 12.0);
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("home_area_pos"), area_pos));

        let btn_surf = self.color_mode.surface();
        let btn_bord = match self.color_mode {
            ColorMode::Dark => egui::Color32::from_rgb(55, 55, 65),
            ColorMode::Light => egui::Color32::from_rgb(200, 195, 185),
            ColorMode::Sepia => egui::Color32::from_rgb(170, 140, 100),
        };
        let icon_color = match self.color_mode {
            ColorMode::Dark => ACCENT_LIGHT,
            ColorMode::Light => egui::Color32::from_rgb(80, 60, 20),
            ColorMode::Sepia => egui::Color32::from_rgb(90, 55, 10),
        };
        let icon_text = match self.color_mode {
            ColorMode::Dark => icons::ph(icons::MOON, 20.0, icon_color),
            ColorMode::Light => icons::ph(icons::SUN, 20.0, icon_color),
            ColorMode::Sepia => icons::ph(icons::LAMP_PENDANT, 20.0, icon_color),
        };

        // ── Botão tema ────────────────────────────────────────────────────────
        egui::Area::new(egui::Id::new("home_theme_btn"))
            .fixed_pos(area_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(btn_surf)
                    .stroke(egui::Stroke::new(1.4, btn_bord))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        let resp = ui
                            .add(
                                egui::Button::new(icon_text)
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(28.0, 28.0)),
                            )
                            .on_hover_text(self.color_mode.label());
                        if resp.clicked() {
                            self.color_mode = self.color_mode.next();
                            self.viewer.tex_cache.clear();
                            self.viewer.texture = None;
                            self.viewer.loaded_page = None;
                            self.pending_rerender = true;
                        }
                        if resp.hovered() {
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                    });
            });

        // ── Botão settings ────────────────────────────────────────────────────
        if let Some(ap) = ctx.data(|d| d.get_temp::<egui::Pos2>(egui::Id::new("home_area_pos"))) {
            let cm_snap = self.color_mode;
            egui::Area::new(egui::Id::new("home_settings_btn"))
                .fixed_pos(egui::pos2(ap.x - 44.0, ap.y))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let hid = egui::Id::new("home_settings_hover");
                    let hov = ui.ctx().data(|d| d.get_temp::<bool>(hid).unwrap_or(false));
                    egui::Frame::new()
                        .fill(if hov {
                            cm_snap.surface()
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(egui::Stroke::new(
                            1.1,
                            if hov {
                                cm_snap.border()
                            } else {
                                egui::Color32::TRANSPARENT
                            },
                        ))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            let r = ui
                                .add(
                                    egui::Button::new(icons::ph(
                                        icons::GEAR,
                                        20.0,
                                        cm_snap.text_muted(),
                                    ))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(28.0, 28.0)),
                                )
                                .on_hover_text(crate::t!("toolbar-settings-hint"));
                            ui.ctx().data_mut(|d| d.insert_temp(hid, r.hovered()));
                            if r.hovered() {
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if r.clicked() {
                                self.settings_open = !self.settings_open;
                            }
                        });
                });
        }
    }
}
