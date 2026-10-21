// reader_profile.rs — Página do PDF em cada tema
//
// Cada tema (escuro, claro, sépia) tem o próprio conjunto de páginas, sempre
// coerente com ele: tema escuro nunca mostra página clara e vice-versa. A
// escolha é guardada por tema; trocar de tema troca a página junto.
//
// A página é desenhada com um filtro no bitmap (inversão, preto e branco) e
// uma tinta multiplicada na textura.

use crate::app_color_mode::ColorMode;
use crate::t;
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Páginas do tema escuro: todas invertem (fundo escuro, texto claro).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum DarkPage {
    /// Grafite com texto off-white (o padrão de sempre).
    #[default]
    Night,
    /// Grafite com texto creme: menos luz azul à noite.
    WarmNight,
    /// Quase preto, para telas OLED e ambientes escuros.
    Deep,
}

/// Páginas do tema claro.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum LightPage {
    /// O PDF como foi feito (com o toque da paleta).
    #[default]
    Original,
    /// Papel off-white de livro novo.
    NewBook,
    /// Cinza suave: menos brilho em telas fortes.
    SoftGray,
    /// Preto e branco com contraste: texto preto puro, papel branco.
    Mono,
}

/// Páginas do tema sépia: todas quentes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum SepiaPage {
    #[default]
    Sepia,
    /// Papel envelhecido, mais amarelado.
    Parchment,
    /// Sépia leve, quase creme.
    SoftSepia,
}

/// Página escolhida para cada tema.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct PageStyles {
    #[serde(default)]
    pub dark: DarkPage,
    #[serde(default)]
    pub light: LightPage,
    #[serde(default)]
    pub sepia: SepiaPage,
}

/// Uma opção de página do tema atual, para o seletor nas Configurações.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageChoice {
    Dark(DarkPage),
    Light(LightPage),
    Sepia(SepiaPage),
}

impl PageChoice {
    /// Opções do tema, na ordem do seletor.
    pub fn options(mode: ColorMode) -> Vec<Self> {
        match mode {
            ColorMode::Dark => [DarkPage::Night, DarkPage::WarmNight, DarkPage::Deep]
                .map(Self::Dark)
                .to_vec(),
            ColorMode::Light => [
                LightPage::Original,
                LightPage::NewBook,
                LightPage::SoftGray,
                LightPage::Mono,
            ]
            .map(Self::Light)
            .to_vec(),
            ColorMode::Sepia => [SepiaPage::Sepia, SepiaPage::Parchment, SepiaPage::SoftSepia]
                .map(Self::Sepia)
                .to_vec(),
        }
    }

    pub fn current(styles: PageStyles, mode: ColorMode) -> Self {
        match mode {
            ColorMode::Dark => Self::Dark(styles.dark),
            ColorMode::Light => Self::Light(styles.light),
            ColorMode::Sepia => Self::Sepia(styles.sepia),
        }
    }

    pub fn apply(self, styles: &mut PageStyles) {
        match self {
            Self::Dark(page) => styles.dark = page,
            Self::Light(page) => styles.light = page,
            Self::Sepia(page) => styles.sepia = page,
        }
    }

    pub fn label(self) -> String {
        t!(match self {
            Self::Dark(DarkPage::Night) => "page-night",
            Self::Dark(DarkPage::WarmNight) => "page-warm-night",
            Self::Dark(DarkPage::Deep) => "page-deep",
            Self::Light(LightPage::Original) => "page-original",
            Self::Light(LightPage::NewBook) => "page-new-book",
            Self::Light(LightPage::SoftGray) => "page-soft-gray",
            Self::Light(LightPage::Mono) => "page-mono",
            Self::Sepia(SepiaPage::Sepia) => "page-sepia",
            Self::Sepia(SepiaPage::Parchment) => "page-parchment",
            Self::Sepia(SepiaPage::SoftSepia) => "page-soft-sepia",
        })
    }

