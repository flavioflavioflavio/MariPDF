// ui/settings_widgets.rs — Widgets reutilizáveis do painel de configurações
// sec, divider, lerp_color, chip, action_btn, toggle_row

use eframe::egui;

// ── Tokens de cor derivados do ColorMode ──────────────────────────────────────

pub struct Tokens {
    pub surf: egui::Color32,
    pub surf_hv: egui::Color32,
    pub stat: egui::Color32,
    pub bord: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub subtle: egui::Color32,
    pub accent: egui::Color32,
    pub on_acc: egui::Color32,
    pub danger: egui::Color32,
    pub dw: f32,
}

use crate::app::ColorMode;

impl Tokens {
    pub fn from(cm: ColorMode) -> Self {
        Self {
            surf: cm.surface(),
            surf_hv: cm.surface_hover(),
            stat: cm.statusbar(),
            bord: cm.border(),
            text: cm.text_primary(),
            muted: cm.text_muted(),
            subtle: cm.text_subtle(),
            accent: cm.accent(),
            on_acc: cm.on_accent(),
            danger: cm.danger(),
            dw: cm.divider_width(),
        }
    }
}

// ── Helpers visuais ───────────────────────────────────────────────────────────

pub fn sec(ui: &mut egui::Ui, label: impl Into<String>, subtle: egui::Color32) {
    ui.label(
        egui::RichText::new(label.into())
            .size(11.0)
            .color(subtle)
            .strong(),
    );
    ui.add_space(10.0);
}

pub fn divider(ui: &mut egui::Ui, bord: egui::Color32, dw: f32) {
    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top();
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(dw, bord),
    );
}

/// Lerp linear entre duas cores
pub fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Chip button com hover animado — só COR muda (sem layout shift)
pub fn chip(ui: &mut egui::Ui, label: &str, selected: bool, tok: &Tokens) -> egui::Response {
    let id = ui.id().with(label).with("chip_hov");
    let hov_id = id.with("state");
    let was_hov: bool = ui.ctx().data(|d| d.get_temp(hov_id).unwrap_or(false));
    let t = ui.ctx().animate_bool_with_time(id, was_hov, 0.12);

    let (animated_bg, animated_fg, brd) = if selected {
        let darken = (t * 22.0) as u8;
        let hov_acc = egui::Color32::from_rgb(
            tok.accent.r().saturating_sub(darken),
            tok.accent.g().saturating_sub(darken),
            tok.accent.b().saturating_sub(darken),
        );
        (lerp_color(tok.accent, hov_acc, t), tok.on_acc, tok.accent)
    } else {
        let bg = lerp_color(tok.surf, tok.surf_hv, t);
        let fg = lerp_color(tok.muted, tok.text, t * 0.65);
        let brd = lerp_color(tok.bord, tok.accent, t * 0.35);
        (bg, fg, brd)
    };

    let resp = ui.add(
        egui::Button::new(egui::RichText::new(label).size(12.5).color(animated_fg))
            .fill(animated_bg)
            .stroke(egui::Stroke::new(tok.dw * 1.2, brd))
            .corner_radius(egui::CornerRadius::same(24)),
    );
    let is_hov = resp.hovered();
    ui.ctx().data_mut(|d| d.insert_temp(hov_id, is_hov));
    if is_hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.ctx().request_repaint();
    }
    resp
}

/// Botão de ação primário com hover escurecido + cursor hand
pub fn action_btn(
    ui: &mut egui::Ui,
    label: &str,
    bg: egui::Color32,
    fg: egui::Color32,
    tok: &Tokens,
) -> egui::Response {
    let id = ui.id().with(label).with("act_hov");
    let hov_id = id.with("state");
    let was_hov: bool = ui.ctx().data(|d| d.get_temp(hov_id).unwrap_or(false));
    let t = ui.ctx().animate_bool_with_time(id, was_hov, 0.12);

    let hover_bg = egui::Color32::from_rgba_unmultiplied(
        bg.r().saturating_sub((12.0 * t) as u8),
        bg.g().saturating_sub((12.0 * t) as u8),
        bg.b().saturating_sub((10.0 * t) as u8),
        bg.a(),
    );
    let resp = ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(12.5).color(fg))
                .fill(hover_bg)
                .stroke(egui::Stroke::new(tok.dw, lerp_color(bg, tok.surf, t * 0.2)))
                .corner_radius(egui::CornerRadius::same(8))
                .min_size(egui::vec2(80.0, 32.0)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let is_hov = resp.hovered();
    ui.ctx().data_mut(|d| d.insert_temp(hov_id, is_hov));
    if is_hov {
        ui.ctx().request_repaint();
    }
    resp
}

/// Toggle ON/OFF com cursor hand — retorna true se mudou
pub fn toggle_row(
    ui: &mut egui::Ui,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    value: &mut bool,
    tok: &Tokens,
) -> bool {
    let title_s = title.into();
    let subtitle_s = subtitle.into();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&title_s).size(13.0).color(tok.text));
            ui.label(egui::RichText::new(&subtitle_s).size(11.0).color(tok.muted));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let col = if *value { tok.accent } else { tok.muted };
            let lbl = if *value { " ON " } else { " OFF" };
            let resp = ui
                .add(
                    egui::Button::new(egui::RichText::new(lbl).size(11.5).color(tok.on_acc))
                        .fill(col)
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(24))
                        .min_size(egui::vec2(56.0, 28.0)),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                *value = !*value;
                changed = true;
            }
        });
    });
    changed
}
