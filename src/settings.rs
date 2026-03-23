// settings.rs — Configurações persistidas do MariPDF
// Enums de opção em settings_types.rs; aqui ficam AppSettings + I/O

pub use crate::settings_types::{
    UiScale, FontChoice, RenderQuality, PageMargin, DefaultPanel, ColorPalette,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("MariPDF");
    std::fs::create_dir_all(&p).ok();
    p.push("settings.toml");
    p
}

// ── Configurações gerais ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)] pub scale:           UiScale,
    #[serde(default)] pub font:            FontChoice,
    #[serde(default)] pub render_quality:  RenderQuality,
    #[serde(default)] pub page_margin:     PageMargin,
    #[serde(default)] pub smooth_scroll:   bool,
    #[serde(default)] pub track_history:   bool,
    #[serde(default)] pub recent_files:    Vec<PathBuf>,
    #[serde(default)] pub default_panel:   DefaultPanel,
    #[serde(default)] pub palette:         ColorPalette,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            scale:          UiScale::default(),
            font:           FontChoice::default(),
            render_quality: RenderQuality::default(),
            page_margin:    PageMargin::default(),
            smooth_scroll:  true,
            track_history:  true,
            recent_files:   Vec::new(),
            default_panel:  DefaultPanel::default(),
            palette:        ColorPalette::default(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = settings_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            toml::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            std::fs::write(path, toml_str).ok();
        }
    }
}
