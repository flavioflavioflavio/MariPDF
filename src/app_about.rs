// app_about.rs — About dialog do MariPDF
//
// Exibido via Ctrl+Shift+I ou botão "?" na toolbar.
// Strings via t!() para i18n.

use crate::app_constants::{ACCENT_PRIMARY, BG_SURFACE, TEXT_MUTED, TEXT_PRIMARY};
use crate::t;
use eframe::egui;

const REPO_URL: &str = "https://github.com/flavioflavioflavio/MariPDF";

#[derive(Default)]
pub struct AboutState {
    pub open: bool,
}

impl AboutState {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let screen = ctx.screen_rect();
        let w = 380.0_f32;
        let h = 260.0_f32;
        let pos = egui::pos2(
            (screen.center().x - w / 2.0).max(0.0),
            (screen.center().y - h / 2.0).max(0.0),
        );

        let version = env!("CARGO_PKG_VERSION");

        egui::Window::new(t!("about-title"))
            .fixed_pos(pos)
            .fixed_size([w, h])
            .collapsible(false)
            .resizable(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(BG_SURFACE)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("MariPDF")
                            .font(egui::FontId::new(
                                28.0,
                                egui::FontFamily::Name("Title".into()),
                            ))
                            .color(ACCENT_PRIMARY),
                    );

                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(t!("about-version", version => version))
                            .size(13.0)
                            .color(TEXT_MUTED),
                    );

                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(t!("about-desc"))
                            .size(13.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(t!("app-tagline-sub"))
                            .size(13.0)
                            .color(TEXT_MUTED),
                    );

                    ui.add_space(16.0);

                    if ui
                        .link(
                            egui::RichText::new(t!("btn-github"))
                                .color(ACCENT_PRIMARY)
                                .size(13.0),
                        )
                        .clicked()
                    {
                        let _ = open::that(REPO_URL);
                    }

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(t!("app-license"))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );

                    ui.add_space(16.0);

                    if ui
                        .button(egui::RichText::new(t!("btn-close")).size(13.0))
                        .clicked()
                    {
                        self.open = false;
                    }
                });
            });
    }
}
