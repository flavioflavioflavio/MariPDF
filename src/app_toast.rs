// app_toast.rs — UI de notificações (Toast) do MariPDF
// Extraído de app.rs para manter abaixo de 250 linhas.

use crate::app::MariApp;
use crate::app_constants::ToastKind;
use eframe::egui;

impl MariApp {
    pub(crate) fn ui_toast(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if let Some(ref t) = self.toast.clone() {
            if t.is_alive(now) {
                let alpha = t.alpha(now);
                let msg = t.message.clone();
                let (bg, stroke_c, icon, icon_c, text_c) = match t.kind {
                    ToastKind::Error => (
                        egui::Color32::from_rgba_unmultiplied(35, 12, 12, (245.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(180, 50, 50, (200.0 * alpha) as u8),
                        "⚠",
                        egui::Color32::from_rgba_unmultiplied(239, 100, 80, (255.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(230, 200, 200, (255.0 * alpha) as u8),
                    ),
                    ToastKind::Info => (
                        egui::Color32::from_rgba_unmultiplied(15, 20, 35, (245.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(120, 80, 220, (200.0 * alpha) as u8),
                        "✓",
                        egui::Color32::from_rgba_unmultiplied(167, 139, 250, (255.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(210, 200, 240, (255.0 * alpha) as u8),
                    ),
                };
                egui::Area::new(egui::Id::new("toast"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(bg)
                            .stroke(egui::Stroke::new(1.0, stroke_c))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(icon).size(14.0).color(icon_c));
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new(&msg).size(12.0).color(text_c));
                                });
                            });
                    });
                ctx.request_repaint();
            } else {
                self.toast = None;
            }
        }
    }
}
