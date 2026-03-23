// settings_panel.rs — Painel de configurações do MariPDF
// Layout: tabs (Aparência / Leitura / Privacidade)
// Todas as cores via tokens do ColorMode — zero hardcoded

use eframe::egui;
use crate::settings::{
    AppSettings, UiScale, FontChoice, RenderQuality, PageMargin, DefaultPanel, ColorPalette,
};
use crate::app::ColorMode;
use crate::ui::icons;
use crate::ui::settings_widgets::{Tokens, sec, divider, chip, action_btn, toggle_row};
use crate::ui::settings_tabs::{tab_appearance, tab_reading, tab_privacy};




// ── Tabs ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab { Appearance, Reading, Privacy }
impl Default for Tab { fn default() -> Self { Tab::Appearance } }

// ── Ponto de entrada público ──────────────────────────────────────────────────

pub fn show_settings_panel(
    ctx:                 &egui::Context,
    open:                &mut bool,
    settings:            &mut AppSettings,
    color_mode:          &mut ColorMode,
    needs_font_reload:   &mut bool,
    needs_render_reload: &mut bool,
) {
    if !*open { return; }

    let cm  = *color_mode;
    let tok = Tokens::from(cm);

    let outer_frame = egui::Frame::new()
        .fill(tok.surf)
        .stroke(egui::Stroke::new(tok.dw * 1.5, tok.bord))
        .corner_radius(egui::CornerRadius::same(16))
        .inner_margin(egui::Margin::same(0));

    let mut close_req = false;
    let tab_id = egui::Id::new("settings_tab");
    let mut active_tab: Tab = ctx.data(|d| d.get_temp(tab_id).unwrap_or_default());

    egui::Window::new("settings_wnd")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .default_pos(ctx.screen_rect().center() - egui::vec2(250.0, 220.0))
        .fixed_size(egui::vec2(500.0, 440.0))
        .frame(outer_frame)
        .show(ctx, |ui| {

            // ── Header ───────────────────────────────────────────────────────
            egui::Frame::new()
                .fill(tok.stat)
                .corner_radius(egui::CornerRadius { nw: 16, ne: 16, sw: 0, se: 0 })
                .inner_margin(egui::Margin { left: 20, right: 16, top: 16, bottom: 16 })
                .show(ui, |ui| {
                    ui.set_min_width(460.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(icons::GEAR)
                                .size(18.0).color(tok.accent)
                                .family(egui::FontFamily::Name("Icons".into())),
                        );
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Configurações").size(15.0).strong().color(tok.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(icons::X_CIRCLE).size(18.0).color(tok.muted)
                                        .family(egui::FontFamily::Name("Icons".into()))
                                ).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE)
                                .min_size(egui::vec2(30.0, 30.0))
                            ).on_hover_cursor(egui::CursorIcon::PointingHand)
                             .on_hover_ui(|ui| { ui.label("Fechar configurações"); });
                            if resp.clicked() { close_req = true; }
                        });
                    });
                });

            // ── Tabs ─────────────────────────────────────────────────────────
            egui::Frame::new()
                .fill(tok.stat)
                .inner_margin(egui::Margin { left: 16, right: 16, top: 2, bottom: 0 })
                .show(ui, |ui| {
                    ui.set_min_width(468.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (label, icon, tab) in [
                            ("Aparência",   icons::PAINT_BRUSH, Tab::Appearance),
                            ("Leitura",     icons::BOOK,        Tab::Reading),
                            ("Privacidade", icons::SHIELD,      Tab::Privacy),
                        ] {
                            let is_active = active_tab == tab;
                            let tab_bg   = if is_active { tok.surf } else { egui::Color32::TRANSPARENT };
                            let fg_text  = if is_active { tok.text  } else { tok.subtle };
                            let fg_icon  = if is_active { tok.accent } else { tok.subtle };
                            let r = egui::Frame::new()
                                .fill(tab_bg)
                                .corner_radius(egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 })
                                .inner_margin(egui::Margin { left: 18, right: 18, top: 12, bottom: 12 })
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(icon).size(13.0).color(fg_icon)
                                                .family(egui::FontFamily::Name("Icons".into()))
                                        );
                                        ui.add_space(5.0);
                                        ui.label(egui::RichText::new(label).size(13.0).color(fg_text));
                                    });
                                }).response;
                            if r.interact(egui::Sense::click()).clicked() { active_tab = tab; }
                            if r.interact(egui::Sense::hover()).hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    });
                });

            ui.add(egui::Separator::default().horizontal().spacing(1.0));

            // ── Content ──────────────────────────────────────────────────────
            egui::Frame::new()
                .inner_margin(egui::Margin { left: 24, right: 24, top: 20, bottom: 20 })
                .show(ui, |ui| {
                    ui.set_min_width(452.0);
                    ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);
                    ui.spacing_mut().item_spacing   = egui::vec2(8.0, 10.0);

                    match active_tab {
                        Tab::Appearance => tab_appearance(ui, settings, color_mode, cm, needs_font_reload, needs_render_reload, &tok),
                        Tab::Reading    => tab_reading(ui, settings, needs_render_reload, &tok),
                        Tab::Privacy    => tab_privacy(ui, settings, &tok),
                    }
                });

            // ── Footer ───────────────────────────────────────────────────────
            egui::Frame::new()
                .fill(tok.stat)
                .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: 16, se: 16 })
                .inner_margin(egui::Margin { left: 24, right: 20, top: 14, bottom: 14 })
                .show(ui, |ui| {
                    ui.set_min_width(456.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("MariPDF — Leitor ultra-leve").size(10.5).color(tok.subtle));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let resp = action_btn(ui, "  Fechar  ", tok.accent, tok.on_acc, &tok);
                            if resp.clicked() { close_req = true; }
                        });
                    });
                });
        });

    ctx.data_mut(|d| d.insert_temp(tab_id, active_tab));
    if close_req { *open = false; }
}

