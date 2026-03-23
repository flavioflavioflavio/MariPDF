// settings_panel.rs — Painel de configurações do MariPDF
// Layout: tabs (Aparência / Leitura / Privacidade)
// Todas as cores via tokens do ColorMode — zero hardcoded

use eframe::egui;
use crate::settings::{
    AppSettings, UiScale, FontChoice, RenderQuality, PageMargin, DefaultPanel, ColorPalette,
};
use crate::app::ColorMode;
use crate::ui::icons;

// ── Tokens de cor derivados do ColorMode ──────────────────────────────────────

struct Tokens {
    surf:    egui::Color32,
    surf_hv: egui::Color32,  // surface em hover
    stat:    egui::Color32,
    bord:    egui::Color32,
    text:    egui::Color32,
    muted:   egui::Color32,
    subtle:  egui::Color32,
    accent:  egui::Color32,
    on_acc:  egui::Color32,
    danger:  egui::Color32,
    dw:      f32,
}

impl Tokens {
    fn from(cm: ColorMode) -> Self {
        Self {
            surf:    cm.surface(),
            surf_hv: cm.surface_hover(),
            stat:    cm.statusbar(),
            bord:    cm.border(),
            text:    cm.text_primary(),
            muted:   cm.text_muted(),
            subtle:  cm.text_subtle(),
            accent:  cm.accent(),
            on_acc:  cm.on_accent(),
            danger:  cm.danger(),
            dw:      cm.divider_width(),
        }
    }
}

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

// ── Tab: Aparência ────────────────────────────────────────────────────────────

fn tab_appearance(
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

// ── Tab: Leitura ──────────────────────────────────────────────────────────────

fn tab_reading(
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

// ── Tab: Privacidade ──────────────────────────────────────────────────────────

fn tab_privacy(
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sec(ui: &mut egui::Ui, label: &str, subtle: egui::Color32) {
    ui.label(egui::RichText::new(label).size(11.0).color(subtle).strong());
    ui.add_space(10.0);
}

fn divider(ui: &mut egui::Ui, bord: egui::Color32, dw: f32) {
    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top();
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(dw, bord),
    );
}

/// Lerp linear entre duas cores
fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Chip button com hover animado real: só COR muda (sem layout shift)
fn chip(
    ui:       &mut egui::Ui,
    label:    &str,
    selected: bool,
    tok:      &Tokens,
) -> egui::Response {
    let id      = ui.id().with(label).with("chip_hov");
    let hov_id  = id.with("state");

    // animação 1-frame behind: lemos o hover do frame anterior
    let was_hov: bool = ui.ctx().data(|d| d.get_temp(hov_id).unwrap_or(false));
    let t = ui.ctx().animate_bool_with_time(id, was_hov, 0.12);

    // ── cores animadas (só cor, sem mexer em layout) ──────────────────────────
    let (animated_bg, animated_fg, brd) = if selected {
        // Chip selecionado: hover escurece o accent ~15%
        let darken = (t * 22.0) as u8;
        let hov_acc = egui::Color32::from_rgb(
            tok.accent.r().saturating_sub(darken),
            tok.accent.g().saturating_sub(darken),
            tok.accent.b().saturating_sub(darken),
        );
        (lerp_color(tok.accent, hov_acc, t), tok.on_acc, tok.accent)
    } else {
        // Chip não-selecionado: hover clarea o surface, texto fica mais visível
        let bg  = lerp_color(tok.surf, tok.surf_hv, t);
        let fg  = lerp_color(tok.muted, tok.text, t * 0.65);
        let brd = lerp_color(tok.bord, tok.accent, t * 0.35);
        (bg, fg, brd)
    };

    let resp = ui.add(
        egui::Button::new(egui::RichText::new(label).size(12.5).color(animated_fg))
            .fill(animated_bg)
            .stroke(egui::Stroke::new(tok.dw * 1.2, brd))
            .corner_radius(egui::CornerRadius::same(24))
    );

    // Salva estado para próximo frame
    let is_hov = resp.hovered();
    ui.ctx().data_mut(|d| d.insert_temp(hov_id, is_hov));
    if is_hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.ctx().request_repaint();
    }
    resp

}

/// Botão de ação primário (Fechar / Limpar) com hover escurecido + cursor hand
fn action_btn(
    ui:    &mut egui::Ui,
    label: &str,
    bg:    egui::Color32,
    fg:    egui::Color32,
    tok:   &Tokens,
) -> egui::Response {
    let id     = ui.id().with(label).with("act_hov");
    let hov_id = id.with("state");
    let was_hov: bool = ui.ctx().data(|d| d.get_temp(hov_id).unwrap_or(false));
    let t = ui.ctx().animate_bool_with_time(id, was_hov, 0.12);

    // Escurece levemente o bg no hover
    let hover_bg = egui::Color32::from_rgba_unmultiplied(
        bg.r().saturating_sub((12.0 * t) as u8),
        bg.g().saturating_sub((12.0 * t) as u8),
        bg.b().saturating_sub((10.0 * t) as u8),
        bg.a(),
    );

    let resp = ui.add(
        egui::Button::new(egui::RichText::new(label).size(12.5).color(fg))
            .fill(hover_bg)
            .stroke(egui::Stroke::new(tok.dw, lerp_color(bg, tok.surf, t * 0.2)))
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(80.0, 32.0))
    ).on_hover_cursor(egui::CursorIcon::PointingHand);

    let is_hov = resp.hovered();
    ui.ctx().data_mut(|d| d.insert_temp(hov_id, is_hov));
    if is_hov { ui.ctx().request_repaint(); }
    resp
}

/// Toggle ON/OFF com cursor hand — retorna true se mudou
fn toggle_row(
    ui:       &mut egui::Ui,
    title:    &str,
    subtitle: &str,
    value:    &mut bool,
    tok:      &Tokens,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(13.0).color(tok.text));
            ui.label(egui::RichText::new(subtitle).size(11.0).color(tok.muted));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let col = if *value { tok.accent } else { tok.muted };
            let lbl = if *value { " ON " } else { " OFF" };
            let resp = ui.add(
                egui::Button::new(egui::RichText::new(lbl).size(11.5).color(tok.on_acc))
                    .fill(col).stroke(egui::Stroke::NONE)
                    .corner_radius(egui::CornerRadius::same(24))
                    .min_size(egui::vec2(56.0, 28.0))
            ).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                *value = !*value;
                changed = true;
            }
        });
    });
    changed
}