    pub fn look(self, mode: ColorMode, palette: u8) -> PageLook {
        let rgb = egui::Color32::from_rgb;
        let palette_tint = mode.page_tint_for(palette);
        match self {
            Self::Dark(DarkPage::Night) => PageLook {
                tint: palette_tint,
                invert: Invert::Soft,
                mono: false,
            },
            Self::Dark(DarkPage::WarmNight) => PageLook {
                tint: rgb(236, 218, 190),
                invert: Invert::Soft,
                mono: false,
            },
            Self::Dark(DarkPage::Deep) => PageLook {
                tint: rgb(226, 226, 230),
                invert: Invert::Deep,
                mono: false,
            },
            Self::Light(LightPage::Original) => PageLook {
                tint: palette_tint,
                ..PageLook::PLAIN
            },
            Self::Light(LightPage::NewBook) => PageLook {
                tint: rgb(251, 246, 232),
                ..PageLook::PLAIN
            },
            Self::Light(LightPage::SoftGray) => PageLook {
                tint: rgb(224, 224, 220),
                ..PageLook::PLAIN
            },
            Self::Light(LightPage::Mono) => PageLook {
                mono: true,
                ..PageLook::PLAIN
            },
            Self::Sepia(SepiaPage::Sepia) => PageLook {
                tint: palette_tint,
                ..PageLook::PLAIN
            },
            Self::Sepia(SepiaPage::Parchment) => PageLook {
                tint: rgb(238, 214, 168),
                ..PageLook::PLAIN
            },
            Self::Sepia(SepiaPage::SoftSepia) => PageLook {
                tint: rgb(250, 238, 214),
                ..PageLook::PLAIN
            },
        }
    }
}

/// Como inverter as cores da página.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Invert {
    None,
    /// Branco → grafite (38), preto → off-white (243).
    Soft,
    /// Branco → quase preto (12), preto → cinza claro (226).
    Deep,
}

/// Como a página é desenhada: filtro no bitmap + tinta na textura.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageLook {
    pub tint: egui::Color32,
    pub invert: Invert,
    pub mono: bool,
}

impl PageLook {
    pub const PLAIN: Self = Self {
        tint: egui::Color32::WHITE,
        invert: Invert::None,
        mono: false,
    };

    /// true se o bitmap precisa passar por `apply_filter` antes do upload.
    pub fn filters_pixels(&self) -> bool {
        self.invert != Invert::None || self.mono
    }

    /// Sufixo estável para nomes de textura (miniaturas).
    pub fn key(&self) -> &'static str {
        match (self.invert, self.mono) {
            (Invert::None, false) => "",
            (Invert::Soft, false) => "_inv",
            (Invert::Deep, false) => "_deep",
            (_, true) => "_mono",
        }
    }

    /// Cores de papel e texto resultantes (miniaturas do seletor).
    pub fn preview(&self) -> (egui::Color32, egui::Color32) {
        let shade = |value: u8| {
            let mut px = [value, value, value, 255];
            apply_filter(&mut px, *self);
            let tint = self.tint;
            egui::Color32::from_rgb(
                (px[0] as u16 * tint.r() as u16 / 255) as u8,
                (px[1] as u16 * tint.g() as u16 / 255) as u8,
                (px[2] as u16 * tint.b() as u16 / 255) as u8,
            )
        };
        (shade(255), shade(30))
    }
}

/// Página do tema atual.
pub fn page_look(styles: PageStyles, mode: ColorMode, palette: u8) -> PageLook {
    PageChoice::current(styles, mode).look(mode, palette)
}

/// Níveis do preto e branco: abaixo de LO vira preto, acima de HI vira branco.
const MONO_LO: f32 = 0.18;
const MONO_HI: f32 = 0.86;

