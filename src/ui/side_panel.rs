// ui/side_panel.rs — Side panels: TOC e Thumbnails (dispatcher)
// show_toc   → side_panel_toc.rs
// show_thumbs + render_thumb_item → side_panel_thumbs.rs

use eframe::egui;
use crate::pdf::thumb_cache::ThumbCache;
use crate::pdf::engine::BookmarkEntry;

pub use crate::ui::side_panel_toc::show_toc;
pub use crate::ui::side_panel_thumbs::show_thumbs;

/// Largura padrão do painel lateral
pub const SIDE_PANEL_WIDTH: f32 = 220.0;
