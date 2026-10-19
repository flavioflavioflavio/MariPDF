// app_frame/bar.rs — Controles flutuantes do modo Flutuante.
// Invisível fora da janela, um chevron semitransparente sobre a página e
// opacidade total ao passar o mouse na barra. O chevron expande uma pílula com
// zoom, mão de pan, fixar no topo e sair.

use super::FrameAction;
use crate::app::MariApp;
use crate::t;
use crate::ui::icons;
use eframe::egui;

/// Distância da barra até as bordas da moldura.
const BAR_INSET: f32 = 10.0;
/// Folga ao redor da barra que já conta como hover.
const BAR_HOVER_SLACK: f32 = 8.0;
/// Opacidade do chevron/barra com o mouse sobre a página.
const IDLE_ALPHA: f32 = 0.4;
const FADE_SECS: f32 = 0.18;
const BTN_SIZE: egui::Vec2 = egui::vec2(26.0, 24.0);

/// Opacidade alvo: some fora da janela e durante arraste, aparece leve sobre
/// a página e por inteiro quando o ponteiro está na barra.
pub(super) fn bar_target_alpha(pointer_in_window: bool, over_bar: bool, dragging: bool) -> f32 {
    match (pointer_in_window, over_bar, dragging) {
        (_, true, _) => 1.0,
        (false, ..) | (_, _, true) => 0.0,
        _ => IDLE_ALPHA,
    }
}

impl MariApp {
    pub(super) fn frame_bar(
        &mut self,
        ctx: &egui::Context,
        window: egui::Rect,
    ) -> Option<FrameAction> {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let over_bar = match (pointer, self.frame.bar_rect) {
            (Some(pos), Some(rect)) => rect.expand(BAR_HOVER_SLACK).contains(pos),
            _ => false,
        };
        let dragging = ctx.input(|i| i.pointer.any_down()) && !over_bar;
        let target = bar_target_alpha(pointer.is_some(), over_bar, dragging);
        let alpha =
            ctx.animate_value_with_time(egui::Id::new("frame_bar_alpha"), target, FADE_SECS);
        if alpha < 0.01 {
            // Mantém a área conhecida para o hover reaparecer sem piscar.
            return None;
        }

        let cm = self.color_mode;
        let palette = self.settings.palette.0;
        let colors = BarColors {
            fill: cm.statusbar_for(palette).gamma_multiply(0.92),
            border: cm.border(),
            icon: cm.text_muted(),
            strong: cm.text_primary(),
            hover: cm.surface_hover(),
            accent: cm.accent_for(palette),
        };
        let mut action = None;

        let area = egui::Area::new(egui::Id::new("frame_bar"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(egui::pos2(
                window.right() - BAR_INSET,
                window.top() + BAR_INSET,
            ))
            .show(ctx, |ui| {
                ui.multiply_opacity(alpha);
                egui::Frame::new()
                    .fill(colors.fill)
                    .stroke(egui::Stroke::new(1.0_f32, colors.border))
                    .corner_radius(egui::CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                        ui.horizontal(|ui| {
                            if self.frame.bar_open {
                                action = action.or(self.bar_controls(ui, &colors));
                            }
                            let (chevron, tip) = if self.frame.bar_open {
                                (icons::CARET_RIGHT, t!("frame-bar-close"))
                            } else {
                                (icons::CARET_LEFT, t!("frame-bar-open"))
                            };
                            let label = icons::ph(chevron, 13.0, colors.icon);
                            if bar_button(ui, label, false, &colors)
                                .on_hover_text(tip)
                                .clicked()
                            {
                                action = Some(FrameAction::ToggleBar);
                            }
                        });
                    });
            });
        self.frame.bar_rect = Some(area.response.rect);
        if alpha < 0.99 || target != alpha {
            ctx.request_repaint();
        }
        action
    }

    fn bar_controls(&self, ui: &mut egui::Ui, colors: &BarColors) -> Option<FrameAction> {
        let mut action = None;
        let button = |ui: &mut egui::Ui,
                      label: egui::RichText,
                      active: bool,
                      tip: String,
                      value: FrameAction| {
            bar_button(ui, label, active, colors)
                .on_hover_text(tip)
                .clicked()
                .then_some(value)
        };

        let zoomed = self.frame.zoom > 1.0;
        action = action.or(button(
            ui,
            icons::ph(icons::MINUS, 13.0, colors.icon),
            false,
            t!("frame-zoom-out"),
            FrameAction::ZoomOut,
        ));
        let pct = egui::RichText::new(format!("{:.0}%", self.frame.zoom * 100.0))
            .size(11.0)
            .color(if zoomed { colors.strong } else { colors.icon });
        if zoom_label(ui, pct, colors)
            .on_hover_text(t!("frame-zoom-fit"))
            .clicked()
        {
            action = Some(FrameAction::ResetView);
        }
        action = action.or(button(
            ui,
            icons::ph(icons::PLUS, 13.0, colors.icon),
            false,
            t!("frame-zoom-in"),
            FrameAction::ZoomIn,
        ));
        separator(ui, colors);

        let pan_on = self.frame.pan_tool;
        let pan_tip = if pan_on {
            t!("frame-pan-on")
        } else {
            t!("frame-pan-off")
        };
        let hand_color = if pan_on { colors.accent } else { colors.icon };
        action = action.or(button(
            ui,
            icons::ph(icons::HAND, 14.0, hand_color),
            pan_on,
            pan_tip,
            FrameAction::TogglePan,
        ));
        let pin_color = if self.frame.always_on_top {
            colors.accent
        } else {
            colors.icon
        };
        action = action.or(button(
            ui,
            icons::ph(icons::PUSH_PIN, 14.0, pin_color),
            self.frame.always_on_top,
            t!("frame-always-on-top"),
            FrameAction::ToggleOnTop,
        ));
        separator(ui, colors);
        action = action.or(button(
            ui,
            icons::ph(icons::X_CIRCLE, 14.0, colors.icon),
            false,
            t!("frame-exit"),
            FrameAction::Exit,
        ));
        action
    }
}

struct BarColors {
    fill: egui::Color32,
    border: egui::Color32,
    icon: egui::Color32,
    strong: egui::Color32,
    hover: egui::Color32,
    accent: egui::Color32,
}

/// Botão redondo discreto: fundo só no hover ou quando ativo.
fn bar_button(
    ui: &mut egui::Ui,
    label: egui::RichText,
    active: bool,
    colors: &BarColors,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(BTN_SIZE, egui::Sense::click());
    paint_button(ui, rect, &resp, label, active, colors);
    resp
}

fn zoom_label(ui: &mut egui::Ui, label: egui::RichText, colors: &BarColors) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(42.0, BTN_SIZE.y), egui::Sense::click());
    paint_button(ui, rect, &resp, label, false, colors);
    resp
}

