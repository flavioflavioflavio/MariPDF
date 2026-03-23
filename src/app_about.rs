// app_about.rs — About dialog do MariPDF
//
// Exibido via Ctrl+Shift+A ou botão "?" na toolbar.
// Mostra versão, descrição, link GitHub e licença.

use eframe::egui;
use crate::app_constants::{BG_SURFACE, ACCENT_PRIMARY, TEXT_PRIMARY, TEXT_MUTED};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_URL: &str = "https://github.com/flaviomarcilio/MariPDF";

pub struct AboutState {
    pub open: bool,
}

impl Default for AboutState {
    fn default() -> Self { Self { open: false } }
}

impl AboutState {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open { return; }

        let screen = ctx.screen_rect();
        let w = 380.0_f32;
        let h = 260.0_f32;
        let pos = egui::pos2(
            (screen.center().x - w / 2.0).max(0.0),
            (screen.center().y - h / 2.0).max(0.0),
        );

        egui::Window::new("Sobre o MariPDF")
            .fixed_pos(pos)
            .fixed_size([w, h])
            .collapsible(false)
            .resizable(false)
            .frame(egui::Frame::window(&ctx.style())
                .fill(BG_SURFACE)
                .inner_margin(egui::Margin::same(24))
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Logo / nome
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("MariPDF")
                        .font(egui::FontId::new(28.0, egui::FontFamily::Name("Title".into())))
                        .color(ACCENT_PRIMARY));

                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(format!("Versão {VERSION}"))
                        .size(13.0).color(TEXT_MUTED));

                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Leitor de PDF ultra-leve para Windows.")
                        .size(13.0).color(TEXT_PRIMARY));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Rápido, portátil, sem instalar.")
                        .size(13.0).color(TEXT_MUTED));

                    ui.add_space(16.0);

                    // Link GitHub
                    if ui.link(egui::RichText::new("🔗  GitHub")
                        .color(ACCENT_PRIMARY).size(13.0))
                        .clicked()
                    {
                        let _ = open::that(REPO_URL);
                    }

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Licença GPLv3  •  © 2026 Flávio Marcílio")
                        .size(11.0).color(TEXT_MUTED));

                    ui.add_space(16.0);

                    // Botão fechar
                    if ui.button(egui::RichText::new("Fechar").size(13.0))
                        .clicked()
                    {
                        self.open = false;
                    }
                });
            });
    }
}
