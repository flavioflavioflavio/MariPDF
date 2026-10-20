// main.rs — Entry point do MariPDF
// Responsabilidade: inicializar logging, parsear args CLI, lançar UI

use crate::settings::FontChoice;
use std::path::PathBuf;

mod app;
mod app_about;
mod app_color_mode;
mod app_constants;
mod app_frame;
mod app_home;
mod app_home_btns;
mod app_nav;
mod app_overlays;
mod app_reader;
mod app_text_sel;
mod app_toast;
mod app_toolbar;
mod app_tools;
mod app_update;
mod app_window;
mod bench;
mod byte_cache;
mod i18n;
mod logger;
mod notices;
mod pdf;
mod platform;
mod reader_profile;
mod settings;
mod settings_types;
mod showcase;
mod soak;
mod storage;
mod text_selection;
mod ui;
mod updater;

fn main() -> anyhow::Result<()> {
    bench::mark_process_start();
    // Logging: stdout + arquivo em %APPDATA%\maripdf\maripdf.log
    // Manter _log_guard vivo até o fim do processo para flush do buffer
    let _log_guard = logger::init();

    // i18n: carrega idioma salvo ou pt-BR padrão
    // Idioma padrão primeiro: se settings.toml estiver corrompido, o aviso
    // de recuperação já sai traduzido em vez de mostrar a chave crua.
    crate::i18n::init(crate::i18n::Language::default());
    let lang = crate::settings::AppSettings::load().language;
    crate::i18n::init(lang);

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
            // Tamanho inicial; a posição salva é aplicada no primeiro frame,
            // antes de a janela aparecer (app_window.rs).
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(load_icon()),
        wgpu_options: wgpu_configuration(),
        ..Default::default()
    };

    // ── Tema do Windows: base da preferência "Sistema" ────────────────────────
    let system_prefers_dark = platform::theme::system_prefers_dark();

    eframe::run_native(
        "MariPDF",
        native_options,
        Box::new(move |cc| {
            // ── Registrar fontes ─────────────────────────────────────────────
            let mut fonts = egui::FontDefinitions::default();

            // Outfit Regular → fonte proporcional padrão do app
            fonts.font_data.insert(
                "outfit".into(),
                egui::FontData::from_static(include_bytes!("../assets/Outfit-Regular.ttf")).into(),
            );
            // Outfit Black → família "Title" para o logo
            fonts.font_data.insert(
                "outfit_black".into(),
                egui::FontData::from_static(include_bytes!("../assets/Outfit-Black.ttf")).into(),
            );
            // Phosphor → família "Icons"
            fonts.font_data.insert(
                "phosphor".into(),
                egui::FontData::from_static(include_bytes!("../assets/phosphor.ttf")).into(),
            );

            // Outfit como primeira opção proporcional (mantém fallback egui)
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "outfit".into());

            // Família "Title" com Outfit Black
            fonts
                .families
                .entry(egui::FontFamily::Name("Title".into()))
                .or_default()
                .insert(0, "outfit_black".into());

            // Família "Icons" com Phosphor
            fonts
                .families
                .entry(egui::FontFamily::Name("Icons".into()))
                .or_default()
                .insert(0, "phosphor".into());

            // Inter — tipografia humanista legível
            fonts.font_data.insert(
                "inter".into(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf"))
                    .into(),
            );
            // Lora — serifada elegante
            fonts.font_data.insert(
                "lora".into(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Lora-Regular.ttf"))
                    .into(),
            );
            // Quicksand — geométrica suave
            fonts.font_data.insert(
                "quicksand".into(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/Quicksand-Regular.ttf"
                ))
                .into(),
            );
            // Roboto Mono — monospace compacta e legível
            fonts.font_data.insert(
                "robotomono".into(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/RobotoMono-Regular.ttf"
                ))
                .into(),
            );

            cc.egui_ctx.set_fonts(fonts);

            // Preferências visuais devem valer desde o primeiro frame.
            let persisted = crate::settings::AppSettings::load();
            cc.egui_ctx
                .set_pixels_per_point(persisted.scale.pixels_per_point());
            crate::apply_font_settings(&cc.egui_ctx, persisted.font);

            Ok(Box::new(app::MariApp::new(
                cc,
                cli_file,
                system_prefers_dark,
            )))
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
    fonts.font_data.insert(
        "outfit".into(),
        egui::FontData::from_static(include_bytes!("../assets/Outfit-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "outfit_black".into(),
        egui::FontData::from_static(include_bytes!("../assets/Outfit-Black.ttf")).into(),
    );
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(include_bytes!("../assets/phosphor.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "lora".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Lora-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "quicksand".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Quicksand-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "robotomono".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/RobotoMono-Regular.ttf"))
            .into(),
    );

    // Família Title
    fonts
        .families
        .entry(egui::FontFamily::Name("Title".into()))
        .or_default()
        .insert(0, "outfit_black".into());

    // Família Icons
    fonts
        .families
        .entry(egui::FontFamily::Name("Icons".into()))
        .or_default()
        .insert(0, "phosphor".into());

    // Família Proportional — inserimos a fonte escolhida no topo, preservando
    // os fallbacks padrão do egui (Ubuntu-Light, etc.) para não crashar.
    let chosen = font.family_key();
    let prop = fonts
        .families
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

/// DX12 nativo com GL de reserva e GPU de baixo consumo. Medido no corpus
/// (BF-003): o Vulkan padrão custava ~140 MB a mais de working set e ~200 ms
/// a mais até o primeiro frame. `WGPU_BACKEND` / `WGPU_POWER_PREF` ainda
/// sobrescrevem para diagnóstico.
fn wgpu_configuration() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu;

    let mut setup = WgpuSetupCreateNew::default();
    setup.instance_descriptor.backends =
        wgpu::Backends::from_env().unwrap_or(wgpu::Backends::DX12 | wgpu::Backends::GL);
    setup.power_preference =
        wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::LowPower);
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Falha ao carregar ícone (assets/icon.png)")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
