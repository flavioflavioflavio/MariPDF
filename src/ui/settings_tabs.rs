// ui/settings_tabs.rs — Conteúdo das abas do painel de configurações
// Extraído de settings_panel.rs para respeitar o hardcap de 250L.
// Cada função renderiza o conteúdo de uma aba específica.

use crate::app::ColorMode;
use crate::pdf::persistence::PersistenceManager;
use crate::settings::{
    AppSettings, ColorPalette, DefaultPanel, FontChoice, Language, PageMargin, RenderQuality,
    ThemePreference, UiScale,
};
use crate::t;
use crate::ui::settings_widgets::{action_btn, chip, divider, sec, toggle_row, Tokens};
use eframe::egui;

// ── Tab: Aparência ─────────────────────────────────────────────────────────────

pub(super) fn tab_appearance(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    color_mode: &mut ColorMode,
    system_prefers_dark: bool,
    needs_font_reload: &mut bool,
    needs_render_reload: &mut bool,
    tok: &Tokens,
) {
    let cm = *color_mode;
    sec(ui, t!("settings-ui-scale"), tok.subtle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for s in UiScale::all() {
            let sel = settings.scale == s;
            if chip(ui, &s.label(), sel, tok).clicked() {
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
            if chip(ui, &name, sel, tok).clicked() {
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
        for theme in ThemePreference::all() {
            let sel = settings.theme == theme;
            if chip(ui, &t!(theme.label_key()), sel, tok).clicked() {
                settings.theme = theme;
                *color_mode = theme.resolve(system_prefers_dark);
                settings.save();
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
            if chip(ui, &m.label(), sel, tok).clicked() {
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
            if chip(ui, &q.label(), sel, tok).clicked() {
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
            if chip(ui, &p.label(), sel, tok).clicked() {
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

    clear_history_row(ui, persistence, tok);

    // ── Integração Windows ─────────────────────────────────────────────────
    ui.add_space(20.0);
    divider(ui, tok.bord, tok.dw);
    ui.add_space(16.0);
    sec(ui, t!("settings-windows"), tok.subtle);
    ui.add_space(8.0);

    use crate::platform::file_assoc::{self, AssociationState};
    let state = file_assoc::association_state();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(t!("settings-assoc-title"))
                    .size(13.0)
                    .color(tok.text),
            );
            let (status, color) = match state {
                AssociationState::Default => (t!("settings-assoc-active"), tok.accent),
                AssociationState::Registered => (t!("settings-assoc-pending"), tok.muted),
                AssociationState::NotRegistered => (t!("settings-assoc-inactive"), tok.muted),
            };
            ui.label(egui::RichText::new(status).size(11.0).color(color));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state == AssociationState::Default {
                // O UserChoice é protegido: remover o ProgID agora deixaria o
                // .pdf apontando para um handler inexistente. O usuário troca
                // o padrão no Windows primeiro; aí o Remover aparece.
                let resp = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-assoc-change-btn")),
                    tok.accent,
                    tok.on_acc,
                    tok,
                );
                if resp.clicked() {
                    file_assoc::open_default_apps();
                }
            }
            if state == AssociationState::Registered {
                let resp = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-assoc-remove-btn")),
                    tok.danger,
                    tok.on_acc,
                    tok,
                );
                if resp.clicked() {
                    if let Err(error) = file_assoc::unregister_pdf_association() {
                        tracing::warn!("associação .pdf: falha ao remover: {error}");
                        crate::notices::error(t!("settings-assoc-failed", error => error));
                    }
                }
            }
            if state != AssociationState::Default {
                let resp = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-assoc-set-btn")),
                    tok.accent,
                    tok.on_acc,
                    tok,
                );
                if resp.clicked() {
                    match file_assoc::register_pdf_association() {
                        Ok(()) => crate::notices::info(t!("settings-assoc-choose")),
                        Err(error) => {
                            tracing::warn!("associação .pdf: falha ao registrar: {error}");
                            crate::notices::error(t!("settings-assoc-failed", error => error));
                        }
                    }
                }
            }
        });
    });
}

// ── Limpar histórico: confirmação em duas etapas + feedback ───────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum ClearHistoryState {
    #[default]
    Idle,
    Confirming,
    Done(f64),
    Failed(f64),
}

const CLEAR_FEEDBACK_SECS: f64 = 3.0;

fn clear_history_row(ui: &mut egui::Ui, persistence: &mut PersistenceManager, tok: &Tokens) {
    let state_id = egui::Id::new("settings_clear_history_state");
    let now = ui.input(|i| i.time);
    let mut state: ClearHistoryState = ui.data(|d| d.get_temp(state_id).unwrap_or_default());
    if let ClearHistoryState::Done(at) | ClearHistoryState::Failed(at) = state {
        if now - at >= CLEAR_FEEDBACK_SECS {
            state = ClearHistoryState::Idle;
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(CLEAR_FEEDBACK_SECS));
        }
    }

    let (desc, desc_color) = match state {
        ClearHistoryState::Idle => (t!("settings-clear-history-desc"), tok.muted),
        ClearHistoryState::Confirming => (t!("settings-clear-history-confirm"), tok.danger),
        ClearHistoryState::Done(_) => (t!("settings-clear-history-done"), tok.accent),
        ClearHistoryState::Failed(_) => (t!("settings-clear-history-failed"), tok.danger),
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(t!("settings-clear-history"))
                    .size(13.0)
                    .color(tok.text),
            );
            ui.label(egui::RichText::new(desc).size(11.0).color(desc_color));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let confirming = state == ClearHistoryState::Confirming;
            let label = if confirming {
                t!("settings-clear-confirm-btn")
            } else {
                t!("settings-clear-btn")
            };
            let resp = action_btn(ui, &format!("  {label}  "), tok.danger, tok.on_acc, tok);
            if resp.clicked() {
                state = if confirming {
                    match persistence.clear_all() {
                        Ok(()) => ClearHistoryState::Done(now),
                        Err(_) => ClearHistoryState::Failed(now),
                    }
                } else {
                    ClearHistoryState::Confirming
                };
            }
            if confirming {
                let cancel = action_btn(
                    ui,
                    &format!("  {}  ", t!("settings-cancel-btn")),
                    tok.stat,
                    tok.text,
                    tok,
                );
                if cancel.clicked() {
                    state = ClearHistoryState::Idle;
                }
            }
        });
    });

    ui.data_mut(|d| d.insert_temp(state_id, state));
}
