// app.rs — Estado global e dispatcher de UI do MariPDF
// Funções extraídas: app_constants.rs, app_color_mode.rs, app_home.rs,
//                    app_reader.rs, app_toolbar.rs, app_text_sel.rs, app_overlays.rs

use crate::app_about::AboutState;
use crate::app_frame::FrameMode;
use crate::app_window::WindowTracker;
use crate::pdf::document_worker::DocumentWorker;
use crate::pdf::engine::{get_pdfium, BookmarkEntry, PdfEngine, PdfLinkInfo, TextCharInfo};
use crate::pdf::persistence::{FileSession, PersistenceManager};
use crate::pdf::text_search::TextSearch;
use crate::pdf::thumb_cache::ThumbCache;
use crate::settings::{AppSettings, DefaultPanel};
use crate::ui::viewer::{PdfViewer, ViewMode};
use eframe::egui;
use std::{path::PathBuf, sync::Arc};

pub use crate::app_color_mode::ColorMode;
pub use crate::app_constants::{
    Toast, ACCENT_LIGHT, ACCENT_PRIMARY, BG_BASE, BG_SURFACE, BORDER_SUBTLE, MAX_ZOOM, MIN_ZOOM,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SUBTLE, WINE, WINE_DEEP, WINE_LIGHT,
};
pub use crate::app_reader::SidePanelKind;

// ── Estado global ─────────────────────────────────────────────────────────────

pub struct MariApp {
    pub version: &'static str,

    // Documento
    pub current_file: Option<PathBuf>,
    pub engine: Option<Arc<PdfEngine>>,
    pub document_worker: Option<DocumentWorker>,
    pub current_page: usize,
    pub page_count: usize,
    pub render_error: Option<String>,

    // Viewer
    pub viewer: PdfViewer,

    // UI state
    pub drop_zone_hovered: bool,
    pub fullscreen: bool,
    pub color_mode: ColorMode,
    pub system_prefers_dark: bool,

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
    /// Ctrl+A pedido antes de os chars da página chegarem.
    pub pending_select_all: bool,

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

    // Aba Sobre das Configurações (estado da verificação de atualização)
    pub about: AboutState,

    // Modo Flutuante (BF-415)
    pub frame: FrameMode,

    // Posição/tamanho da janela (BF-412)
    pub window: WindowTracker,

    // Ferramentas de leitura (app_tools.rs)
    pub nav: crate::app_tools::NavHistory,
    pub file_watch: crate::app_tools::FileWatch,
    /// Senha do documento aberto, só em memória (recarga automática).
    pub doc_password: Option<String>,
    pub password_prompt: Option<crate::app_tools::PasswordPrompt>,
    /// Páginas marcadas pelo usuário no documento atual (ordenadas).
    pub marks: Vec<usize>,
    pub properties: Option<crate::app_tools::DocProperties>,

    // Medidor/smoke test embutido (BF-002/BF-007), só com MARIPDF_BENCH
    pub bench: Option<crate::bench::BenchRun>,

    // Teste de longa duração (BF-708), só com MARIPDF_SOAK_SECS
    pub soak: Option<crate::soak::SoakRun>,

    // Capturas de tela do fluxo real (BF-706), só com MARIPDF_SHOWCASE
    pub showcase: Option<crate::showcase::ShowcaseRun>,
}

