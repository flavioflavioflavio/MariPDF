// app.rs — Estado global e dispatcher de UI do MariPDF
// Funções extraídas: app_constants.rs, app_color_mode.rs, app_home.rs,
//                    app_reader.rs, app_toolbar.rs, app_text_sel.rs, app_overlays.rs

use crate::app_about::AboutState;
use crate::pdf::engine::{get_pdfium, BookmarkEntry, PdfEngine, PdfLinkInfo, TextCharInfo};
use crate::pdf::persistence::{FileSession, PersistenceManager};
use crate::pdf::text_search::TextSearch;
use crate::pdf::thumb_cache::ThumbCache;
use crate::settings::{AppSettings, DefaultPanel};
use crate::ui::viewer::{PdfViewer, ViewMode};
use eframe::egui;
use std::path::PathBuf;

pub use crate::app_color_mode::ColorMode;
pub use crate::app_constants::{
    Toast, ACCENT_LIGHT, ACCENT_PRIMARY, BG_BASE, BG_SURFACE, BORDER_SUBTLE, MAX_ZOOM, MIN_ZOOM,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SUBTLE,
};
pub use crate::app_reader::SidePanelKind;

// ── Estado global ─────────────────────────────────────────────────────────────

pub struct MariApp {
    pub version: &'static str,

    // Documento
    pub current_file: Option<PathBuf>,
    pub engine: Option<PdfEngine>,
    pub current_page: usize,
    pub page_count: usize,
    pub render_error: Option<String>,

    // Viewer
    pub viewer: PdfViewer,

    // UI state
    pub drop_zone_hovered: bool,
    pub fullscreen: bool,
    pub color_mode: ColorMode,

    // Overlay GoTo
    pub goto_open: bool,
    pub goto_input: String,

    // Fit Width
    pub available_width: f32,

    // Persistência
    pub persistence: PersistenceManager,
    pub pending_open: Option<PathBuf>,

    // Toast
    pub toast: Option<Toast>,

    // Busca (Ctrl+F)
    pub search: TextSearch,
    pub search_open: bool,
    pub search_focused: bool,

    // Seleção de texto
    pub sel_anchor: Option<usize>,
    pub sel_focus: Option<usize>,
    pub text_chars: Vec<TextCharInfo>,
    pub text_chars_page: usize,
    pub selected_text: Option<String>,

    // Links
    pub page_links: Vec<PdfLinkInfo>,
    pub links_page: usize,

    // Flags
    pub pending_rerender: bool,
    pub toolbar_opacity: f32,
    pub pending_close: bool,

    // Side Panel
    pub side_panel: SidePanelKind,
    pub bookmarks: Vec<BookmarkEntry>,
    pub thumb_cache: Option<ThumbCache>,

    // Configurações
    pub settings: AppSettings,
    pub settings_open: bool,

    // About dialog
    pub about: AboutState,
}

