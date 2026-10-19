// app_about.rs — Diálogo "Sobre" do MariPDF
//
// Exibido via Ctrl+Shift+I ou pelo botão ⓘ da toolbar. Textos via t!().

use crate::app_constants::{ACCENT_LIGHT, ACCENT_PRIMARY, BG_SURFACE, TEXT_MUTED, TEXT_PRIMARY};
use crate::t;
use eframe::egui;

const REPO_URL: &str = "https://github.com/flavioflavioflavio/MariPDF";
const LINKEDIN_URL: &str = "https://www.linkedin.com/in/takemotoflavio/";
const EVOMIND_URL: &str = "https://evomind.app/investors";

const DIALOG_WIDTH: f32 = 440.0;

#[derive(Default)]
pub struct AboutState {
    pub open: bool,
}

impl AboutState {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let version = env!("CARGO_PKG_VERSION");
        let mut close = false;

        egui::Window::new(t!("about-title"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size([DIALOG_WIDTH, 0.0])
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(BG_SURFACE)
                    .inner_margin(egui::Margin::symmetric(28, 24)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("MariPDF")
                            .font(egui::FontId::new(
                                32.0,
                                egui::FontFamily::Name("Title".into()),
                            ))
                            .color(ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(t!("about-version", version => version))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(t!("about-desc"))
                            .size(13.5)
                            .color(TEXT_PRIMARY),
                    );

                    // ── Dedicatória ─────────────────────────────────────────
                    ui.add_space(18.0);
                    ui.label(egui::RichText::new("♥").size(18.0).color(ACCENT_LIGHT));
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(t!("about-dedication"))
                            .size(14.0)
                            .italics()
                            .color(TEXT_PRIMARY),
                    );

                    // ── Autor ───────────────────────────────────────────────
                    ui.add_space(18.0);
                    ui.separator();
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(t!("about-author-title"))
                            .size(10.5)
                            .strong()
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(t!("about-author-name"))
                            .size(16.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(t!("about-author-bio"))
                                .size(12.5)
                                .color(TEXT_MUTED),
                        )
                        .wrap(),
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let links = [
                            (t!("about-link-linkedin"), LINKEDIN_URL),
                            (t!("about-link-evomind"), EVOMIND_URL),
                            (t!("btn-github"), REPO_URL),
                        ];
                        // Centraliza a linha de links dentro do diálogo.
                        let spacing = ui.spacing().item_spacing.x * 4.0;
                        let estimate: f32 = links
                            .iter()
                            .map(|(label, _)| label.chars().count() as f32 * 7.0 + spacing)
                            .sum();
                        ui.add_space(((ui.available_width() - estimate) * 0.5).max(0.0));
                        for (index, (label, url)) in links.into_iter().enumerate() {
                            if index > 0 {
                                ui.label(egui::RichText::new("·").color(TEXT_MUTED));
                            }
                            let link = ui
                                .link(egui::RichText::new(label).size(12.5).color(ACCENT_PRIMARY));
                            if link.on_hover_text(url).clicked() {
                                let _ = open::that(url);
                            }
                        }
                    });

                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(t!("app-license"))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(14.0);
                    if ui
                        .button(egui::RichText::new(t!("btn-close")).size(13.0))
                        .clicked()
                    {
                        close = true;
                    }
                });
            });

        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
        }
    }
}
