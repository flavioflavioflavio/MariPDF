// settings_types.rs — Enums de configuração do MariPDF
// Extraído de settings.rs para respeitar o hardcap de 250L.

use crate::app_color_mode::ColorMode;
use crate::t;
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
    pub fn label(&self) -> String {
        t!(match self {
            Self::Normal => "scale-normal",
            Self::Largo => "scale-large",
            Self::Xl => "scale-xl",
            Self::Xxl => "scale-xxl",
            Self::Enorme => "scale-huge",
            Self::Gigante => "scale-giant",
            Self::Maximo => "scale-max",
        })
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
    pub fn label(&self) -> String {
        t!(match self {
            Self::Fast => "quality-fast",
            Self::Balanced => "quality-balanced",
            Self::Sharp => "quality-sharp",
            Self::Ultra => "quality-ultra",
        })
    }
    /// Pixels de bitmap por pixel físico da tela.
    pub fn scale(&self) -> f32 {
        match self {
            Self::Fast => 0.75,
            Self::Balanced => 1.0,
            Self::Sharp => 1.5,
            Self::Ultra => 2.0,
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
    pub fn label(&self) -> String {
        t!(match self {
            Self::None => "margin-none",
            Self::Slim => "margin-slim",
            Self::Normal => "margin-normal",
            Self::Wide => "margin-wide",
        })
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
    pub fn label(&self) -> String {
        t!(match self {
            Self::None => "panel-none",
            Self::Thumbnails => "panel-thumbnails",
            Self::Toc => "panel-toc",
        })
    }
    pub fn all() -> [Self; 3] {
        [Self::None, Self::Thumbnails, Self::Toc]
    }
}

// ── Tema ───────────────────────────────────────────────────────────────────────

/// Preferência persistida de tema. `System` segue o tema do Windows no startup.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
    Sepia,
}

impl ThemePreference {
    pub fn resolve(self, system_prefers_dark: bool) -> ColorMode {
        match self {
            Self::System if system_prefers_dark => ColorMode::Dark,
            Self::System => ColorMode::Light,
            Self::Dark => ColorMode::Dark,
            Self::Light => ColorMode::Light,
            Self::Sepia => ColorMode::Sepia,
        }
    }

    /// Preferência a persistir depois que o modo ativo mudou por qualquer
    /// controle da UI. Mantém `System` enquanto o modo ainda segue o sistema.
    pub fn after_mode_change(self, mode: ColorMode, system_prefers_dark: bool) -> Self {
        if self.resolve(system_prefers_dark) == mode {
            self
        } else {
            mode.into()
        }
    }

    pub fn label_key(&self) -> &'static str {
        match self {
            Self::System => "theme-system",
            Self::Dark => "theme-dark",
            Self::Light => "theme-light",
            Self::Sepia => "theme-sepia",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::System, Self::Dark, Self::Light, Self::Sepia]
    }
}

impl From<ColorMode> for ThemePreference {
    fn from(mode: ColorMode) -> Self {
        match mode {
            ColorMode::Dark => Self::Dark,
            ColorMode::Light => Self::Light,
            ColorMode::Sepia => Self::Sepia,
        }
    }
}

// ── Paleta de cores ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct ColorPalette(pub u8);

impl ColorPalette {
    pub fn dark_name(i: u8) -> String {
        palette_name("dark", i)
    }
    pub fn light_name(i: u8) -> String {
        palette_name("light", i)
    }
    pub fn sepia_name(i: u8) -> String {
        palette_name("sepia", i)
    }
}

fn palette_name(mode: &str, i: u8) -> String {
    t!(&format!("palette-{mode}-{}", i.min(3)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `t!` devolve a própria chave quando ela falta no bundle; nenhum rótulo
    /// montado dinamicamente pode cair nesse fallback.
    #[test]
    fn dynamic_labels_resolve_in_both_languages() {
        const KEY_PREFIXES: [&str; 6] = [
            "scale-", "quality-", "margin-", "panel-", "palette-", "theme-",
        ];
        let _guard = crate::i18n::TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        for language in crate::i18n::Language::all() {
            crate::i18n::init(language);
            let mut labels: Vec<String> = Vec::new();
            labels.extend(UiScale::all().iter().map(UiScale::label));
            labels.extend(RenderQuality::all().iter().map(RenderQuality::label));
            labels.extend(PageMargin::all().iter().map(PageMargin::label));
            labels.extend(DefaultPanel::all().iter().map(DefaultPanel::label));
            labels.extend(
                ThemePreference::all()
                    .iter()
                    .map(|t| crate::t!(t.label_key())),
            );
            for i in 0..4 {
                labels.push(ColorPalette::dark_name(i));
                labels.push(ColorPalette::light_name(i));
                labels.push(ColorPalette::sepia_name(i));
            }
            let unresolved: Vec<_> = labels
                .iter()
                .filter(|label| KEY_PREFIXES.iter().any(|p| label.starts_with(p)))
                .collect();
            assert!(unresolved.is_empty(), "{language:?}: {unresolved:?}");
        }
        crate::i18n::init(crate::i18n::Language::default());
    }

    #[test]
    fn render_quality_scales_are_ordered() {
        let scales: Vec<f32> = RenderQuality::all()
            .into_iter()
            .map(|quality| quality.scale())
            .collect();

        assert_eq!(scales, vec![0.75, 1.0, 1.5, 2.0]);
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

    #[test]
    fn theme_system_follows_windows_preference() {
        assert_eq!(ThemePreference::default(), ThemePreference::System);
        assert_eq!(ThemePreference::System.resolve(true), ColorMode::Dark);
        assert_eq!(ThemePreference::System.resolve(false), ColorMode::Light);
        assert_eq!(ThemePreference::Sepia.resolve(true), ColorMode::Sepia);
    }

    #[test]
    fn theme_change_persists_explicit_choice_only_when_needed() {
        let system = ThemePreference::System;
        assert_eq!(
            system.after_mode_change(ColorMode::Dark, true),
            ThemePreference::System
        );
        assert_eq!(
            system.after_mode_change(ColorMode::Sepia, true),
            ThemePreference::Sepia
        );
        assert_eq!(
            ThemePreference::Light.after_mode_change(ColorMode::Dark, false),
            ThemePreference::Dark
        );
    }

    #[test]
    fn theme_preference_round_trips_through_toml() {
        #[derive(Serialize, Deserialize)]
        struct Doc {
            theme: ThemePreference,
        }
        let text = toml::to_string(&Doc {
            theme: ThemePreference::Sepia,
        })
        .unwrap();
        let back: Doc = toml::from_str(&text).unwrap();
        assert_eq!(back.theme, ThemePreference::Sepia);
    }
}
