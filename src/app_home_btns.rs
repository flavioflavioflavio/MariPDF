// app_home_btns.rs — Botões flutuantes da home (configurações e tema)
//
// Os dois botões têm o mesmo tamanho e o mesmo estilo "fantasma": sem fundo
// em repouso, superfície e borda no hover.

use crate::app::MariApp;
use crate::app_color_mode::ColorMode;
use crate::ui::icons;
use eframe::egui;

const BUTTON: f32 = 36.0;
const GAP: f32 = 6.0;
const MARGIN: f32 = 14.0;

impl MariApp {
    pub(crate) fn ui_home_buttons(&mut self, ctx: &egui::Context) {
        let cm = self.color_mode;
        let screen = ctx.screen_rect();
        let theme_icon = match cm {
            ColorMode::Dark => icons::MOON,
            ColorMode::Light => icons::SUN,
            ColorMode::Sepia => icons::LAMP_PENDANT,
        };

        egui::Area::new(egui::Id::new("home_corner_buttons"))
            .fixed_pos(egui::pos2(
                screen.right() - MARGIN - BUTTON * 2.0 - GAP,
                screen.top() + MARGIN,
            ))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    let settings = ghost_button(ui, cm, icons::GEAR, "home_settings_btn")
                        .on_hover_text(crate::t!("toolbar-settings-hint"));
                    if settings.clicked() {
                        self.settings_open = !self.settings_open;
                    }
                    let theme = ghost_button(ui, cm, theme_icon, "home_theme_btn")
                        .on_hover_text(cm.label());
                    if theme.clicked() {
                        self.color_mode = cm.next();
                        self.viewer.tex_cache.clear();
                        self.viewer.texture = None;
                        self.viewer.loaded_page = None;
                        self.pending_rerender = true;
                    }
                });
            });
    }
}

fn ghost_button(ui: &mut egui::Ui, cm: ColorMode, glyph: &str, id: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(BUTTON, BUTTON), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(egui::Id::new(id), response.hovered(), 0.10);
    let painter = ui.painter();
    if hover > 0.0 {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(10),
            cm.surface().gamma_multiply(hover),
        );
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(10),
            egui::Stroke::new(1.0_f32, cm.border().gamma_multiply(hover)),
            egui::StrokeKind::Inside,
        );
    }
    let color = if response.hovered() {
        cm.text_primary()
    } else {
        cm.text_muted()
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::new(19.0, egui::FontFamily::Name("Icons".into())),
        color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
