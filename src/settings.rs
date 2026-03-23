// settings.rs — Configurações persistidas do MariPDF

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("MariPDF");
    std::fs::create_dir_all(&p).ok();
    p.push("settings.toml");
    p
}

// ── Escala de UI ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum UiScale {
    Normal,  // 1.0x — padrão (mínimo)
    Largo,   // 1.2x — confortável
    Xl,      // 1.4x — grande
    Xxl,     // 1.6x — extra grande
    Enorme,  // 1.8x — enorme
    Gigante, // 2.0x — gigante
    Maximo,  // 2.4x — máximo
}

impl UiScale {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal  => "Normal",
            Self::Largo   => "Largo",
            Self::Xl      => "XL",
            Self::Xxl     => "XXL",
            Self::Enorme  => "Enorme",
            Self::Gigante => "Gigante",
            Self::Maximo  => "Máximo",
        }
    }
    pub fn pixels_per_point(&self) -> f32 {
        match self {
            Self::Normal  => 1.00,
            Self::Largo   => 1.20,
            Self::Xl      => 1.40,
            Self::Xxl     => 1.60,
            Self::Enorme  => 1.80,
            Self::Gigante => 2.00,
            Self::Maximo  => 2.40,
        }
    }
    pub fn all() -> [Self; 7] {
        [Self::Normal, Self::Largo, Self::Xl, Self::Xxl, Self::Enorme, Self::Gigante, Self::Maximo]
    }
}

impl Default for UiScale {
    fn default() -> Self { Self::Normal }
}

// ── Fonte de texto ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FontChoice {
    Outfit,     // Fonte oficial do MariPDF — geométrica moderna
    Inter,      // Inter — tipografia humanista extremamente legível
    Lora,       // Lora — serifada elegante para leitura longa
    Quicksand,  // Quicksand — geométrica suave e simpática
    RobotoMono, // Roboto Mono — monospace compacta para texto técnico
}

impl FontChoice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Outfit     => "Outfit  ✦",
            Self::Inter      => "Inter",
            Self::Lora       => "Lora",
            Self::Quicksand  => "Quicksand",
            Self::RobotoMono => "Roboto Mono",
        }
    }
    /// Identificador da familia de fonte no egui FontData
    pub fn family_key(&self) -> &'static str {
        match self {
            Self::Outfit     => "outfit",
            Self::Inter      => "inter",
            Self::Lora       => "lora",
            Self::Quicksand  => "quicksand",
            Self::RobotoMono => "robotomono",
        }
    }
    pub fn all() -> [Self; 5] {
        [Self::Outfit, Self::Inter, Self::Lora, Self::Quicksand, Self::RobotoMono]
    }
}

impl Default for FontChoice {
    fn default() -> Self { Self::Outfit }
}

// ── Qualidade de render ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RenderQuality {
    Fast,      // 1.0x — renderização rápida, boa para documentos grandes
    Balanced,  // 1.5x — equilíbrio entre nitidez e performance
    Sharp,     // 2.0x — alta nitidez, ideal para HiDPI
    Ultra,     // 3.0x — máxima nitidez, pode ser lento
}

impl RenderQuality {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fast     => "Rápida",
            Self::Balanced => "Balanceada",
            Self::Sharp    => "Nítida",
            Self::Ultra    => "Ultra",
        }
    }
    pub fn scale(&self) -> f32 {
        match self {
            Self::Fast     => 1.0,
            Self::Balanced => 1.5,
            Self::Sharp    => 2.0,
            Self::Ultra    => 3.0,
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::Fast, Self::Balanced, Self::Sharp, Self::Ultra]
    }
    pub fn scale_factor(&self) -> f32 { self.scale() }
}

impl Default for RenderQuality {
    fn default() -> Self { Self::Balanced }
}

// ── Margens de página ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PageMargin {
    None,
    Slim,
    Normal,
    Wide,
}

impl PageMargin {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None   => "Nenhuma",
            Self::Slim   => "Estreita",
            Self::Normal => "Normal",
            Self::Wide   => "Larga",
        }
    }
    pub fn px(&self) -> f32 {
        match self {
            Self::None   => 0.0,
            Self::Slim   => 8.0,
            Self::Normal => 20.0,
            Self::Wide   => 40.0,
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::None, Self::Slim, Self::Normal, Self::Wide]
    }
}

impl Default for PageMargin {
    fn default() -> Self { Self::Normal }
}

// ── Painel padrão ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DefaultPanel { None, Thumbnails, Toc }

impl DefaultPanel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None       => "Nenhum",
            Self::Thumbnails => "Miniaturas",
            Self::Toc        => "Sumário",
        }
    }
    pub fn all() -> [Self; 3] {
        [Self::None, Self::Thumbnails, Self::Toc]
    }
}

impl Default for DefaultPanel {
    fn default() -> Self { Self::None }
}

// ── Paleta de cores ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ColorPalette(pub u8);

impl Default for ColorPalette {
    fn default() -> Self { Self(0) }
}

impl ColorPalette {
    pub fn dark_name(i: u8) -> &'static str {
        match i {
            0 => "Carvão",
            1 => "Meia-noite",
            2 => "Violeta",
            _ => "Floresta",
        }
    }
    pub fn light_name(i: u8) -> &'static str {
        match i {
            0 => "Névoa",
            1 => "Ártico",
            2 => "Papel",
            _ => "Menta",
        }
    }
    pub fn sepia_name(i: u8) -> &'static str {
        match i {
            0 => "Pergaminho",
            1 => "Âmbar",
            2 => "Vintage",
            _ => "Rosa Antigo",
        }
    }
}

// ── Configurações gerais ──────────────────────────────────────────────────────

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
