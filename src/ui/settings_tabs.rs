// ui/settings_tabs.rs — Conteúdo das abas do painel de configurações
// Extraído de settings_panel.rs para respeitar o hardcap de 250L.
// Cada função renderiza o conteúdo de uma aba específica.

use crate::app::ColorMode;
use crate::pdf::persistence::PersistenceManager;
use crate::settings::{
    AppSettings, ColorPalette, DefaultPanel, FontChoice, Language, PageMargin, RenderQuality,
    UiScale,
};
use crate::t;
use crate::ui::settings_widgets::{action_btn, chip, divider, sec, toggle_row, Tokens};
use eframe::egui;

// ── Tab: Aparência ─────────────────────────────────────────────────────────────

pub(super) fn tab_appearance(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    color_mode: &mut ColorMode,
    cm: ColorMode,
    needs_font_reload: &mut bool,
    needs_render_reload: &mut bool,
    tok: &Tokens,
) {
    sec(ui, t!("settings-ui-scale"), tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for s in UiScale::all() {
            let sel = settings.scale == s;
            if chip(ui, s.label(), sel, tok).clicked() {
                settings.scale = s;
                ui.ctx().set_pixels_per_point(s.pixels_per_point());
                settings.save();
            }
        }
    });

    ui.add_space(18.0);
    sec(ui, t!("settings-font"), tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for f in FontChoice::all() {
            let sel = settings.font == f;
            if chip(ui, f.label(), sel, tok).clicked() {
                settings.font = f;
                *needs_font_reload = true;
                settings.save();
            }
        }
    });

    ui.add_space(18.0);
    sec(ui, t!("settings-palette"), tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for i in 0u8..4 {
            let name = match cm {
                ColorMode::Dark => ColorPalette::dark_name(i),
                ColorMode::Light => ColorPalette::light_name(i),
                ColorMode::Sepia => ColorPalette::sepia_name(i),
            };
            let sel = settings.palette.0 == i;
            if chip(ui, name, sel, tok).clicked() {
                settings.palette = ColorPalette(i);
                settings.save();
                *needs_render_reload = true;
            }
        }
    });

    ui.add_space(18.0);
    sec(ui, t!("settings-color-mode"), tok.subtle);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (label, mode) in [
            ("Dark", ColorMode::Dark),
            ("Light", ColorMode::Light),
            ("Sépia", ColorMode::Sepia),
        ] {
            let sel = *color_mode == mode;
            if chip(ui, label, sel, tok).clicked() {
                *color_mode = mode;
            }
        }
    });

    ui.add_space(18.0);
    sec(ui, t!("settings-language"), tok.subtle);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for lang in Language::all() {
            let sel = settings.language == lang;
            if chip(ui, lang.label(), sel, tok).clicked() {
                settings.language = lang;
                settings.save();
                crate::i18n::init(lang);
            }
        }
    });
}

// ── Tab: Leitura ───────────────────────────────────────────────────────────────

pub(super) fn tab_reading(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    needs_render_reload: &mut bool,
    tok: &Tokens,
) {
    sec(ui, t!("settings-page-margin"), tok.subtle);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for m in PageMargin::all() {
            let sel = settings.page_margin == m;
            if chip(ui, m.label(), sel, tok).clicked() {
                settings.page_margin = m;
                settings.save();
            }
        }
    });

    ui.add_space(18.0);
    sec(ui, t!("settings-render-quality"), tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for q in RenderQuality::all() {
            let sel = settings.render_quality == q;
            if chip(ui, q.label(), sel, tok).clicked() {
                settings.render_quality = q;
                *needs_render_reload = true;
                settings.save();
            }
        }
    });

    ui.add_space(24.0);
    if toggle_row(
        ui,
        t!("settings-smooth-scroll"),
        t!("settings-smooth-scroll-desc"),
        &mut settings.smooth_scroll,
        tok,
    ) {
        settings.save();
    }
}

// ── Tab: Privacidade ───────────────────────────────────────────────────────────

pub(super) fn tab_privacy(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    persistence: &mut PersistenceManager,
    tok: &Tokens,
) {
    sec(ui, t!("settings-default-panel"), tok.subtle);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for p in DefaultPanel::all() {
            let sel = settings.default_panel == p;
            if chip(ui, p.label(), sel, tok).clicked() {
                settings.default_panel = p;
                settings.save();
            }
        }
    });

    ui.add_space(24.0);
    if toggle_row(
        ui,
        t!("settings-track-history"),
        t!("settings-track-history-desc"),
        &mut settings.track_history,
        tok,
    ) {
        settings.save();
    }

    ui.add_space(20.0);
    divider(ui, tok.bord, tok.dw);
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(t!("settings-clear-history"))
                    .size(13.0)
                    .color(tok.text),
            );
            ui.label(
                egui::RichText::new(t!("settings-clear-history-desc"))
                    .size(11.0)
                    .color(tok.muted),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = action_btn(
                ui,
                &format!("  {}  ", t!("settings-clear-btn")),
                tok.danger,
                tok.on_acc,
                tok,
            );
            if resp.clicked() {
                persistence.clear_all();
            }
        });
    });

    // ── Integração Windows ─────────────────────────────────────────────────
    ui.add_space(20.0);
    divider(ui, tok.bord, tok.dw);
    ui.add_space(16.0);
    sec(ui, t!("settings-windows"), tok.subtle);
    ui.add_space(8.0);

    let is_assoc = crate::platform::file_assoc::is_pdf_associated();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(t!("settings-assoc-title"))
                    .size(13.0)
                    .color(tok.text),
            );
            if is_assoc {
                ui.label(
                    egui::RichText::new(t!("settings-assoc-active"))
                        .size(11.0)
                        .color(tok.accent),
                );
            } else {
                ui.label(
                    egui::RichText::new(t!("settings-assoc-inactive"))
                        .size(11.0)
                        .color(tok.muted),
                );
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if is_assoc {
                let resp = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-assoc-remove-btn")),
                    tok.danger,
                    tok.on_acc,
                    tok,
                );
                if resp.clicked() {
                    let _ = crate::platform::file_assoc::unregister_pdf_association();
                }
            } else {
                let resp = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-assoc-set-btn")),
                    tok.accent,
                    tok.on_acc,
                    tok,
                );
                if resp.clicked() {
                    let _ = crate::platform::file_assoc::register_pdf_association();
                }
            }
        });
    });
}
