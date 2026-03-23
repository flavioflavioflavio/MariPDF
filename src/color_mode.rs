// color_mode.rs — Tokens de cor do design system MariPDF
// Zero valores hardcoded fora deste arquivo — todas as cores passam por aqui.

use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorMode { Dark, Light, Sepia }

impl ColorMode {
    pub fn bg(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(30, 30, 36),
            Self::Light => egui::Color32::from_rgb(240, 240, 235),
            Self::Sepia => egui::Color32::from_rgb(245, 235, 210),
        }
    }
    pub fn surface(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(22, 22, 26),
            Self::Light => egui::Color32::from_rgb(241, 241, 237),
            Self::Sepia => egui::Color32::from_rgb(246, 236, 211),
        }
    }
    pub fn statusbar(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(16, 16, 20),
            Self::Light => egui::Color32::from_rgb(237, 237, 233),
            Self::Sepia => egui::Color32::from_rgb(242, 232, 207),
        }
    }
    pub fn border(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(40, 40, 50),
            Self::Light => egui::Color32::from_rgba_premultiplied(120, 120, 110, 28),
            Self::Sepia => egui::Color32::from_rgba_premultiplied(120, 90, 40, 28),
        }
    }
    pub fn divider_width(&self) -> f32 {
        match self { Self::Dark => 1.0, _ => 0.4 }
    }
    pub fn page_border(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgba_premultiplied(255, 255, 255, 12),
            Self::Light => egui::Color32::from_rgba_premultiplied(0, 0, 0, 18),
            Self::Sepia => egui::Color32::from_rgba_premultiplied(80, 50, 10, 22),
        }
    }
    pub fn text_primary(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(250, 250, 252),
            Self::Light => egui::Color32::from_rgb(28, 28, 32),
            Self::Sepia => egui::Color32::from_rgb(55, 35, 15),
        }
    }
    pub fn text_muted(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(120, 120, 140),
            Self::Light => egui::Color32::from_rgb(90, 90, 100),
            Self::Sepia => egui::Color32::from_rgb(110, 75, 40),
        }
    }
    pub fn text_subtle(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(70, 70, 85),
            Self::Light => egui::Color32::from_rgb(145, 145, 150),
            Self::Sepia => egui::Color32::from_rgb(150, 115, 72),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dark  => "🌙 Dark",
            Self::Light => "☀ Light",
            Self::Sepia => "📜 Sepia",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Dark  => Self::Light,
            Self::Light => Self::Sepia,
            Self::Sepia => Self::Dark,
        }
    }
    pub fn page_tint(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::WHITE,
            Self::Light => egui::Color32::from_rgb(245, 245, 252),
            Self::Sepia => egui::Color32::from_rgb(255, 235, 185),
        }
    }
    pub fn accent_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::from_rgb(255, 106,  26),
                1 => egui::Color32::from_rgb( 88, 166, 255),
                2 => egui::Color32::from_rgb(192, 132, 252),
                _ => egui::Color32::from_rgb( 74, 222, 128),
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(220,  85,  20),
                1 => egui::Color32::from_rgb( 37, 130, 246),
                2 => egui::Color32::from_rgb(147,  51, 234),
                _ => egui::Color32::from_rgb( 22, 163,  74),
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(200,  80,  20),
                1 => egui::Color32::from_rgb(217, 119,   6),
                2 => egui::Color32::from_rgb(146,  64,  14),
                _ => egui::Color32::from_rgb(190,  24,  93),
            },
        }
    }
    pub fn accent(&self) -> egui::Color32 { self.accent_for(0) }
    pub fn on_accent(&self) -> egui::Color32 { egui::Color32::WHITE }
    pub fn danger(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(220, 60, 60),
            Self::Light => egui::Color32::from_rgb(195, 40, 40),
            Self::Sepia => egui::Color32::from_rgb(180, 50, 35),
        }
    }
    pub fn surface_hover(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(45, 45, 58),
            Self::Light => egui::Color32::from_rgb(225, 225, 220),
            Self::Sepia => egui::Color32::from_rgb(232, 222, 197),
        }
    }
    pub fn surface_for(&self, palette: u8) -> egui::Color32 {
        let bg = self.bg_for(palette);
        match self {
            Self::Dark => egui::Color32::from_rgb(
                bg.r().saturating_sub(8),
                bg.g().saturating_sub(8),
                bg.b().saturating_sub(10),
            ),
            Self::Light | Self::Sepia => egui::Color32::from_rgb(
                bg.r().saturating_sub(3),
                bg.g().saturating_sub(3),
                bg.b().saturating_sub(3),
            ),
        }
    }
    pub fn statusbar_for(&self, palette: u8) -> egui::Color32 {
        let bg = self.bg_for(palette);
        match self {
            Self::Dark => egui::Color32::from_rgb(
                bg.r().saturating_sub(14),
                bg.g().saturating_sub(14),
                bg.b().saturating_sub(16),
            ),
            Self::Light | Self::Sepia => egui::Color32::from_rgb(
                bg.r().saturating_sub(6),
                bg.g().saturating_sub(6),
                bg.b().saturating_sub(4),
            ),
        }
    }
    pub fn page_tint_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::WHITE,
                1 => egui::Color32::from_rgb(200, 220, 255),
                2 => egui::Color32::from_rgb(220, 200, 255),
                _ => egui::Color32::from_rgb(200, 255, 215),
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(245, 245, 252),
                1 => egui::Color32::from_rgb(230, 242, 255),
                2 => egui::Color32::from_rgb(250, 250, 250),
                _ => egui::Color32::from_rgb(235, 252, 238),
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(255, 235, 185),
                1 => egui::Color32::from_rgb(255, 243, 199),
                2 => egui::Color32::from_rgb(249, 230, 210),
                _ => egui::Color32::from_rgb(255, 235, 245),
            },
        }
    }
    pub fn bg_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::from_rgb(30, 30, 36),
                1 => egui::Color32::from_rgb(13, 17, 23),
                2 => egui::Color32::from_rgb(26, 16, 37),
                _ => egui::Color32::from_rgb(13, 31, 20),
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(240, 240, 235),
                1 => egui::Color32::from_rgb(237, 244, 251),
                2 => egui::Color32::from_rgb(250, 250, 247),
                _ => egui::Color32::from_rgb(239, 246, 238),
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(245, 235, 210),
                1 => egui::Color32::from_rgb(254, 243, 199),
                2 => egui::Color32::from_rgb(249, 238, 226),
                _ => egui::Color32::from_rgb(253, 242, 248),
            },
        }
    }
}