impl MariApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cli_file: Option<PathBuf>,
        initial_color_mode: ColorMode,
    ) -> Self {
        let _ = cc;
        Self {
            version: env!("CARGO_PKG_VERSION"),
            current_file: None,
            engine: None,
            current_page: 0,
            page_count: 0,
            render_error: None,
            viewer: PdfViewer::new(),
            drop_zone_hovered: false,
            fullscreen: false,
            color_mode: initial_color_mode,
            goto_open: false,
            goto_input: String::new(),
            available_width: 800.0,
            persistence: PersistenceManager::load(),
            pending_open: cli_file,
            toast: None,
            search: TextSearch::new(),
            search_open: false,
            search_focused: false,
            selected_text: None,
            sel_anchor: None,
            sel_focus: None,
            text_chars: Vec::new(),
            text_chars_page: usize::MAX,
            page_links: Vec::new(),
            links_page: usize::MAX,
            pending_rerender: false,
            toolbar_opacity: 1.0,
            pending_close: false,
            side_panel: SidePanelKind::None,
            bookmarks: Vec::new(),
            thumb_cache: None,
            settings: AppSettings::load(),
            settings_open: false,
            about: AboutState::default(),
        }
    }

    // ── Abertura de arquivo ────────────────────────────────────────────────────

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_title("Abrir PDF — MariPDF")
            .pick_file()
        {
            self.pending_open = Some(path);
        }
    }

    pub(crate) fn open_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        tracing::info!("Abrindo: {}", path.display());
        match get_pdfium().and_then(|p| PdfEngine::open(&path, p)) {
            Ok(engine) => {
                let page_count = engine.page_count();
                self.page_count = page_count;
                self.bookmarks = engine.extract_bookmarks();
                self.engine = Some(engine);
                self.current_file = Some(path.clone());
                self.search.attach_document(path.clone());
                self.render_error = None;
                self.viewer.reset();
                self.side_panel = match self.settings.default_panel {
                    DefaultPanel::None => SidePanelKind::None,
                    DefaultPanel::Thumbnails => SidePanelKind::Thumbs,
                    DefaultPanel::Toc => SidePanelKind::Toc,
                };
                self.thumb_cache = Some(ThumbCache::new(path.clone(), page_count));
                if self.viewer.mode == ViewMode::Continuous {
                    let scale = self.viewer.zoom * self.settings.render_quality.scale();
                    self.viewer.init_cache(path.clone(), scale);
                }
                if let Some(session) = self.persistence.get(&path) {
                    self.viewer.zoom = session.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                    self.current_page = session.last_page.min(page_count.saturating_sub(1));
                } else {
                    self.current_page = 0;
                    self.viewer.zoom = 1.0;
                }
                if self.viewer.mode == ViewMode::SinglePage {
                    self.render_current_page(ctx);
                }
            }
            Err(e) => {
                let msg = error_to_user_message(&e);
                tracing::error!("{e}");
                let now = ctx.input(|i| i.time);
                self.toast = Some(Toast::error(msg, 5.0, now));
                self.render_error = None;
                self.current_file = None;
            }
        }
    }

    pub fn save_session(&mut self) {
        if !self.settings.track_history {
            return;
        }
        if let Some(ref path) = self.current_file.clone() {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.persistence.save(
                path,
                FileSession {
                    last_page: self.current_page,
                    zoom: self.viewer.zoom,
                    last_opened: now_unix,
                },
            );
        }
    }
}

// ── eframe::App — loop principal ──────────────────────────────────────────────

impl eframe::App for MariApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_session();
        // Salva tamanho atual da janela
        // (available_width é a largura do painel central; usamos como aproximação)
        PersistenceManager::save_window(self.available_width + 48.0, 768.0);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        configure_visuals(ctx);
        crate::app_update::run(self, ctx);
    }
}

// ── Helpers globais ───────────────────────────────────────────────────────────

pub(crate) fn configure_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG_BASE;
    v.window_fill = BG_SURFACE;
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.window_stroke = egui::Stroke::new(1.0, BORDER_SUBTLE);
    v.override_text_color = Some(TEXT_PRIMARY);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(65, 48, 128);
    v.widgets.hovered.bg_fill = ACCENT_PRIMARY;
    v.widgets.active.bg_fill = ACCENT_LIGHT;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.scroll.bar_width = 7.0;
        s.spacing.scroll.bar_inner_margin = 2.0;
        s.spacing.scroll.bar_outer_margin = 1.0;
    });
}

pub(crate) fn error_to_user_message(e: &anyhow::Error) -> String {
    let s = e.to_string().to_lowercase();
    if s.contains("password") || s.contains("encrypted") || s.contains("senha") {
        return "🔒 PDF protegido por senha — não suportado no momento".into();
    }
    if s.contains("corrupt") || s.contains("invalid") || s.contains("damaged") {
        return "⚠ Arquivo PDF corrompido ou inválido".into();
    }
    if s.contains("not found") || s.contains("no such file") {
        return "❌ Arquivo não encontrado".into();
    }
    format!("⚠ Erro ao abrir PDF: {e}")
}
