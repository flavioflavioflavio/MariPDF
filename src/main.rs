// main.rs — Entry point do MariPDF
// Responsabilidade: inicializar logging, parsear args CLI, lançar UI

use tracing_subscriber::{fmt, EnvFilter};
use crate::settings::FontChoice;
use std::path::PathBuf;

mod app;
mod color_mode;
mod pdf;
mod platform;
mod toast;
mod ui;
mod settings;
mod settings_types;

fn main() -> anyhow::Result<()> {
    // Logging (RUST_LOG=debug para verbose)
    fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("maripdf=info".parse()?))
        .init();

    tracing::info!("MariPDF v{} iniciando", env!("CARGO_PKG_VERSION"));

    // ── Argumento CLI opcional: maripdf.exe <arquivo.pdf> ────────────────
    let cli_file: Option<PathBuf> = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.extension().map(|e| e == "pdf").unwrap_or(false))
        .filter(|p| p.exists());

    if let Some(ref f) = cli_file {
        tracing::info!("Arquivo via CLI: {}", f.display());
    }

    // ── Opções da janela ─────────────────────────────────────────────────
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MariPDF")
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    // ── Detecta tema do Windows: se dark, usa Dark mode, senão Light ──────────
    let initial_color_mode = if platform::theme::system_prefers_dark() {
        app::ColorMode::Dark
    } else {
        app::ColorMode::Light
    };

    eframe::run_native(
        "MariPDF",
        native_options,
        Box::new(move |cc| {
            // ── Registrar fontes ─────────────────────────────────────────────
            let mut fonts = egui::FontDefinitions::default();

            // Outfit Regular → fonte proporcional padrão do app
            fonts.font_data.insert(
                "outfit".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/Outfit-Regular.ttf")
                ).into(),
            );
            // Outfit Black → família "Title" para o logo
            fonts.font_data.insert(
                "outfit_black".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/Outfit-Black.ttf")
                ).into(),
            );
            // Phosphor → família "Icons"
            fonts.font_data.insert(
                "phosphor".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/phosphor.ttf")
                ).into(),
            );

            // Outfit como primeira opção proporcional (mantém fallback egui)
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "outfit".into());

            // Família "Title" com Outfit Black
            fonts.families
                .entry(egui::FontFamily::Name("Title".into()))
                .or_default()
                .insert(0, "outfit_black".into());

            // Família "Icons" com Phosphor
            fonts.families
                .entry(egui::FontFamily::Name("Icons".into()))
                .or_default()
                .insert(0, "phosphor".into());

            // Inter — tipografia humanista legível
            fonts.font_data.insert(
                "inter".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/fonts/Inter-Regular.ttf")
                ).into(),
            );
            // Lora — serifada elegante
            fonts.font_data.insert(
                "lora".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/fonts/Lora-Regular.ttf")
                ).into(),
            );
            // Quicksand — geométrica suave
            fonts.font_data.insert(
                "quicksand".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/fonts/Quicksand-Regular.ttf")
                ).into(),
            );
            // Roboto Mono — monospace compacta e legível
            fonts.font_data.insert(
                "robotomono".into(),
                egui::FontData::from_static(
                    include_bytes!("../assets/fonts/RobotoMono-Regular.ttf")
                ).into(),
            );

            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(app::MariApp::new(cc, cli_file, initial_color_mode)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Erro ao iniciar UI: {e}"))?;

    Ok(())
}

/// Aplica a fonte escolhida em runtime, trocando o topo da família Proportional.
/// Todas as fontes já estão embedadas no binário; só reorganizamos a ordem.
pub fn apply_font_settings(ctx: &egui::Context, font: FontChoice) {
    let mut fonts = egui::FontDefinitions::default();

    // Embute todas as fontes do app
    fonts.font_data.insert("outfit".into(),
        egui::FontData::from_static(include_bytes!("../assets/Outfit-Regular.ttf")).into());
    fonts.font_data.insert("outfit_black".into(),
        egui::FontData::from_static(include_bytes!("../assets/Outfit-Black.ttf")).into());
    fonts.font_data.insert("phosphor".into(),
        egui::FontData::from_static(include_bytes!("../assets/phosphor.ttf")).into());
    fonts.font_data.insert("inter".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into());
    fonts.font_data.insert("lora".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Lora-Regular.ttf")).into());
    fonts.font_data.insert("quicksand".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Quicksand-Regular.ttf")).into());
    fonts.font_data.insert("robotomono".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/RobotoMono-Regular.ttf")).into());

    // Família Title
    fonts.families
        .entry(egui::FontFamily::Name("Title".into()))
        .or_default()
        .insert(0, "outfit_black".into());

    // Família Icons
    fonts.families
        .entry(egui::FontFamily::Name("Icons".into()))
        .or_default()
        .insert(0, "phosphor".into());

    // Família Proportional — inserimos a fonte escolhida no topo, preservando
    // os fallbacks padrão do egui (Ubuntu-Light, etc.) para não crashar.
    let chosen = font.family_key();
    let prop = fonts.families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    // Remove duplicata se já existir na lista, depois insere no topo
    prop.retain(|f| f.as_str() != chosen);
    prop.insert(0, chosen.into());
    // Garante que "outfit" também está como segundo fallback (se não for o escolhido)
    if chosen != "outfit" {
        prop.retain(|f| f.as_str() != "outfit");
        prop.insert(1, "outfit".into());
    }

    ctx.set_fonts(fonts);
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Falha ao carregar ícone (assets/icon.png)")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData { rgba: image.into_raw(), width, height }
}
