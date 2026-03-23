// icons.rs — Mapeamento de ícones Phosphor para uso no egui
//
// Fonte: Phosphor Regular (assets/phosphor.ttf) embedada no binário.
// Uso: `ph(ARROW_LEFT, 16.0, color)` → RichText com FontFamily::Name("Icons")
//
// Codepoints extraídos diretamente via análise do GSUB ligature table da fonte.
// Verificados contra a tabela cmap PUA (U+E000..U+EBD2).
// Verificação: "at" = U+E0AE ✓, "asterisk" = U+E0AC ✓

#![allow(dead_code)]

use eframe::egui;


// ── Codepoints Phosphor Regular (verificados via GSUB decode) ─────────────────

pub const ARROW_LEFT:       &str = "\u{e05a}"; // ArrowLeft
pub const ARROW_RIGHT:      &str = "\u{e06e}"; // ArrowRight
pub const ARROW_UP:         &str = "\u{e090}"; // ArrowUp

pub const CARET_LEFT:       &str = "\u{e13a}"; // CaretLeft
pub const CARET_RIGHT:      &str = "\u{e13c}"; // CaretRight

pub const MAGNIFY_GLASS:    &str = "\u{e30e}"; // MagnifyingGlass
pub const X_CIRCLE:         &str = "\u{e4fa}"; // XCircle (close/dismiss)
pub const CHECK:            &str = "\u{e184}"; // Check

pub const FILE_PDF:         &str = "\u{e704}"; // FilePdf
pub const FOLDER_OPEN:      &str = "\u{e258}"; // FolderOpen
pub const CLOCK:            &str = "\u{e1a0}"; // Clock
pub const UPLOAD_SIMPLE:    &str = "\u{e4c2}"; // UploadSimple

pub const MOON:             &str = "\u{e330}"; // Moon
pub const SUN:              &str = "\u{e472}"; // Sun
pub const LAMP_PENDANT:     &str = "\u{ee2e}"; // LampPendant (genie lamp)
pub const BOOK_OPEN:        &str = "\u{e0e8}"; // BookOpen

pub const MINUS:            &str = "\u{e32c}"; // Minus
pub const PLUS:             &str = "\u{e3d6}"; // Plus
pub const ARROWS_OUT:       &str = "\u{e0a4}"; // ArrowsOut (fit)
pub const FRAME_CORNERS:    &str = "\u{e628}"; // FrameCorners (wait - see note)

pub const ROWS:             &str = "\u{e5a4}"; // Rows (continuous mode)
pub const SQUARE_HALF:      &str = "\u{e464}"; // SquareHalf (single page)
pub const ARROW_COUNTER_CW: &str = "\u{e03a}"; // ArrowCounterClockwise (reset)

pub const WARNING:          &str = "\u{e4e2}"; // Warning
pub const SPINNER_GAP:      &str = "\u{e66e}"; // SpinnerGap (loading)

pub const LIST:             &str = "\u{e2dc}"; // List (TOC)
pub const SQUARES_FOUR:     &str = "\u{e472}"; // SquaresFour (Thumbnails)
pub const SLIDERS:          &str = "\u{e45c}"; // SlidersHorizontal (Settings)
pub const GEAR:             &str = "\u{e270}"; // Gear (Settings v1.0)
pub const PAINT_BRUSH:      &str = "\u{e37e}"; // PaintBrush (Appearance)
pub const BOOK:             &str = "\u{e0e4}"; // Book (Reading)
pub const SHIELD:           &str = "\u{e446}"; // Shield (Privacy)
pub const INFO:             &str = "\u{e2b8}"; // Info (About)

// ── Helper ────────────────────────────────────────────────────────────────────

/// Cria um `RichText` com o ícone Phosphor solicitado.
/// `glyph` = constante `icons::*`, `size` = tamanho em pts, `color` = cor RGBA.
pub fn ph(glyph: &str, size: f32, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(glyph)
        .size(size)
        .color(color)
        .font(egui::FontId::new(size, egui::FontFamily::Name("Icons".into())))
}
