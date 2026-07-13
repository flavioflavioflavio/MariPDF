// settings_types.rs — Enums de configuração do MariPDF
// Extraído de settings.rs para respeitar o hardcap de 250L.

use serde::{Deserialize, Serialize};

// ── Escala de UI ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum UiScale {
    #[default]
    Normal, // 1.0x
    Largo,   // 1.2x
    Xl,      // 1.4x
    Xxl,     // 1.6x
    Enorme,  // 1.8x
    Gigante, // 2.0x
    Maximo,  // 2.4x
}

impl UiScale {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Largo => "Largo",
            Self::Xl => "XL",
            Self::Xxl => "XXL",
            Self::Enorme => "Enorme",
            Self::Gigante => "Gigante",
            Self::Maximo => "Máximo",
        }
    }
    pub fn pixels_per_point(&self) -> f32 {
        match self {
            Self::Normal => 1.00,
            Self::Largo => 1.20,
            Self::Xl => 1.40,
            Self::Xxl => 1.60,
            Self::Enorme => 1.80,
            Self::Gigante => 2.00,
            Self::Maximo => 2.40,
        }
    }
    pub fn all() -> [Self; 7] {
        [
            Self::Normal,
            Self::Largo,
            Self::Xl,
            Self::Xxl,
            Self::Enorme,
            Self::Gigante,
            Self::Maximo,
        ]
    }
}

// ── Fonte de texto ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum FontChoice {
    #[default]
    Outfit,
    Inter,
    Lora,
    Quicksand,
    RobotoMono,
}

impl FontChoice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Outfit => "Outfit  ✦",
            Self::Inter => "Inter",
            Self::Lora => "Lora",
            Self::Quicksand => "Quicksand",
            Self::RobotoMono => "Roboto Mono",
        }
    }
    pub fn family_key(&self) -> &'static str {
        match self {
            Self::Outfit => "outfit",
            Self::Inter => "inter",
            Self::Lora => "lora",
            Self::Quicksand => "quicksand",
            Self::RobotoMono => "robotomono",
        }
    }
    pub fn all() -> [Self; 5] {
        [
            Self::Outfit,
            Self::Inter,
            Self::Lora,
            Self::Quicksand,
            Self::RobotoMono,
        ]
    }
}

// ── Qualidade de render ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum RenderQuality {
    Fast,
    #[default]
    Balanced,
    Sharp,
    Ultra,
}

impl RenderQuality {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fast => "Rápida",
            Self::Balanced => "Balanceada",
            Self::Sharp => "Nítida",
            Self::Ultra => "Ultra",
        }
    }
    pub fn scale(&self) -> f32 {
        match self {
            Self::Fast => 1.0,
            Self::Balanced => 1.5,
            Self::Sharp => 2.0,
            Self::Ultra => 3.0,
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::Fast, Self::Balanced, Self::Sharp, Self::Ultra]
    }
}

// ── Margens de página ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum PageMargin {
    None,
    Slim,
    #[default]
    Normal,
    Wide,
}

impl PageMargin {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Nenhuma",
            Self::Slim => "Estreita",
            Self::Normal => "Normal",
            Self::Wide => "Larga",
        }
    }
    pub fn px(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Slim => 8.0,
            Self::Normal => 20.0,
            Self::Wide => 40.0,
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::None, Self::Slim, Self::Normal, Self::Wide]
    }
}

// ── Painel padrão ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum DefaultPanel {
    #[default]
    None,
    Thumbnails,
    Toc,
}

impl DefaultPanel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Nenhum",
            Self::Thumbnails => "Miniaturas",
            Self::Toc => "Sumário",
        }
    }
    pub fn all() -> [Self; 3] {
        [Self::None, Self::Thumbnails, Self::Toc]
    }
}

// ── Paleta de cores ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct ColorPalette(pub u8);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_quality_scales_are_ordered() {
        let scales: Vec<f32> = RenderQuality::all()
            .into_iter()
            .map(|quality| quality.scale())
            .collect();

        assert_eq!(scales, vec![1.0, 1.5, 2.0, 3.0]);
        assert!(scales.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn page_margins_match_the_ui_contract() {
        assert_eq!(PageMargin::None.px(), 0.0);
        assert_eq!(PageMargin::Slim.px(), 8.0);
        assert_eq!(PageMargin::Normal.px(), 20.0);
        assert_eq!(PageMargin::Wide.px(), 40.0);
    }

    #[test]
    fn ui_scale_default_is_one_to_one() {
        assert_eq!(UiScale::default(), UiScale::Normal);
        assert_eq!(UiScale::default().pixels_per_point(), 1.0);
    }
}
