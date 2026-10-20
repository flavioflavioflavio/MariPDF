// reader_profile.rs — Perfis de cor da página
//
// Decide como o bitmap do PDF aparece na tela, independente do tema da
// interface: tinta multiplicada na textura, inversão (leitura noturna) e
// preto e branco com contraste. `Auto` mantém o comportamento original:
// segue o tema e a paleta.

use crate::app_color_mode::ColorMode;
use crate::t;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum ReaderProfile {
    /// Segue o tema: escuro inverte a página, sépia aquece, claro suaviza.
    #[default]
    Auto,
    /// O PDF exatamente como foi feito.
    Original,
    /// Papel off-white de livro novo.
    NewBook,
    /// Papel envelhecido, quente.
    Sepia,
    /// Cinza suave: menos brilho em telas fortes.
    SoftGray,
    /// Preto e branco com contraste: texto preto puro, papel branco.
    Mono,
    /// Noturno: página escura com texto claro levemente quente.
    Night,
}

impl ReaderProfile {
    pub fn all() -> [Self; 7] {
        [
            Self::Auto,
            Self::Original,
            Self::NewBook,
            Self::Sepia,
            Self::SoftGray,
            Self::Mono,
            Self::Night,
        ]
    }

    pub fn label(&self) -> String {
        t!(match self {
            Self::Auto => "profile-auto",
            Self::Original => "profile-original",
            Self::NewBook => "profile-new-book",
            Self::Sepia => "profile-sepia",
            Self::SoftGray => "profile-soft-gray",
            Self::Mono => "profile-mono",
            Self::Night => "profile-night",
        })
    }
}

/// Como a página é desenhada: filtro no bitmap + tinta na textura.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageLook {
    pub tint: egui::Color32,
    pub invert: bool,
    pub mono: bool,
}

impl PageLook {
    pub const PLAIN: Self = Self {
        tint: egui::Color32::WHITE,
        invert: false,
        mono: false,
    };

    /// true se o bitmap precisa passar por `apply_filter` antes do upload.
    pub fn filters_pixels(&self) -> bool {
        self.invert || self.mono
    }

    /// Sufixo estável para nomes de textura (miniaturas).
    pub fn key(&self) -> &'static str {
        match (self.invert, self.mono) {
            (false, false) => "",
            (true, false) => "_inv",
            (false, true) => "_mono",
            (true, true) => "_mono_inv",
        }
    }
}

pub fn page_look(profile: ReaderProfile, mode: ColorMode, palette: u8) -> PageLook {
    let tint = |r, g, b| egui::Color32::from_rgb(r, g, b);
    match profile {
        ReaderProfile::Auto => PageLook {
            tint: mode.page_tint_for(palette),
            invert: matches!(mode, ColorMode::Dark),
            mono: false,
        },
        ReaderProfile::Original => PageLook::PLAIN,
        ReaderProfile::NewBook => PageLook {
            tint: tint(251, 246, 232),
            ..PageLook::PLAIN
        },
        ReaderProfile::Sepia => PageLook {
            tint: tint(240, 222, 185),
            ..PageLook::PLAIN
        },
        ReaderProfile::SoftGray => PageLook {
            tint: tint(222, 222, 218),
            ..PageLook::PLAIN
        },
        ReaderProfile::Mono => PageLook {
            mono: true,
            ..PageLook::PLAIN
        },
        ReaderProfile::Night => PageLook {
            tint: tint(220, 214, 204),
            invert: true,
            mono: false,
        },
    }
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
    // Inversão suave: branco → grafite (38), preto → off-white (243).
    let invert: [u8; 256] = std::array::from_fn(|v| {
        if look.invert {
            38 + ((255 - v as u16) * 205 / 255) as u8
        } else {
            v as u8
        }
    });
    let pixels = rgba.as_chunks_mut::<4>().0;
    if look.mono {
        // Níveis do preto e branco aplicados sobre a luminância (0–255).
        let lo = (MONO_LO * 255.0) as i32;
        let hi = (MONO_HI * 255.0) as i32;
        let level: [u8; 256] = std::array::from_fn(|l| {
            let v = ((l as i32 - lo) * 255 / (hi - lo)).clamp(0, 255) as usize;
            invert[v]
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
            px[0] = invert[px[0] as usize];
            px[1] = invert[px[1] as usize];
            px[2] = invert[px[2] as usize];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_follows_the_theme() {
        assert!(page_look(ReaderProfile::Auto, ColorMode::Dark, 0).invert);
        assert!(!page_look(ReaderProfile::Auto, ColorMode::Light, 0).invert);
        // Perfis explícitos não dependem do tema.
        assert_eq!(
            page_look(ReaderProfile::Original, ColorMode::Dark, 0),
            PageLook::PLAIN
        );
    }

    #[test]
    fn mono_makes_text_black_and_paper_white() {
        // texto cinza-escuro, papel levemente amarelado, cor média
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
    fn night_inverts_softly() {
        let mut rgba = vec![255, 255, 255, 255, 0, 0, 0, 255];
        apply_filter(
            &mut rgba,
            page_look(ReaderProfile::Night, ColorMode::Light, 0),
        );
        assert_eq!(&rgba[0..3], &[38, 38, 38]);
        assert_eq!(&rgba[4..7], &[243, 243, 243]);
    }
}
