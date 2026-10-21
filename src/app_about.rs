// app_about.rs — Aba "Sobre" das Configurações do MariPDF
//
// Aberta pela aba Sobre ou por Ctrl+Shift+I. Cores via tokens do tema;
// textos via t!().

use crate::t;
use crate::ui::settings_widgets::{sec, Tokens};
use eframe::egui;

const REPO_URL: &str = "https://github.com/flavioflavioflavio/MariPDF";
const LINKEDIN_URL: &str = "https://www.linkedin.com/in/takemotoflavio/";
const EVOMIND_URL: &str = "https://evomind.app/investors";
/// Canal de doação (BF-610). Enquanto não houver um canal oficial configurado,
/// o link não aparece — o README pede para não enviar valores a links antigos.
const DONATION_URL: Option<&str> = None;

const LOGO_HEIGHT: f32 = 30.0;

#[derive(Default)]
pub struct AboutState {
    update: crate::updater::UpdateState,
}

impl AboutState {
    pub fn ui(&mut self, ui: &mut egui::Ui, tok: &Tokens) {
        let version = env!("CARGO_PKG_VERSION");

        // ── Identidade ──────────────────────────────────────────────────────
        ui.add_space(2.0);
        ui.add(crate::app_home::brand_logo(ui.ctx(), LOGO_HEIGHT));
        ui.label(
            egui::RichText::new(t!("about-version", version => version))
                .size(12.0)
                .color(tok.muted),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(t!("about-desc"))
                .size(13.0)
                .color(tok.text),
        );

        // ── Dedicatória ─────────────────────────────────────────────────────
        ui.add_space(6.0);
        egui::Frame::new()
            .fill(tok.accent.gamma_multiply(0.12))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("♥").size(15.0).color(tok.ink));
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(t!("about-dedication"))
                            .size(13.0)
                            .italics()
                            .color(tok.text),
                    );
                });
            });

        // ── Autor ───────────────────────────────────────────────────────────
        ui.add_space(10.0);
        sec(ui, t!("about-author-title"), tok.subtle);
        ui.label(
            egui::RichText::new(t!("about-author-name"))
                .size(15.0)
                .strong()
                .color(tok.text),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(t!("about-author-bio"))
                    .size(12.0)
                    .color(tok.muted),
            )
            .wrap(),
        );
        ui.horizontal(|ui| {
            let links = [
                (t!("about-link-linkedin"), LINKEDIN_URL),
                (t!("about-link-evomind"), EVOMIND_URL),
                (t!("btn-github"), REPO_URL),
            ];
            for (index, (label, url)) in links.into_iter().enumerate() {
                if index > 0 {
                    ui.label(egui::RichText::new("·").color(tok.subtle));
                }
                let link = ui.link(egui::RichText::new(label).size(12.5).color(tok.ink));
                if link.on_hover_text(url).clicked() {
                    let _ = open::that(url);
                }
            }
            if let Some(url) = DONATION_URL {
                ui.label(egui::RichText::new("·").color(tok.subtle));
                if ui
                    .link(
                        egui::RichText::new(t!("about-donate"))
                            .size(12.5)
                            .color(tok.ink),
                    )
                    .clicked()
                {
                    let _ = open::that(url);
                }
            }
        });

        // ── Atualizações ────────────────────────────────────────────────────
        ui.add_space(10.0);
        sec(ui, t!("about-updates-title"), tok.subtle);
        self.update_section(ui, tok);

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(t!("app-license"))
                .size(11.0)
                .color(tok.subtle),
        );
    }

    /// Verificação manual de atualização (BF-606): nada vai à rede sem clique.
    fn update_section(&mut self, ui: &mut egui::Ui, tok: &Tokens) {
        use crate::updater::{self, UpdateState};
        let ctx = ui.ctx().clone();
        self.update.poll();
        let muted = |text: String| egui::RichText::new(text).size(12.0).color(tok.muted);
        let mut action: Option<UpdateAction> = None;

        match &self.update {
            UpdateState::Idle => {
                if ui.button(t!("update-check")).clicked() {
                    action = Some(UpdateAction::Check);
                }
            }
            UpdateState::Checking(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(muted(t!("update-checking")));
                });
            }
            UpdateState::UpToDate => {
                ui.horizontal(|ui| {
                    ui.label(muted(t!("update-up-to-date")));
                    if ui.small_button(t!("update-check-again")).clicked() {
                        action = Some(UpdateAction::Check);
                    }
                });
            }
            UpdateState::Available(release) => {
                ui.label(
                    egui::RichText::new(t!("update-available", version => release.tag_name))
                        .size(13.0)
                        .strong()
                        .color(tok.text),
                );
                ui.horizontal(|ui| {
                    if updater::is_installed_copy()
                        && release.setup_asset().is_some()
                        && ui.button(t!("update-download-install")).clicked()
                    {
                        action = Some(UpdateAction::Download(release.clone()));
                    }
                    if ui.link(t!("update-open-page")).clicked() {
                        let _ = open::that(&release.html_url);
                    }
                });
            }
            UpdateState::Downloading(release, _) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(muted(t!("update-downloading", version => release.tag_name)));
                });
            }
            UpdateState::Ready(path) => {
                ui.label(muted(t!("update-verified")));
                if ui.button(t!("update-install-now")).clicked() {
                    action = Some(UpdateAction::Install(path.clone()));
                }
            }
            UpdateState::Failed(message) => {
                ui.label(egui::RichText::new(message).size(12.0).color(tok.danger));
                ui.horizontal(|ui| {
                    if ui.small_button(t!("update-check-again")).clicked() {
                        action = Some(UpdateAction::Check);
                    }
                    if ui.link(t!("update-open-page")).clicked() {
                        let _ = open::that(updater::RELEASES_PAGE);
                    }
                });
            }
        }

        match action {
            Some(UpdateAction::Check) => self.update.start_check(&ctx),
            Some(UpdateAction::Download(release)) => self.update.start_download(release, &ctx),
            Some(UpdateAction::Install(path)) => match updater::launch_installer(&path) {
                // O instalador fecha a cópia em execução; sair libera o .exe.
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => self.update = UpdateState::Failed(error.to_string()),
            },
            None => {}
        }
    }
}

enum UpdateAction {
    Check,
    Download(crate::updater::Release),
    Install(std::path::PathBuf),
}