/// Aplica preto e branco e/ou inversão no RGBA (alfa preservado).
///
/// Roda na thread da UI a cada upload de página: tabelas de 256 entradas
/// mantêm o laço por pixel sem ponto flutuante nem desvios.
pub fn apply_filter(rgba: &mut [u8], look: PageLook) {
    if !look.filters_pixels() {
        return;
    }
    let (dark_end, light_end) = match look.invert {
        Invert::None => (0u16, 255u16),
        Invert::Soft => (38, 243),
        Invert::Deep => (12, 226),
    };
    let map: [u8; 256] = std::array::from_fn(|v| match look.invert {
        Invert::None => v as u8,
        _ => (dark_end + (255 - v as u16) * (light_end - dark_end) / 255) as u8,
    });
    let pixels = rgba.as_chunks_mut::<4>().0;
    if look.mono {
        // Níveis do preto e branco aplicados sobre a luminância (0–255).
        let lo = (MONO_LO * 255.0) as i32;
        let hi = (MONO_HI * 255.0) as i32;
        let level: [u8; 256] = std::array::from_fn(|l| {
            let v = ((l as i32 - lo) * 255 / (hi - lo)).clamp(0, 255) as usize;
            map[v]
        });
        for px in pixels {
            // Luminância Rec. 709 em inteiros (pesos somam 256).
            let luma = (54 * px[0] as u32 + 183 * px[1] as u32 + 19 * px[2] as u32) >> 8;
            let value = level[luma as usize];
            px[0] = value;
            px[1] = value;
            px[2] = value;
        }
    } else {
        for px in pixels {
            px[0] = map[px[0] as usize];
            px[1] = map[px[1] as usize];
            px[2] = map[px[2] as usize];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_offers_only_pages_that_match_it() {
        for mode in [ColorMode::Dark, ColorMode::Light, ColorMode::Sepia] {
            for choice in PageChoice::options(mode) {
                let (paper, text) = choice.look(mode, 0).preview();
                let lum = |c: egui::Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
                if mode == ColorMode::Dark {
                    assert!(
                        lum(paper) < lum(text),
                        "{choice:?}: escuro precisa de papel escuro"
                    );
                    assert!(lum(paper) < 3 * 80, "{choice:?}: papel escuro de verdade");
                } else {
                    assert!(
                        lum(paper) > lum(text),
                        "{choice:?}: claro precisa de papel claro"
                    );
                    assert!(lum(paper) > 3 * 180, "{choice:?}: papel claro de verdade");
                }
            }
        }
    }

    #[test]
    fn choice_is_kept_per_theme() {
        let mut styles = PageStyles::default();
        PageChoice::Light(LightPage::Mono).apply(&mut styles);
        PageChoice::Dark(DarkPage::Deep).apply(&mut styles);
        assert_eq!(
            PageChoice::current(styles, ColorMode::Light),
            PageChoice::Light(LightPage::Mono)
        );
        assert_eq!(
            PageChoice::current(styles, ColorMode::Dark),
            PageChoice::Dark(DarkPage::Deep)
        );
        assert_eq!(
            PageChoice::current(styles, ColorMode::Sepia),
            PageChoice::Sepia(SepiaPage::Sepia)
        );
    }

    #[test]
    fn mono_makes_text_black_and_paper_white() {
        let mut rgba = vec![40, 40, 40, 255, 250, 245, 230, 255, 200, 30, 30, 128];
        apply_filter(
            &mut rgba,
            PageLook {
                mono: true,
                ..PageLook::PLAIN
            },
        );
        assert_eq!(&rgba[0..3], &[0, 0, 0]);
        assert_eq!(&rgba[4..7], &[255, 255, 255]);
        assert_eq!(rgba[8], rgba[9]);
        assert_eq!(rgba[11], 128, "alfa preservado");
    }

    #[test]
    fn inversions_keep_their_ends() {
        for (invert, white_to, black_to) in [(Invert::Soft, 38, 243), (Invert::Deep, 12, 226)] {
            let mut rgba = vec![255, 255, 255, 255, 0, 0, 0, 255];
            apply_filter(
                &mut rgba,
                PageLook {
                    invert,
                    ..PageLook::PLAIN
                },
            );
            assert_eq!(&rgba[0..3], &[white_to; 3]);
            assert_eq!(&rgba[4..7], &[black_to; 3]);
        }
    }
}