fn paint_button(
    ui: &egui::Ui,
    rect: egui::Rect,
    resp: &egui::Response,
    label: egui::RichText,
    active: bool,
    colors: &BarColors,
) {
    let bg = if active {
        colors.accent.gamma_multiply(0.18)
    } else if resp.hovered() {
        colors.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(12), bg);
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let pos = rect.center() - galley.size() * 0.5;
    ui.painter().galley(pos, galley, colors.icon);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

fn separator(ui: &mut egui::Ui, colors: &BarColors) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(7.0, BTN_SIZE.y), egui::Sense::hover());
    let x = rect.center().x;
    ui.painter().line_segment(
        [
            egui::pos2(x, rect.top() + 6.0),
            egui::pos2(x, rect.bottom() - 6.0),
        ],
        egui::Stroke::new(1.0_f32, colors.border),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_is_hidden_outside_and_while_dragging() {
        assert_eq!(bar_target_alpha(false, false, false), 0.0);
        assert_eq!(bar_target_alpha(true, false, true), 0.0);
    }

    #[test]
    fn bar_is_faint_over_page_and_solid_on_hover() {
        assert_eq!(bar_target_alpha(true, false, false), IDLE_ALPHA);
        assert_eq!(bar_target_alpha(true, true, false), 1.0);
        // Ponteiro na barra vence mesmo com botão pressionado (clique nela).
        assert_eq!(bar_target_alpha(true, true, true), 1.0);
    }
}