impl MariApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cli_file: Option<PathBuf>,
        system_prefers_dark: bool,
    ) -> Self {
        let _ = cc;
        let settings = AppSettings::load();
        let color_mode = settings.theme.resolve(system_prefers_dark);
        Self {
            version: env!("CARGO_PKG_VERSION"),
            current_file: None,
            engine: None,
            document_worker: None,
            current_page: 0,
            page_count: 0,
            render_error: None,
            viewer: PdfViewer::new(),
            drop_zone_hovered: false,
            fullscreen: false,
            color_mode,
            system_prefers_dark,
            goto_open: false,
            goto_input: String::new(),
            available_width: 800.0,
            persistence: PersistenceManager::load(),
            pending_open: cli_file,
            toast: None,
            search: TextSearch::new(),
            search_open: false,
            search_focused: false,
            pending_select_all: false,
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
            settings,
            settings_open: false,
            about: AboutState::default(),
            frame: FrameMode::default(),
            window: WindowTracker::default(),
            nav: Default::default(),
            file_watch: Default::default(),
            doc_password: None,
            password_prompt: None,
            marks: Vec::new(),
            properties: None,
            bench: crate::bench::BenchRun::from_env(),
            soak: crate::soak::SoakRun::from_env(),
            showcase: crate::showcase::ShowcaseRun::from_env(),
        }
    }

    // ── Abertura de arquivo ────────────────────────────────────────────────────

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_title(crate::t!("dialog-open-title"))
            .pick_file()
        {
            self.pending_open = Some(path);
        }
    }

    pub(crate) fn open_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.open_document(ctx, path, None, false);
    }

    /// Abre (ou recarrega, com `reload`) um PDF. Devolve true se abriu.
    /// Recarga mantém painel, modo e rotação; PDF com senha abre o pedido de
    /// senha em vez de mostrar erro.
    pub(crate) fn open_document(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
        password: Option<String>,
        reload: bool,
    ) -> bool {
        tracing::info!("Abrindo: {}", path.display());
        // Na recarga o estado vem do documento aberto agora, não do histórico
        // (que pode estar desligado ou desatualizado).
        let live = reload.then(|| FileSession {
            last_page: self.current_page,
            zoom: self.viewer.zoom,
            last_opened: 0,
            rotation: self.viewer.rotation,
            marks: self.marks.clone(),
        });
        let open_started = std::time::Instant::now();
        let opened =
            get_pdfium().and_then(|p| PdfEngine::open_with_password(&path, p, password.as_deref()));
        if let Some(bench) = self.bench.as_mut() {
            bench.record_open(open_started.elapsed().as_secs_f64() * 1000.0);
        }
        if let Some(soak) = self.soak.as_mut() {
            soak.record_open(opened.is_ok());
        }
        match opened {
            Ok(engine) => {
                // A troca so altera o documento atual depois que o novo PDF
                // abriu com sucesso. Assim uma entrada invalida nao desmonta
                // a sessao que o usuario ainda esta lendo.
                self.save_session();
                let engine = Arc::new(engine);
                let repaint = ctx.clone();
                let mut document_worker = DocumentWorker::with_waker(
                    Arc::clone(&engine),
                    Some(Arc::new(move || repaint.request_repaint())),
                );
                document_worker.request_bookmarks();
                let scheduler = document_worker.handle();
                let page_count = engine.page_count();
                self.page_count = page_count;
                self.bookmarks.clear();
                self.engine = Some(Arc::clone(&engine));
                self.document_worker = Some(document_worker);
                self.current_file = Some(path.clone());
                self.search.attach_document(scheduler.clone(), page_count);
                self.render_error = None;
                self.page_links.clear();
                self.links_page = usize::MAX;
                self.text_chars.clear();
                self.text_chars_page = usize::MAX;
                self.sel_anchor = None;
                self.sel_focus = None;
                self.pending_select_all = false;
                let mode = self.viewer.mode;
                self.viewer.reset();
                if reload {
                    self.viewer.mode = mode;
                } else {
                    self.side_panel = match self.settings.default_panel {
                        DefaultPanel::None => SidePanelKind::None,
                        DefaultPanel::Thumbnails => SidePanelKind::Thumbs,
                        DefaultPanel::Toc => SidePanelKind::Toc,
                    };
                    self.nav.clear();
                }
                self.thumb_cache = (self.side_panel == SidePanelKind::Thumbs)
                    .then(|| ThumbCache::new(page_count, scheduler.clone()));
                self.doc_password = password;
                self.file_watch.reset(&path);
                self.properties = None;
                let session = live
                    .clone()
                    .or_else(|| self.persistence.get(&path).cloned())
                    .unwrap_or_default();
                if session.zoom > 0.0 {
                    self.viewer.zoom = session.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                    self.current_page = session.last_page.min(page_count.saturating_sub(1));
                } else {
                    self.current_page = 0;
                    self.viewer.zoom = 1.0;
                }
                self.marks = session.marks.clone();
                self.marks.retain(|&p| p < page_count);
                self.viewer.rotation = session.rotation;
                engine.set_rotation(session.rotation);
                self.viewer.init_cache(scheduler, page_count);
                if self.viewer.mode == ViewMode::SinglePage {
                    self.render_current_page(ctx);
                } else {
                    self.viewer.request_continuous_page(self.current_page);
                }
                true
            }
            Err(e)
                if e.downcast_ref::<crate::pdf::engine::PasswordRequired>()
                    .is_some() =>
            {
                tracing::info!(path = %path.display(), "PDF pede senha");
                let wrong = password.is_some();
                self.password_prompt = Some(crate::app_tools::PasswordPrompt::new(path, wrong));
                false
            }
            Err(e) => {
                let msg = error_to_user_message(&e);
                tracing::error!(path = %path.display(), error = %e, "Falha ao abrir PDF");
                let now = ctx.input(|i| i.time);
                if let Some(bench) = self.bench.as_mut() {
                    bench.record_open_failure(msg.clone());
                }
                self.toast = Some(Toast::error(msg, 5.0, now));
                false
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
                    rotation: self.viewer.rotation,
                    marks: self.marks.clone(),
                },
            );
        }
    }
}

