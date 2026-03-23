// ui/settings_tabs.rs — Conteúdo das abas do painel de configurações
// Extraído de settings_panel.rs para respeitar o hardcap de 250L.
// Cada função renderiza o conteúdo de uma aba específica.

use eframe::egui;
use crate::app::ColorMode;
use crate::settings::{AppSettings, UiScale, FontChoice, RenderQuality, PageMargin, DefaultPanel, ColorPalette};
use crate::ui::settings_widgets::{Tokens, sec, divider, chip, action_btn, toggle_row};

// ── Tab: Aparência ─────────────────────────────────────────────────────────────

pub(super) fn tab_appearance(
    ui:    &mut egui::Ui,
    settings:  &mut AppSettings,
    color_mode: &mut ColorMode,
    cm:         ColorMode,
    needs_font_reload:   &mut bool,
    needs_render_reload: &mut bool,
    tok:   &Tokens,
) {
    sec(ui, "Tamanho da Interface", tok.subtle);
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
    sec(ui, "Fonte da Interface", tok.subtle);
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
    sec(ui, "Paleta de Cores", tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for i in 0u8..4 {
            let name = match cm {
                ColorMode::Dark  => ColorPalette::dark_name(i),
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
    sec(ui, "Modo de Cor", tok.subtle);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (label, mode) in [("Dark", ColorMode::Dark), ("Light", ColorMode::Light), ("Sépia", ColorMode::Sepia)] {
            let sel = *color_mode == mode;
            if chip(ui, label, sel, tok).clicked() {
                *color_mode = mode;
            }
        }
    });
}

// ── Tab: Leitura ───────────────────────────────────────────────────────────────

pub(super) fn tab_reading(
    ui:       &mut egui::Ui,
    settings: &mut AppSettings,
    needs_render_reload: &mut bool,
    tok:      &Tokens,
) {
    sec(ui, "Margem da Página", tok.subtle);
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
    sec(ui, "Qualidade de Renderização", tok.subtle);
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
    if toggle_row(ui, "Scroll Suave", "Animação ao navegar entre páginas",
                  &mut settings.smooth_scroll, tok) {
        settings.save();
    }
}

// ── Tab: Privacidade ───────────────────────────────────────────────────────────

pub(super) fn tab_privacy(
    ui:       &mut egui::Ui,
    settings: &mut AppSettings,
    tok:      &Tokens,
) {
    sec(ui, "Painel Lateral ao Abrir PDF", tok.subtle);
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
    if toggle_row(ui, "Rastrear Recentes", "Salvar histórico de PDFs abertos",
                  &mut settings.track_history, tok) {
        settings.save();
    }

    ui.add_space(20.0);
    divider(ui, tok.bord, tok.dw);
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Limpar Histórico").size(13.0).color(tok.text));
            ui.label(egui::RichText::new("Remove todos os PDFs da lista de recentes").size(11.0).color(tok.muted));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = action_btn(ui, "  Limpar  ", tok.danger, tok.on_acc, tok);
            if resp.clicked() { /* limpar histórico */ }
        });
    });
}
