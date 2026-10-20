// settings.rs — Configurações persistidas do MariPDF
// Enums de opção em settings_types.rs; aqui ficam AppSettings + I/O

pub use crate::i18n::Language;
pub use crate::settings_types::{
    ColorPalette, DefaultPanel, FontChoice, PageMargin, RenderQuality, ThemePreference, UiScale,
};

use crate::storage::{self, StoreKind};
use serde::{Deserialize, Serialize};

// ── Configurações gerais ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "storage::schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub scale: UiScale,
    #[serde(default)]
    pub font: FontChoice,
    #[serde(default)]
    pub render_quality: RenderQuality,
    #[serde(default)]
    pub page_margin: PageMargin,
    #[serde(default)]
    pub smooth_scroll: bool,
    #[serde(default)]
    pub track_history: bool,
    #[serde(default)]
    pub default_panel: DefaultPanel,
    #[serde(default)]
    pub palette: ColorPalette,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: ThemePreference,
    /// Página escolhida para cada tema (sempre coerente com ele).
    #[serde(default)]
    pub page_styles: crate::reader_profile::PageStyles,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: storage::SCHEMA_VERSION,
            scale: UiScale::default(),
            font: FontChoice::default(),
            render_quality: RenderQuality::default(),
            page_margin: PageMargin::default(),
            smooth_scroll: true,
            track_history: true,
            default_panel: DefaultPanel::default(),
            palette: ColorPalette::default(),
            language: Language::default(),
            theme: ThemePreference::default(),
            page_styles: crate::reader_profile::PageStyles::default(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        storage::load_toml(&storage::settings_path(), StoreKind::Settings)
    }

    /// Grava atomicamente; falhas viram aviso na UI (storage::save_toml).
    pub fn save(&self) {
        let _ = storage::save_toml(&storage::settings_path(), self, StoreKind::Settings);
    }
}
