// app_constants.rs — Constantes de cor globais e struct Toast
// Extraído de app.rs para manter o arquivo principal abaixo de 250 linhas.

use eframe::egui;

// ── Paleta base (Dark) ────────────────────────────────────────────────────────
pub const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(22, 22, 26);
/// Vermelho da marca (#AF0C0C) para preenchimentos: botões, chips, seleção.
/// Texto branco sobre ele tem contraste 7.2:1.
pub const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(175, 12, 12);
/// Coral do logo (#F15855): ícones e textos em destaque sobre fundo escuro,
/// onde o vermelho de preenchimento ficaria ilegível.
pub const ACCENT_LIGHT: egui::Color32 = egui::Color32::from_rgb(241, 88, 85);
/// Vinho dos controles nativos do egui (seleção, foco, botões no hover),
/// no lugar do azul padrão.
pub const WINE: egui::Color32 = egui::Color32::from_rgb(122, 28, 48);
pub const WINE_DEEP: egui::Color32 = egui::Color32::from_rgb(96, 20, 36);
/// Vinho claro para traços e links sobre fundo escuro.
pub const WINE_LIGHT: egui::Color32 = egui::Color32::from_rgb(226, 140, 156);
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(250, 250, 252);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(120, 120, 140);
pub const TEXT_SUBTLE: egui::Color32 = egui::Color32::from_rgb(70, 70, 85);
pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(40, 40, 50);

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 5.0;

// ── Toast ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastKind {
    Info,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub expires: f64,
}

impl Toast {
    pub fn info(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Info,
            expires: now + duration_secs,
        }
    }
    pub fn error(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Error,
            expires: now + duration_secs,
        }
    }
    pub fn is_alive(&self, now: f64) -> bool {
        now < self.expires
    }
    pub fn alpha(&self, now: f64) -> f32 {
        ((self.expires - now).max(0.0) / 0.5).min(1.0) as f32
    }
}
