// icons.rs — Mapeamento de ícones Phosphor para uso no egui
//
// Fonte: Phosphor Regular 2.1 (assets/phosphor.ttf) embutida no binário.
// Uso: `ph(ARROW_LEFT, 16.0, color)` → RichText com FontFamily::Name("Icons").
//
// Codepoints conferidos contra o CSS oficial de @phosphor-icons/web@2.1.1
// (src/regular/style.css); todos existem no cmap da fonte embutida.
// Ao trocar a fonte, regenere a tabela a partir do CSS da mesma versão.

#![allow(dead_code)]

use eframe::egui;

// ── Navegação ─────────────────────────────────────────────────────────────────

pub const ARROW_LEFT: &str = "\u{e058}"; // arrow-left — voltar
pub const ARROW_RIGHT: &str = "\u{e06c}"; // arrow-right
pub const ARROW_UP: &str = "\u{e08e}"; // arrow-up
pub const CARET_LEFT: &str = "\u{e138}"; // caret-left — página anterior / expandir
pub const CARET_RIGHT: &str = "\u{e13a}"; // caret-right — próxima página / recolher
pub const CARET_DOWN: &str = "\u{e136}"; // caret-down — abre menu

// ── Ações ─────────────────────────────────────────────────────────────────────

pub const MAGNIFY_GLASS: &str = "\u{e30c}"; // magnifying-glass — busca
pub const X_CIRCLE: &str = "\u{e4f8}"; // x-circle — fechar/dispensar
pub const X: &str = "\u{e4f6}"; // x — fechar barra/overlay
pub const CHECK: &str = "\u{e182}"; // check
pub const FILE_PDF: &str = "\u{e702}"; // file-pdf
pub const FOLDER_OPEN: &str = "\u{e256}"; // folder-open
pub const CLOCK: &str = "\u{e19a}"; // clock — recentes
pub const UPLOAD_SIMPLE: &str = "\u{e4c0}"; // upload-simple — drop zone
pub const ARROW_COUNTER_CW: &str = "\u{e038}"; // arrow-counter-clockwise — reset

// ── Tema ──────────────────────────────────────────────────────────────────────

pub const MOON: &str = "\u{e330}"; // moon — Dark
pub const SUN: &str = "\u{e472}"; // sun — Light
pub const LAMP_PENDANT: &str = "\u{ee2e}"; // lamp-pendant — Sépia
pub const BOOK_OPEN: &str = "\u{e0e6}"; // book-open

// ── Zoom e visualização ───────────────────────────────────────────────────────

pub const MINUS: &str = "\u{e32a}"; // minus — afastar
pub const PLUS: &str = "\u{e3d4}"; // plus — aproximar
pub const ARROWS_OUT: &str = "\u{e0a2}"; // arrows-out
pub const ARROWS_OUT_LINE_H: &str = "\u{e534}"; // arrows-out-line-horizontal — ajustar largura
pub const FRAME_CORNERS: &str = "\u{e626}"; // frame-corners — ajustar página
pub const FILE: &str = "\u{e230}"; // file — modo página única
pub const ROWS: &str = "\u{e5a2}"; // rows — modo contínuo
pub const SQUARE_HALF: &str = "\u{e462}"; // square-half
pub const PICTURE_IN_PICTURE: &str = "\u{e64c}"; // picture-in-picture — modo Flutuante
pub const HAND: &str = "\u{e298}"; // hand — ferramenta mão (pan)
pub const PUSH_PIN: &str = "\u{e3e2}"; // push-pin — sempre no topo

// ── Estado ────────────────────────────────────────────────────────────────────

pub const WARNING: &str = "\u{e4e0}"; // warning
pub const SPINNER_GAP: &str = "\u{e66c}"; // spinner-gap — carregando

// ── Painéis e configurações ───────────────────────────────────────────────────

pub const LIST: &str = "\u{e2f0}"; // list — sumário/TOC
pub const SQUARES_FOUR: &str = "\u{e464}"; // squares-four — miniaturas
pub const SLIDERS: &str = "\u{e434}"; // sliders-horizontal
pub const GEAR: &str = "\u{e270}"; // gear — configurações
pub const PAINT_BRUSH: &str = "\u{e6f0}"; // paint-brush — aba Aparência
pub const BOOK: &str = "\u{e0e2}"; // book — aba Leitura
pub const SHIELD: &str = "\u{e40a}"; // shield — aba Privacidade
pub const INFO: &str = "\u{e2ce}"; // info — sobre

// ── Ferramentas de leitura ────────────────────────────────────────────────────

pub const DOTS_THREE: &str = "\u{e1fe}"; // dots-three — menu Mais
pub const ARROW_CLOCKWISE: &str = "\u{e036}"; // arrow-clockwise — girar à direita
pub const BOOKMARK_SIMPLE: &str = "\u{e0ea}"; // bookmark-simple — marcador
pub const COPY: &str = "\u{e1ca}"; // copy — copiar página
pub const LOCK_KEY: &str = "\u{e2fe}"; // lock-key — PDF com senha

// ── Helper ────────────────────────────────────────────────────────────────────

/// Cria um `RichText` com o ícone Phosphor solicitado.
/// `glyph` = constante `icons::*`, `size` = tamanho em pts, `color` = cor RGBA.
pub fn ph(glyph: &str, size: f32, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(glyph)
        .size(size)
        .color(color)
        .font(egui::FontId::new(
            size,
            egui::FontFamily::Name("Icons".into()),
        ))
}