// ── eframe::App — loop principal ──────────────────────────────────────────────

impl eframe::App for MariApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_session();
        let track = self.tracks_window();
        self.window.on_exit(track);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let started = std::time::Instant::now();
        let scripted = self.bench.is_some() || self.soak.is_some() || self.showcase.is_some();
        let track = self.tracks_window() && !scripted;
        self.window.on_frame(ctx, frame, track);
        configure_visuals(ctx);
        crate::app_update::run(self, ctx);
        if self.bench.is_some() {
            let frame_ms = started.elapsed().as_secs_f64() * 1000.0;
            self.bench_step(ctx, frame_ms);
        }
        if self.soak.is_some() {
            self.soak_step(ctx);
        }
        if self.showcase.is_some() {
            self.showcase_step(ctx);
        }
    }
}

impl MariApp {
    /// Flutuante e tela cheia não representam a janela normal do usuário.
    fn tracks_window(&self) -> bool {
        !self.frame.active && !self.fullscreen
    }
}

// ── Helpers globais ───────────────────────────────────────────────────────────

pub(crate) fn configure_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG_BASE;
    v.window_fill = BG_SURFACE;
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.window_stroke = egui::Stroke::new(1.0_f32, BORDER_SUBTLE);
    v.override_text_color = Some(TEXT_PRIMARY);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 48, 56);
    // Nada de azul padrão do egui: seleção, foco, cursor, links e botões
    // nativos (hover/pressionado) usam vinho.
    v.widgets.hovered.bg_fill = WINE;
    v.widgets.hovered.weak_bg_fill = WINE;
    v.widgets.active.bg_fill = WINE_DEEP;
    v.widgets.active.weak_bg_fill = WINE_DEEP;
    v.selection.bg_fill = WINE;
    v.selection.stroke = egui::Stroke::new(1.0_f32, WINE_LIGHT);
    v.text_cursor.stroke = egui::Stroke::new(2.0_f32, WINE_LIGHT);
    v.hyperlink_color = WINE_LIGHT;
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
        return crate::t!("err-pdf-password");
    }
    if s.contains("corrupt")
        || s.contains("invalid")
        || s.contains("damaged")
        || s.contains("format")
    {
        return crate::t!("err-pdf-corrupt");
    }
    if s.contains("not found") || s.contains("no such file") {
        return crate::t!("err-pdf-not-found");
    }
    crate::t!("err-pdf-open", error => e)
}
