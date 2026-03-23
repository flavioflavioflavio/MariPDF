// app.rs — Estado global e loop de UI principal do MariPDF

use eframe::egui;
use std::path::PathBuf;
use crate::pdf::engine::{PdfEngine, PdfLinkInfo, TextCharInfo, get_pdfium, BookmarkEntry};
use crate::pdf::persistence::{FileSession, PersistenceManager};
use crate::pdf::text_search::{TextSearch, search_all_pages};
use crate::pdf::thumb_cache::ThumbCache;
use crate::ui::viewer::{PdfViewer, ViewMode, invert_rgba};
use crate::ui::icons;
use crate::ui::side_panel;
use crate::ui::side_panel::SIDE_PANEL_WIDTH;
use crate::settings::AppSettings;

// ── Paleta ────────────────────────────────────────────────────────────────────
const BG_BASE:    egui::Color32 = egui::Color32::from_rgb(14, 14, 16);
const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(22, 22, 26);
/// Laranja primário vibrante — botões CTA, accents fortes
const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(255, 106, 26);   // #FF6A1A
/// Laranja claro — labels, glows, textos accent
const ACCENT_LIGHT:   egui::Color32 = egui::Color32::from_rgb(255, 165, 80);   // #FFA550
/// Ciano 65% saturação — accent secundário (handles, pills, links)
const TEXT_PRIMARY:   egui::Color32 = egui::Color32::from_rgb(250, 250, 252);
const TEXT_MUTED:     egui::Color32 = egui::Color32::from_rgb(120, 120, 140);
const TEXT_SUBTLE:    egui::Color32 = egui::Color32::from_rgb(70, 70, 85);
const BORDER_SUBTLE:  egui::Color32 = egui::Color32::from_rgb(40, 40, 50);

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;

// ── Toast ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToastKind { Info, Error }

#[derive(Clone)]
struct Toast {
    message: String,
    kind:    ToastKind,
    /// Instante em que o toast expira (em segundos desde o epoch monotônico do proceso)
    expires: f64,
}

impl Toast {
    fn info(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Info, expires: now + duration_secs }
    }
    fn error(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Error, expires: now + duration_secs }
    }
    fn is_alive(&self, now: f64) -> bool { now < self.expires }
    /// 0.0 = sumindo, 1.0 = totalmente visível
    fn alpha(&self, now: f64) -> f32 {
        let remaining = (self.expires - now).max(0.0);
        // fade-out no último 0.5s
        (remaining / 0.5).min(1.0) as f32
    }
}


// ── Modos de cor ─────────────────────────────────────────────────────────────

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
    /// Fundo da toolbar e janelas — em Light/Sepia quase igual ao bg
    pub fn surface(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(22, 22, 26),
            Self::Light => egui::Color32::from_rgb(241, 241, 237),
            Self::Sepia => egui::Color32::from_rgb(246, 236, 211),
        }
    }
    /// Fundo da status bar
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
    /// Espessura do separador no toolbar/statusbar
    pub fn divider_width(&self) -> f32 {
        match self {
            Self::Dark  => 1.0,
            Self::Light => 0.4,
            Self::Sepia => 0.4,
        }
    }
    /// Borda fina da página PDF
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
    fn next(&self) -> Self {
        match self {
            Self::Dark  => Self::Light,
            Self::Light => Self::Sepia,
            Self::Sepia => Self::Dark,
        }
    }
    /// Cor de tinta aplicada ao bitmap (egui multiplica pixel * tint)
    /// Dark = sem filtro (WHITE), Light = levemente frio, Sepia = cálido dourado
    pub fn page_tint(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::WHITE,
            Self::Light => egui::Color32::from_rgb(245, 245, 252), // levemente azulado-frio
            Self::Sepia => egui::Color32::from_rgb(255, 235, 185), // amarelo sepia
        }
    }

    /// Cor de accent do modo — muda com a paleta selecionada.
    pub fn accent_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::from_rgb(255, 106,  26), // Laranja (padrão)
                1 => egui::Color32::from_rgb( 88, 166, 255), // Azul meia-noite
                2 => egui::Color32::from_rgb(192, 132, 252), // Violeta
                _ => egui::Color32::from_rgb( 74, 222, 128), // Verde floresta
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(220,  85,  20), // Laranja (padrão)
                1 => egui::Color32::from_rgb( 37, 130, 246), // Azul ártico
                2 => egui::Color32::from_rgb(147,  51, 234), // Violeta papel
                _ => egui::Color32::from_rgb( 22, 163,  74), // Verde menta
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(200,  80,  20), // Laranja (padrão)
                1 => egui::Color32::from_rgb(217, 119,   6), // Âmbar
                2 => egui::Color32::from_rgb(146,  64,  14), // Vintage marrom
                _ => egui::Color32::from_rgb(190,  24,  93), // Rosa antigo
            },
        }
    }

    /// Cor de accent simples (sem paleta) — usa padrão 0
    pub fn accent(&self) -> egui::Color32 { self.accent_for(0) }

    /// Cor de texto SOBRE o accent (invariante por modo — sempre branco)
    pub fn on_accent(&self) -> egui::Color32 { egui::Color32::WHITE }

    /// Cor de perigo (destrutivo) — adaptada por modo
    pub fn danger(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(220, 60, 60),
            Self::Light => egui::Color32::from_rgb(195, 40, 40),
            Self::Sepia => egui::Color32::from_rgb(180, 50, 35),
        }
    }

    /// Hover sutil sobre surface — chip não-selecionado ao passar mouse
    pub fn surface_hover(&self) -> egui::Color32 {
        match self {
            Self::Dark  => egui::Color32::from_rgb(45, 45, 58),
            Self::Light => egui::Color32::from_rgb(225, 225, 220),
            Self::Sepia => egui::Color32::from_rgb(232, 222, 197),
        }
    }

    /// Surface ajustado pela paleta
    pub fn surface_for(&self, palette: u8) -> egui::Color32 {
        // Derivamos escurecendo/clareando o bg ligeiramente
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

    /// Statusbar ajustado pela paleta
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

    /// Tint da página PDF ajustado pela paleta
    pub fn page_tint_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::WHITE,                              // sem tint
                1 => egui::Color32::from_rgb(200, 220, 255),           // azulado frio
                2 => egui::Color32::from_rgb(220, 200, 255),           // violeta suave
                _ => egui::Color32::from_rgb(200, 255, 215),           // verde menta
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(245, 245, 252),           // levemente frio
                1 => egui::Color32::from_rgb(230, 242, 255),           // ártico
                2 => egui::Color32::from_rgb(250, 250, 250),           // neutro papel
                _ => egui::Color32::from_rgb(235, 252, 238),           // menta suave
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(255, 235, 185),           // sépia clássico
                1 => egui::Color32::from_rgb(255, 243, 199),           // âmbar quente
                2 => egui::Color32::from_rgb(249, 230, 210),           // vintage
                _ => egui::Color32::from_rgb(255, 235, 245),           // rosa antigo
            },
        }
    }

    /// Bg ajustado pela paleta
    pub fn bg_for(&self, palette: u8) -> egui::Color32 {
        match self {
            Self::Dark => match palette {
                0 => egui::Color32::from_rgb(30, 30, 36),     // Carvão (padrão)
                1 => egui::Color32::from_rgb(13, 17, 23),     // Meia-noite GitHub
                2 => egui::Color32::from_rgb(26, 16, 37),     // Violeta Escuro
                _ => egui::Color32::from_rgb(13, 31, 20),     // Verde Floresta
            },
            Self::Light => match palette {
                0 => egui::Color32::from_rgb(240, 240, 235),  // Névoa (padrão)
                1 => egui::Color32::from_rgb(237, 244, 251),  // Ártico
                2 => egui::Color32::from_rgb(250, 250, 247),  // Papel
                _ => egui::Color32::from_rgb(239, 246, 238),  // Menta
            },
            Self::Sepia => match palette {
                0 => egui::Color32::from_rgb(245, 235, 210),  // Pergaminho (padrão)
                1 => egui::Color32::from_rgb(254, 243, 199),  // Âmbar claro
                2 => egui::Color32::from_rgb(249, 238, 226),  // Vintage
                _ => egui::Color32::from_rgb(253, 242, 248),  // Rosa Antigo
            },
        }
    }
}

// ── Side Panel ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SidePanelKind {
    #[default]
    None,
    Toc,
    Thumbs,
}

// ── Estado do app ─────────────────────────────────────────────────────────────

pub struct MariApp {
    version: &'static str,

    // ── Documento ──
    current_file:  Option<PathBuf>,
    engine:        Option<PdfEngine>,
    current_page:  usize,
    page_count:    usize,
    render_error:  Option<String>,

    // ── Viewer ──
    viewer:        PdfViewer,

    // ── UI state ──
    drop_zone_hovered: bool,
    fullscreen:        bool,
    color_mode:        ColorMode,

    // ── Overlay "Ir para página" (Ctrl+G) ──
    goto_open:         bool,
    goto_input:        String,

    // ── Fit Width state ──
    /// Largura disponível capturada no último frame (para calcular Fit Width)
    available_width:   f32,

    // ── Persistência ──
    persistence:       PersistenceManager,

    // ── Arquivo pendente (CLI / drop / dialog) ──
    pending_open:      Option<PathBuf>,

    // ── Toast ──
    toast:             Option<Toast>,

    // ── Busca full-text (Ctrl+F) ──
    search:            TextSearch,
    search_open:       bool,
    search_focused:    bool,   // auto-foca o TextEdit no frame seguinte

    // ── Seleção de texto por drag/mouse ──
    /// Índice do char onde o drag começou (âncora)
    sel_anchor:      Option<usize>,
    /// Índice do char onde o drag está agora (foco — atualizado a cada frame)
    sel_focus:       Option<usize>,
    /// Cache dos chars (com bounds em coords PDF) da página atual
    text_chars:      Vec<TextCharInfo>,
    /// Página para qual text_chars foi carregado
    text_chars_page: usize,
    /// Texto selecionado (para Ctrl+C)
    selected_text:   Option<String>,

    // ── Links da página atual (cache por página) ──
    page_links:    Vec<PdfLinkInfo>,
    /// Últtima página para qual os links foram carregados (evita re-fetch todo frame)
    links_page:    usize,

    /// Rerender da página pendente (ex: troca de tema com inversão)
    pending_rerender: bool,

    /// Opacidade animada do toolbar (0.0 = transparente, 1.0 = opaco)
    /// Aumenta quando o mouse entra na área do topo; diminui ao afastar.
    toolbar_opacity: f32,

    /// Flag para fechar o documento no próximo frame (evita modificar state no meio de render)
    pending_close: bool,

    // ── Side Panels ──
    side_panel:  SidePanelKind,
    bookmarks:   Vec<BookmarkEntry>,
    thumb_cache: Option<ThumbCache>,
    // ── Configurações ──
    settings:          AppSettings,
    settings_open:     bool,
    needs_font_reload: bool,
    needs_render_reload: bool,
}


impl MariApp {
    pub fn new(cc: &eframe::CreationContext<'_>, cli_file: Option<PathBuf>, initial_color_mode: ColorMode) -> Self {
        let _ = cc;
        Self {
            version:           env!("CARGO_PKG_VERSION"),
            current_file:      None,
            engine:            None,
            current_page:      0,
            page_count:        0,
            render_error:      None,
            viewer:            PdfViewer::new(),
            drop_zone_hovered: false,
            fullscreen:        false,
            color_mode:        initial_color_mode,
            goto_open:         false,
            goto_input:        String::new(),
            available_width:   800.0,
            persistence:       PersistenceManager::load(),
            pending_open:      cli_file,
            toast:             None,
            search:            TextSearch::new(),
            search_open:       false,
            search_focused:    false,
            selected_text:     None,
            sel_anchor:        None,
            sel_focus:         None,
            text_chars:        Vec::new(),
            text_chars_page:   usize::MAX,
            page_links:        Vec::new(),
            links_page:        usize::MAX,
            pending_rerender:  false,
            toolbar_opacity:   1.0,
            pending_close:     false,
            side_panel:        SidePanelKind::None,
            bookmarks:         Vec::new(),
            thumb_cache:       None,
            settings:          AppSettings::load(),
            settings_open:     false,
            needs_font_reload: false,
            needs_render_reload: false,
        }
    }

    // ── Abertura de arquivo ───────────────────────────────────────────────────

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_title("Abrir PDF — MariPDF")
            .pick_file()
        {
            self.pending_open = Some(path);
        }
    }

    fn open_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        tracing::info!("Abrindo: {}", path.display());
        match get_pdfium().and_then(|p| PdfEngine::open(&path, p)) {
            Ok(engine) => {
                let page_count = engine.page_count();
                self.page_count   = page_count;

                // Extrai bookmarks (TOC) antes de mover engine
                self.bookmarks = engine.extract_bookmarks();

                self.engine       = Some(engine);
                self.current_file = Some(path.clone());
                self.render_error = None;
                self.viewer.reset();

                // Inicia cache de thumbnails (thread dedicada maripdf-thumbs)
                self.thumb_cache = Some(ThumbCache::new(path.clone(), page_count));

                // Inicia o cache de background quando em modo Contínuo
                if self.viewer.mode == ViewMode::Continuous {
                    let scale = self.viewer.zoom * 1.5;
                    self.viewer.init_cache(path.clone(), scale);
                }

                // Restaura sessão anterior (se houver)
                if let Some(session) = self.persistence.get(&path) {
                    self.viewer.zoom  = session.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                    self.current_page = session.last_page.min(page_count.saturating_sub(1));
                } else {
                    self.current_page = 0;
                    self.viewer.zoom  = 1.0;
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

    fn save_session(&mut self) {
        if let Some(ref path) = self.current_file.clone() {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            self.persistence.save(path, FileSession {
                last_page:   self.current_page,
                zoom:        self.viewer.zoom,
                last_opened: now_unix,
            });
        }
    }

    // ── Toast ───────────────────────────────────────────────────────────

    #[allow(dead_code)]
    fn set_toast(&mut self, ctx: &egui::Context, msg: impl Into<String>, secs: f64) {
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast::error(msg, secs, now));
    }

    #[allow(dead_code)]
    fn set_info_toast(&mut self, ctx: &egui::Context, msg: impl Into<String>, secs: f64) {
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast::info(msg, secs, now));
    }

    fn ui_toast(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if let Some(ref t) = self.toast.clone() {
            if t.is_alive(now) {
                let alpha = t.alpha(now);
                let msg   = t.message.clone();

                let (bg, stroke_c, icon, icon_c, text_c) = match t.kind {
                    ToastKind::Error => (
                        egui::Color32::from_rgba_unmultiplied(35, 12, 12,    (245.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(180, 50, 50,   (200.0 * alpha) as u8),
                        "⚠",
                        egui::Color32::from_rgba_unmultiplied(239, 100, 80,  (255.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(230, 200, 200, (255.0 * alpha) as u8),
                    ),
                    ToastKind::Info => (
                        egui::Color32::from_rgba_unmultiplied(15, 20, 35,    (245.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(120, 80, 220,  (200.0 * alpha) as u8),
                        "✓",
                        egui::Color32::from_rgba_unmultiplied(167, 139, 250, (255.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(210, 200, 240, (255.0 * alpha) as u8),
                    ),
                };

                egui::Area::new(egui::Id::new("toast"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(bg)
                            .stroke(egui::Stroke::new(1.0, stroke_c))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(icon)
                                        .size(14.0).color(icon_c));
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new(&msg)
                                        .size(12.0).color(text_c));
                                });
                            });
                    });
                ctx.request_repaint();
            } else {
                self.toast = None;
            }
        }
    }

    // ── Renderização ──────────────────────────────────────────────────────────

    fn render_current_page(&mut self, ctx: &egui::Context) {
        self.viewer.is_loading = true;
        let Some(engine) = &self.engine else { self.viewer.is_loading = false; return };
        match engine.render_page(self.current_page, self.viewer.zoom * 1.5) {
            Ok(mut rendered) => {
                // Dark mode: inverte RGB (mantém Alpha) para PDF "negativo"
                if matches!(self.color_mode, ColorMode::Dark) {
                    invert_rgba(&mut rendered.rgba);
                }
                self.viewer.load_page(ctx, &rendered, self.current_page);
                self.render_error = None;
            }
            Err(e) => {
                tracing::error!("{e}");
                self.render_error = Some(format!("{e}"));
            }
        }
        self.viewer.is_loading = false;
    }

    fn go_to_page(&mut self, ctx: &egui::Context, page: usize) {
        let page = page.min(self.page_count.saturating_sub(1));
        if self.current_page == page && self.viewer.texture.is_some() { return; }
        self.current_page = page;
        if self.viewer.mode == ViewMode::SinglePage {
            self.render_current_page(ctx);
        }
    }

    fn apply_zoom(&mut self, ctx: &egui::Context, delta: f32) {
        let new_zoom = (self.viewer.zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_zoom - self.viewer.zoom).abs() > 0.005 {
            self.viewer.zoom = new_zoom;
            self.invalidate_render(ctx);
        }
    }

    fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if (self.viewer.zoom - zoom).abs() > 0.005 {
            self.viewer.zoom = zoom;
            self.invalidate_render(ctx);
        }
    }

    fn invalidate_render(&mut self, ctx: &egui::Context) {
        if self.viewer.mode == ViewMode::SinglePage {
            self.viewer.texture = None;
            self.render_current_page(ctx);
        } else {
            // Limpa ambos os caches; reinicia o background cache se tiver arquivo
            self.viewer.tex_cache.clear();
            self.viewer.page_cache.clear();
            if let Some(path) = self.current_file.clone() {
                let scale = self.viewer.zoom * 1.5;
                self.viewer.init_cache(path, scale);
            }
        }
    }

    // ── Fit Width / Fit Page ──────────────────────────────────────────────────

    /// Calcula zoom para a página preencher a largura disponível
    fn fit_width(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        // Estimamos tamanho da página a zoom=1.0 (renderizada a 1.5x de escala interna)
        if let Ok(rendered) = engine.render_page(self.current_page, 1.5) {
            // largura útil: available_width menos margens (48px)
            let usable = (self.available_width - 48.0).max(100.0);
            // a página a zoom=1.0 tem rendered.width pixels (a 1.5x scale factor)
            // logo, pixels reais da página = rendered.width / 1.5
            let page_w_logical = rendered.width as f32 / 1.5;
            let new_zoom = usable / page_w_logical;
            self.set_zoom(ctx, new_zoom);
        }
    }

    /// Calcula zoom para a página inteira caber na tela
    fn fit_page(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        if let Ok(rendered) = engine.render_page(self.current_page, 1.5) {
            let usable_w = (self.available_width  - 48.0).max(100.0);
            let usable_h = ctx.screen_rect().height() - 48.0; // descontamos toolbar
            let page_w   = rendered.width  as f32 / 1.5;
            let page_h   = rendered.height as f32 / 1.5;
            let zoom_w   = usable_w / page_w;
            let zoom_h   = usable_h / page_h;
            self.set_zoom(ctx, zoom_w.min(zoom_h));
        }
    }
}

impl eframe::App for MariApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_session();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        configure_visuals(ctx);

        // ── Fechar documento pendente (via botão Voltar) ─────────────────────
        if self.pending_close {
            self.pending_close = false;
            self.current_file = None;
            self.engine       = None;
            self.viewer.reset();
            self.render_error = None;
            self.search.clear();
            self.search_open  = false;
            self.goto_open    = false;
            self.page_links   = Vec::new();
            self.bookmarks    = Vec::new();
            self.thumb_cache  = None;
            self.side_panel   = SidePanelKind::None;
        }

        // ── Drena bitmaps prontos do ThumbCache ──────────────────────────────
        if let Some(ref mut tc) = self.thumb_cache {
            if tc.drain_ready() {
                ctx.request_repaint();
            }
        }

        // ── Arquivo pendente ─────────────────────────────────────────────
        if let Some(path) = self.pending_open.take() {
            let ctx2 = ctx.clone();
            self.open_file(&ctx2, path);
        }

        // ── Rerender pendente (troca de tema) ────────────────────────────
        if self.pending_rerender && self.engine.is_some() {
            self.pending_rerender = false;
            let ctx2 = ctx.clone();
            self.render_current_page(&ctx2);
        }

        // ── Drag & drop ──────────────────────────────────────────────────
        ctx.input(|i| {
            self.drop_zone_hovered = !i.raw.hovered_files.is_empty();
            if let Some(f) = i.raw.dropped_files.first() {
                if let Some(p) = f.path.clone() {
                    if p.extension().map(|e| e == "pdf").unwrap_or(false) {
                        self.pending_open = Some(p);
                    }
                }
            }
        });

        // ── Atalhos globais ──────────────────────────────────────────────
        let mut go_next     = false;
        let mut go_prev     = false;
        let mut zoom_in     = false;
        let mut zoom_out    = false;
        let mut go_home     = false;
        let mut go_end      = false;
        let mut toggle_fs   = false;
        let mut fit_w       = false;
        let mut fit_p       = false;
        let mut open_goto   = false;
        let mut toggle_mode = false;
        let mut open_search = false;
        let mut search_next = false;
        let mut search_prev = false;
        let mut escape      = false;
        let mut select_all  = false;
        let mut copy_text   = false;

        ctx.input(|i| {
            let ctrl  = i.modifiers.ctrl;
            let shift = i.modifiers.shift;

            go_next     = i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown);
            go_prev     = i.key_pressed(egui::Key::ArrowLeft)  || i.key_pressed(egui::Key::PageUp);
            go_home     = i.key_pressed(egui::Key::Home);
            go_end      = i.key_pressed(egui::Key::End);
            toggle_fs   = i.key_pressed(egui::Key::F11);
            zoom_in     = ctrl && (i.key_pressed(egui::Key::Plus)  || i.key_pressed(egui::Key::Equals));
            zoom_out    = ctrl && i.key_pressed(egui::Key::Minus);
            fit_w       = i.key_pressed(egui::Key::W) && !ctrl;
            fit_p       = ctrl && i.key_pressed(egui::Key::Num0);
            open_goto   = ctrl && i.key_pressed(egui::Key::G);
            toggle_mode = ctrl && shift && i.key_pressed(egui::Key::M);
            open_search = ctrl && i.key_pressed(egui::Key::F);
            search_next = i.key_pressed(egui::Key::F3) && !shift;
            search_prev = i.key_pressed(egui::Key::F3) && shift;
            escape      = i.key_pressed(egui::Key::Escape);
            select_all  = ctrl && i.key_pressed(egui::Key::A);
            copy_text   = ctrl && i.key_pressed(egui::Key::C);
        });

        // ── Atalho de tela cheia ──────────────────────────────────────────
        if toggle_fs {
            self.fullscreen = !self.fullscreen;
            let vp = egui::ViewportCommand::Fullscreen(self.fullscreen);
            ctx.send_viewport_cmd(vp);
        }

        // ── Sinais do viewer (zoom_delta, page_delta) ────────────────────
        let zoom_delta = std::mem::replace(&mut self.viewer.zoom_delta, 0.0);
        let page_delta = std::mem::replace(&mut self.viewer.page_delta, 0);

        if zoom_delta != 0.0 { self.apply_zoom(ctx, zoom_delta); }
        if zoom_in           { self.apply_zoom(ctx,  0.15); }
        if zoom_out          { self.apply_zoom(ctx, -0.15); }

        // ── Overlay Ctrl+G ──────────────────────────────────────────
        if open_goto {
            if self.search_open { self.search_open = false; }
            self.goto_open = !self.goto_open;
            self.goto_input.clear();
        }

        // ── Overlay Ctrl+F ──────────────────────────────────────────
        if open_search && self.engine.is_some() {
            if self.goto_open { self.goto_open = false; }
            self.search_open    = !self.search_open;
            self.search_focused = self.search_open;
            if !self.search_open { self.search.clear(); }
        }
        if search_next && self.search_open { self.search.go_next(); }
        if search_prev && self.search_open { self.search.go_prev(); }
        if escape { self.goto_open = false; self.search_open = false; self.search.clear(); }

        // Navega para a página do hit ativo ao navegar nos resultados
        if (search_next || search_prev) && self.search_open {
            if let Some(hit) = self.search.current() {
                let pg = hit.page_index;
                let c = ctx.clone();
                self.go_to_page(&c, pg);
            }
        }

        // ── Fit Width / Fit Page ──────────────────────────────────────────
        if fit_w { let c = ctx.clone(); self.fit_width(&c); }
        if fit_p { let c = ctx.clone(); self.fit_page(&c); }

        if self.engine.is_some() {
            if page_delta != 0 && self.viewer.mode == ViewMode::SinglePage {
                let p = (self.current_page as i64 + page_delta as i64)
                    .clamp(0, self.page_count as i64 - 1) as usize;
                let c = ctx.clone(); self.go_to_page(&c, p);
            }
            if go_next && self.current_page + 1 < self.page_count {
                let p = self.current_page + 1; let c = ctx.clone(); self.go_to_page(&c, p);
            }
            if go_prev && self.current_page > 0 {
                let p = self.current_page - 1; let c = ctx.clone(); self.go_to_page(&c, p);
            }
            if go_home { let c = ctx.clone(); self.go_to_page(&c, 0); }
            if go_end  { let last = self.page_count.saturating_sub(1); let c = ctx.clone(); self.go_to_page(&c, last); }
            if toggle_mode { let c = ctx.clone(); self.do_toggle_mode(&c); }
        }

        // ── Seleção de texto Ctrl+A ───────────────────────────────────────
        if select_all && self.engine.is_some() {
            let now = ctx.input(|i| i.time);
            if let Some(engine) = &self.engine {
                match engine.extract_text(self.current_page) {
                    Ok(text) if !text.trim().is_empty() => {
                        self.selected_text = Some(text);
                        self.toast = Some(Toast::info("Texto selecionado — Ctrl+C para copiar", 3.0, now));
                    }
                    Ok(_) => {
                        self.toast = Some(Toast::error("Página sem texto extraiível", 2.5, now));
                    }
                    Err(_) => {
                        self.toast = Some(Toast::error("Falha ao extrair texto", 2.5, now));
                    }
                }
            }
        }

        // ── Ctrl+C — copia selected_text para clipboard ───────────────────
        if copy_text {
            if let Some(ref text) = self.selected_text.clone() {
                let now = ctx.input(|i| i.time);
                ctx.copy_text(text.clone());
                self.toast = Some(Toast::info("Texto copiado!", 2.0, now));
            }
        }

        // ── Cache de links da página atual ────────────────────────────────
        if self.engine.is_some() && self.links_page != self.current_page {
            self.page_links = self.engine
                .as_ref()
                .map(|e| e.extract_page_links(self.current_page))
                .unwrap_or_default();
            self.links_page = self.current_page;
        }

        // ── Click em link — abre URI no browser ───────────────────────────
        if !self.page_links.is_empty() {
            let maybe_click = ctx.input(|i| {
                if i.pointer.button_clicked(egui::PointerButton::Primary) {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            });

            if let Some(click_pos) = maybe_click {
                let page_rect = self.viewer.last_page_rect;
                if page_rect.is_positive() && page_rect.contains(click_pos) {
                    // Se a página só tem 1 link, abre direto; senão precisaria link_at_point
                    if self.page_links.len() == 1 {
                        let uri = self.page_links[0].uri.clone();
                        let _ = open::that(uri);
                    }
                }
            }
        }

        // ── Renderiza UI ──────────────────────────────────────────────────
        if self.current_file.is_none() {
            self.ui_home(ctx);
        } else {
            self.ui_reader(ctx);
        }

        // ── Overlay Goto Page ─────────────────────────────────────────────
        if self.goto_open && self.engine.is_some() {
            self.ui_goto_overlay(ctx);
        }

        // ── Overlay Busca (Ctrl+F) ────────────────────────────────────────
        self.ui_toast(ctx);
        if self.search_open && self.engine.is_some() {
            self.ui_search_bar(ctx);
        }

        // ── Painel Configurações ──
        if self.settings_open {
            let mut needs_font   = false;
            let mut needs_render = false;
            crate::ui::settings_panel::show_settings_panel(
                ctx,
                &mut self.settings_open,
                &mut self.settings,
                &mut self.color_mode,
                &mut needs_font,
                &mut needs_render,
            );
            if needs_font {
                crate::apply_font_settings(ctx, self.settings.font);
                self.settings.save();
            }
            if needs_render {
                self.invalidate_render(ctx);
                self.settings.save();
            }
        }
    }
}

impl MariApp {
    // ── Tela Inicial ──────────────────────────────────────────────────────────

    fn ui_home(&mut self, ctx: &egui::Context) {
        let cm         = self.color_mode;
        let pal        = self.settings.palette.0;
        let bg         = cm.bg_for(pal);
        let surf       = cm.surface_for(pal);
        let bord       = cm.border();
        let text       = cm.text_primary();
        let muted      = cm.text_muted();
        let dw         = cm.divider_width();

        let recents = self.persistence.recent_files(8);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg))
            .show(ctx, |ui| {
                // Largura máxima do conteúdo principal
                let total_w = ui.available_width();
                let total_h = ui.available_height();
                let content_w = (total_w * 0.80).min(640.0).max(360.0);
                let side_pad = (total_w - content_w) / 2.0;

                // ── Espaço topo ───────────────────────────────────────────
                let top_space = if recents.is_empty() {
                    (total_h * 0.25).max(80.0)
                } else {
                    (total_h * 0.14).max(40.0)
                };
                ui.add_space(top_space);

                // ── Logo ──────────────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    // Outfit Black via família "Title"
                    ui.label(egui::RichText::new("MariPDF")
                        .size(46.0)
                        .color(ACCENT_PRIMARY)
                        .family(egui::FontFamily::Name("Title".into())));
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Leitor PDF ultra-leve para Windows")
                        .size(13.0)
                        .color(muted));
                });

                // ── Botão CTA principal ───────────────────────────────────
                ui.vertical_centered(|ui| {
                    let btn_w = (content_w * 0.55).min(280.0).max(200.0);
                    ui.set_max_width(btn_w);

                    // ── Botão CTA com hover animado ───────────────────────
                    let cta_id     = egui::Id::new("home_open_btn");
                    let cta_hov_id = cta_id.with("hov");
                    let was_h: bool = ctx.data(|d| d.get_temp(cta_hov_id).unwrap_or(false));
                    let t_cta = ctx.animate_bool_with_time(cta_id, was_h, 0.12);

                    // Escurece o accent gradualmente no hover
                    let darken_cta = (t_cta * 24.0) as u8;
                    let cta_bg = egui::Color32::from_rgb(
                        ACCENT_PRIMARY.r().saturating_sub(darken_cta),
                        ACCENT_PRIMARY.g().saturating_sub(darken_cta),
                        ACCENT_PRIMARY.b().saturating_sub(darken_cta),
                    );

                    let cta_resp = ui.add_sized(
                        [btn_w, 44.0],
                        egui::Button::new(
                            egui::RichText::new("Abrir PDF…")
                                .size(15.0)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(cta_bg)
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(12)),
                    );

                    let is_h_cta = cta_resp.hovered();
                    ctx.data_mut(|d| d.insert_temp(cta_hov_id, is_h_cta));
                    if is_h_cta {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        ctx.request_repaint();
                    }
                    if cta_resp.clicked() {
                        self.open_file_dialog();
                    }


                    ui.add_space(10.0);

                    // Hint de drag
                    let drag_color = if self.drop_zone_hovered {
                        ACCENT_LIGHT
                    } else {
                        cm.text_subtle()
                    };
                    let drag_text = if self.drop_zone_hovered {
                        "▼  Solte o PDF aqui"
                    } else {
                        "ou arraste um PDF para qualquer lugar"
                    };
                    ui.label(egui::RichText::new(drag_text)
                        .size(11.5)
                        .color(drag_color));
                });

                // ── Erros ─────────────────────────────────────────────────
                if let Some(ref err) = self.render_error.clone() {
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(format!("⚠ {err}"))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(239, 68, 68)));
                    });
                }

                // ── Recentes ─────────────────────────────────────────────
                if !recents.is_empty() {
                    ui.add_space(32.0);

                    // Título seção
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        ui.label(egui::RichText::new("RECENTES")
                            .size(10.0)
                            .color(muted)
                            .strong());
                    });

                    ui.add_space(8.0);

                    // Linha separadora
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        ui.add(
                            egui::Separator::default()
                                .horizontal()
                                .grow(-(side_pad * 2.0))
                                .spacing(dw),
                        );
                    });

                    ui.add_space(8.0);

                    let mut open_path:   Option<std::path::PathBuf> = None;
                    let mut remove_path: Option<std::path::PathBuf> = None;

                    for (path, session) in &recents {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("PDF")
                            .trim_end_matches(".pdf")
                            .trim_end_matches(".PDF");

                        let dir = path
                            .parent()
                            .and_then(|p| p.to_str())
                            .unwrap_or("");

                        let dir_display = if dir.len() > 55 {
                            format!("…{}", &dir[dir.len() - 52..])
                        } else {
                            dir.to_string()
                        };

                        let pg_label = format!("p.{}", session.last_page + 1);

                        let mut x_btn_rect = egui::Rect::NOTHING;
                        ui.horizontal(|ui| {
                            ui.add_space(side_pad);

                            let card = egui::Frame::new()
                                .fill(surf)
                                .stroke(egui::Stroke::new(dw, bord))
                                .corner_radius(egui::CornerRadius::same(10))
                                .inner_margin(egui::Margin::symmetric(14, 10));

                            let ir = card.show(ui, |ui| {
                                ui.set_min_width(content_w - 2.0);
                                ui.set_max_width(content_w - 2.0);
                                ui.horizontal(|ui| {
                                    // Bullet colorido (PDF)
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(
                                            ui.cursor().min + egui::vec2(0.0, 6.0),
                                            egui::vec2(3.0, 26.0),
                                        ),
                                        egui::CornerRadius::same(2),
                                        egui::Color32::from_rgb(220, 80, 60),
                                    );
                                    ui.add_space(10.0);

                                    let pills_w = 70.0_f32;

                                    // ── Coluna de texto ──────────────────────────
                                    let text_w = (content_w - 2.0 - 3.0 - 10.0 - pills_w - 16.0).max(60.0);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(text_w, 0.0),
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            ui.set_max_width(text_w);
                                            ui.label(
                                                egui::RichText::new(name)
                                                    .size(13.5)
                                                    .color(text)
                                                    .strong(),
                                            );
                                            ui.add_space(1.0);
                                            ui.label(
                                                egui::RichText::new(&dir_display)
                                                    .size(10.5)
                                                    .color(muted),
                                            );
                                        },
                                    );

                                    // ── Direita: página + botão X ────────────────
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                             let x_id = ui.id().with(("xbtn", path.as_path()));
                                             let x_hv = x_id.with("h");
                                             let x_was: bool = ctx.data(|d| d.get_temp(x_hv).unwrap_or(false));
                                             let xt = ctx.animate_bool_with_time(x_id, x_was, 0.10);
                                             let x_bg = egui::Color32::from_rgba_unmultiplied(200, 60, 50, (xt * 55.0) as u8);
                                             let x_fg = if xt > 0.1 { egui::Color32::from_rgb(255, 130, 110) } else { egui::Color32::from_rgb(200, 80, 60) };
                                             let x_btn = ui.add(
                                                  egui::Button::new(egui::RichText::new("\u{E4F8}").size(18.0).color(x_fg).family(egui::FontFamily::Name("Icons".into())))
                                                 .fill(x_bg)
                                                 .stroke(egui::Stroke::NONE)
                                                 .corner_radius(egui::CornerRadius::same(6))
                                                 .min_size(egui::vec2(34.0, 34.0)),
                                             );
                                              x_btn_rect = x_btn.rect;
                                              let x_is = x_btn.hovered();
                                              ctx.data_mut(|d| d.insert_temp(x_hv, x_is));
                                              if x_is { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); ctx.request_repaint(); }
                                              if x_btn.clicked() { remove_path = Some(path.clone()); }
                                              ui.add_space(6.0);
                                             ui.label(
                                                 egui::RichText::new(&pg_label)
                                                     .size(10.5)
                                                     .color(muted)
                                                     .monospace(),
                                             );
                                         },
                                     );
                                 })
                             });

                             // Card click — raw input, sem consumir o click do botão X
                             let card_rect = ir.response.rect;
                             let (hovered, clicked, click_pos) = ctx.input(|i| {
                                 let pos = i.pointer.hover_pos().unwrap_or_default();
                                 let released = i.pointer.any_released();
                                 let interact = i.pointer.interact_pos().unwrap_or_default();
                                 (card_rect.contains(pos), released && card_rect.contains(interact), interact)
                             });
                             if hovered && !x_btn_rect.contains(
                                 ctx.input(|i| i.pointer.hover_pos().unwrap_or_default())
                             ) {
                                 ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                             }
                             if clicked && !x_btn_rect.contains(click_pos) && remove_path.is_none() {
                                 open_path = Some(path.clone());
                             }
                         });
                        ui.add_space(5.0);
                    }

                    if let Some(p) = open_path {
                        self.pending_open = Some(p);
                    }
                    if let Some(p) = remove_path {
                        self.persistence.remove_recent(&p);
                    }
                }

                // ── Rodapé ────────────────────────────────────────────────
                let remaining = ui.available_height();
                ui.add_space(remaining.max(16.0) - 16.0);
                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    ui.label(egui::RichText::new(format!(
                        "v{}  ·  Ctrl+F: Buscar  ·  Ctrl+G: Ir à página  ·  W: Fit Width  ·  F11: Tela cheia",
                        self.version
                    ))
                    .size(10.0)
                    .color(cm.text_subtle()));
                });
            });


            // -- Icone de tema (canto superior direito) ---
            {
                let screen_rect = ctx.screen_rect();
                let area_pos = egui::pos2(screen_rect.right() - 52.0, screen_rect.top() + 12.0);
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("home_area_pos"), area_pos));
                // cores frescas do modo atual (nao usa surf/bord do frame anterior)
                let btn_surf = self.color_mode.surface();
                let btn_bord = match self.color_mode {
                    ColorMode::Dark  => egui::Color32::from_rgb(55, 55, 65),
                    ColorMode::Light => egui::Color32::from_rgb(200, 195, 185),
                    ColorMode::Sepia => egui::Color32::from_rgb(170, 140, 100),
                };

                // Cor adaptativa: escura em fundos claros, laranja em Dark
                let icon_color = match self.color_mode {
                    ColorMode::Dark  => ACCENT_LIGHT,
                    ColorMode::Light => egui::Color32::from_rgb(80, 60, 20),
                    ColorMode::Sepia => egui::Color32::from_rgb(90, 55, 10),
                };

                let icon_text = match self.color_mode {
                    ColorMode::Dark  => icons::ph(icons::MOON, 20.0, icon_color),
                    ColorMode::Light => icons::ph(icons::SUN, 20.0, icon_color),
                    ColorMode::Sepia => icons::ph(icons::LAMP_PENDANT, 20.0, icon_color), // Lâmpada de gênio
                };

                egui::Area::new(egui::Id::new("home_theme_btn"))
                    .fixed_pos(area_pos)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(btn_surf)
                            .stroke(egui::Stroke::new(1.4, btn_bord))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(4))
                            .show(ui, |ui| {
                                let resp = ui.add(
                                    egui::Button::new(icon_text)
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(28.0, 28.0)),
                                )
                                .on_hover_text(self.color_mode.label());

                                if resp.clicked() {
                                    self.color_mode = self.color_mode.next();
                                    self.viewer.tex_cache.clear();
                                    self.viewer.texture    = None;
                                    self.viewer.loaded_page = None;
                                    self.pending_rerender  = true;
                                }
                                if resp.hovered() {
                                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });

                    });
            }

            // Botão Settings — posicionado à esquerda do botão de tema
            if let Some(ap) = ctx.data(|d| d.get_temp::<egui::Pos2>(egui::Id::new("home_area_pos"))) {
                egui::Area::new(egui::Id::new("home_settings_btn"))
                    .fixed_pos(egui::pos2(ap.x - 44.0, ap.y))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let hid = egui::Id::new("home_settings_hover");
                        let hov = ui.ctx().data(|d| d.get_temp::<bool>(hid).unwrap_or(false));
                        egui::Frame::new()
                            .fill(if hov { cm.surface() } else { egui::Color32::TRANSPARENT })
                            .stroke(egui::Stroke::new(1.1, if hov { cm.border() } else { egui::Color32::TRANSPARENT }))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(4))
                            .show(ui, |ui| {
                                let r = ui.add(
                                    egui::Button::new(icons::ph(icons::GEAR, 20.0, cm.text_muted()))
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::NONE)
                                        .min_size(egui::vec2(28.0, 28.0)),
                                ).on_hover_text("Configurações");
                                ui.ctx().data_mut(|d| d.insert_temp(hid, r.hovered()));
                                if r.hovered() { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                                if r.clicked() { self.settings_open = !self.settings_open; }
                            });
                    });
            }
    }

    /// draw_drop_zone: sem uso de ícones Phosphor — mantém API para DnD hover
    #[allow(dead_code)]
    fn draw_drop_zone(&mut self, _ui: &mut egui::Ui, _surf: egui::Color32, _bord: egui::Color32, _text: egui::Color32, _w: f32) {
        // Layout simplificado — drop zone agora é inline na ui_home
    }

    // ── Reader ────────────────────────────────────────────────────────────────
    fn ui_reader(&mut self, ctx: &egui::Context) {
        let path = self.current_file.clone().unwrap();
        let filename = path.file_name()
            .and_then(|n| n.to_str()).unwrap_or("Documento").to_string();
        let page_count   = self.page_count;
        let current_page = self.current_page;
        let zoom_pct     = (self.viewer.zoom * 100.0) as u32;
        let mode         = self.viewer.mode;
        let color_mode   = self.color_mode;
        let fs           = self.fullscreen;

        // ── Toolbar fade: anima opacidade baseada na posição do mouse ────────
        {
            let mouse_y = ctx.input(|i| i.pointer.hover_pos().map(|p| p.y).unwrap_or(999.0));
            // Zona de ativação: mouse nos 52px superiores
            let target_opacity: f32 = if mouse_y < 52.0 { 1.0 } else { 0.18 };
            let dt = ctx.input(|i| i.unstable_dt).min(0.1);
            let speed = if target_opacity > self.toolbar_opacity { 10.0 } else { 4.0 };
            self.toolbar_opacity += (target_opacity - self.toolbar_opacity) * speed * dt;
            self.toolbar_opacity = self.toolbar_opacity.clamp(0.0, 1.0);
            if (self.toolbar_opacity - target_opacity).abs() > 0.005 {
                ctx.request_repaint();
            }
        }
        let tb_alpha = (self.toolbar_opacity * 255.0) as u8;
        let dw = color_mode.divider_width();

        // ── Toolbar ───────────────────────────────────────────────────────────
        if !fs {
            egui::TopBottomPanel::top("toolbar")
                .frame(egui::Frame::new()
                    .fill({
                        let s = color_mode.surface_for(self.settings.palette.0);
                        egui::Color32::from_rgba_premultiplied(s.r(), s.g(), s.b(),
                            ((s.a() as f32) * self.toolbar_opacity) as u8)
                    })
                    .stroke(egui::Stroke::new(dw, egui::Color32::from_rgba_premultiplied(
                        color_mode.border().r(),
                        color_mode.border().g(),
                        color_mode.border().b(),
                        tb_alpha,
                    )))
                    .inner_margin(egui::Margin::symmetric(12, 5)))
                .show(ctx, |ui| {
                    // ── Hover color para os botões icon do toolbar ────────────
                    let palette_idx = self.settings.palette.0;
                    let accent      = color_mode.accent_for(palette_idx);
                    let hover_fill  = egui::Color32::from_rgba_unmultiplied(
                        accent.r(), accent.g(), accent.b(), 18,
                    );
                    let hover_fill_strong = egui::Color32::from_rgba_unmultiplied(
                        accent.r(), accent.g(), accent.b(), 35,
                    );

                    // Helper: botão icon simples com hover bg
                    let make_icon_btn = |ic: &str, size: f32| -> egui::Button<'_> {
                        egui::Button::new(icons::ph(ic, size, color_mode.text_muted()))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                    };

                    ui.horizontal(|ui| {
                        // ← Voltar
                        let r = ui.add(
                            egui::Button::new(icons::ph(icons::ARROW_LEFT, 17.0, color_mode.text_muted()))
                                .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE),
                        ).on_hover_text("Voltar (fecha PDF)");
                        if r.hovered() {
                            ui.painter().rect_filled(r.rect, egui::CornerRadius::same(5), hover_fill_strong);
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if r.clicked() {
                            self.save_session();
                            self.pending_close = true;
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // ── Botões Side Panel ──
                        let toc_color = if self.side_panel == SidePanelKind::Toc {
                            accent
                        } else {
                            color_mode.text_muted()
                        };
                        let thumb_color = if self.side_panel == SidePanelKind::Thumbs {
                            accent
                        } else {
                            color_mode.text_muted()
                        };

                        let r_toc = ui.add(
                            egui::Button::new(icons::ph(icons::LIST, 16.0, toc_color))
                                .fill(if self.side_panel == SidePanelKind::Toc {
                                    egui::Color32::from_rgba_unmultiplied(accent.r(),accent.g(),accent.b(),20)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .stroke(egui::Stroke::NONE),
                        ).on_hover_text("Índice / TOC");
                        if r_toc.hovered() && self.side_panel != SidePanelKind::Toc {
                            ui.painter().rect_filled(r_toc.rect, egui::CornerRadius::same(5), hover_fill);
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if r_toc.clicked() {
                            self.side_panel = if self.side_panel == SidePanelKind::Toc {
                                SidePanelKind::None
                            } else { SidePanelKind::Toc };
                        }

                        let r_th = ui.add(
                            egui::Button::new(icons::ph(icons::SQUARES_FOUR, 16.0, thumb_color))
                                .fill(if self.side_panel == SidePanelKind::Thumbs {
                                    egui::Color32::from_rgba_unmultiplied(accent.r(),accent.g(),accent.b(),20)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .stroke(egui::Stroke::NONE),
                        ).on_hover_text("Miniaturas de página");
                        if r_th.hovered() && self.side_panel != SidePanelKind::Thumbs {
                            ui.painter().rect_filled(r_th.rect, egui::CornerRadius::same(5), hover_fill);
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if r_th.clicked() {
                            self.side_panel = if self.side_panel == SidePanelKind::Thumbs {
                                SidePanelKind::None
                            } else { SidePanelKind::Thumbs };
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Nome do arquivo (truncado)
                        let name = if filename.len() > 40 {
                            format!("…{}", &filename[filename.len()-38..])
                        } else { filename.clone() };
                        ui.label(egui::RichText::new(name).size(13.0).color(color_mode.text_primary()));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // ── Modo de cor ──
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new(color_mode.label()).size(11.0).color(color_mode.text_muted()))
                                    .fill(color_mode.statusbar_for(palette_idx))
                                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                                    .corner_radius(egui::CornerRadius::same(5)),
                            ).on_hover_text("Alternar modo de cor").clicked() {
                                self.color_mode = self.color_mode.next();
                                self.viewer.tex_cache.clear();
                                self.viewer.texture = None;
                                self.viewer.loaded_page = None;
                                self.pending_rerender = true;
                            }

                            ui.add_space(2.0);

                            // ── Botão Configurações ──
                            let resp_settings = ui.add(
                                egui::Button::new(icons::ph(icons::GEAR, 15.0, color_mode.text_muted()))
                                    .fill(color_mode.statusbar_for(palette_idx))
                                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                                    .corner_radius(egui::CornerRadius::same(5)),
                            ).on_hover_text("Configurações");
                            if resp_settings.hovered() { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                            if resp_settings.clicked() { self.settings_open = !self.settings_open; }

                            ui.add_space(4.0);

                            // ── Toggle Single/Contínuo ──
                            let (mode_icon, hint_mode) = match mode {
                                ViewMode::SinglePage => (icons::SQUARE_HALF, "Contínuo (Ctrl+Shift+M)"),
                                ViewMode::Continuous => (icons::ROWS,        "Página única (Ctrl+Shift+M)"),
                            };
                            if ui.add(
                                egui::Button::new(icons::ph(mode_icon, 15.0, color_mode.text_muted()))
                                    .fill(color_mode.statusbar_for(palette_idx))
                                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                                    .corner_radius(egui::CornerRadius::same(5)),
                            ).on_hover_text(hint_mode).clicked() {
                                let c = ctx.clone(); self.do_toggle_mode(&c);
                            }

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // ── Fit Width / Fit Page ──
                            let r_fw = ui.add(make_icon_btn(icons::FRAME_CORNERS, 15.0))
                                .on_hover_text("Fit Page (Ctrl+0)");
                            if r_fw.hovered() {
                                ui.painter().rect_filled(r_fw.rect, egui::CornerRadius::same(5), hover_fill);
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if r_fw.clicked() { let c = ctx.clone(); self.fit_page(&c); }

                            let r_fh = ui.add(make_icon_btn(icons::ARROWS_OUT, 15.0))
                                .on_hover_text("Fit Width (W)");
                            if r_fh.hovered() {
                                ui.painter().rect_filled(r_fh.rect, egui::CornerRadius::same(5), hover_fill);
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if r_fh.clicked() { let c = ctx.clone(); self.fit_width(&c); }

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // ── Zoom ──
                            ui.label(egui::RichText::new(format!("{zoom_pct}%"))
                                .size(12.0).color(color_mode.text_muted()).monospace());
                            ui.add_space(2.0);

                            let r_zm = ui.add(make_icon_btn(icons::MINUS, 15.0));
                            if r_zm.hovered() { ui.painter().rect_filled(r_zm.rect, egui::CornerRadius::same(5), hover_fill); }
                            if r_zm.clicked() { self.apply_zoom(ctx, -0.15); }

                            let r_zr = ui.add(make_icon_btn(icons::ARROW_COUNTER_CW, 15.0))
                                .on_hover_text("100%");
                            if r_zr.hovered() { ui.painter().rect_filled(r_zr.rect, egui::CornerRadius::same(5), hover_fill); }
                            if r_zr.clicked() { self.set_zoom(ctx, 1.0); }

                            let r_zp = ui.add(make_icon_btn(icons::PLUS, 15.0));
                            if r_zp.hovered() { ui.painter().rect_filled(r_zp.rect, egui::CornerRadius::same(5), hover_fill); }
                            if r_zp.clicked() { self.apply_zoom(ctx, 0.15); }

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // ── Navegação + Ctrl+G ──
                            if ui.add(
                                egui::Button::new(egui::RichText::new(
                                    format!("{}/{}", current_page + 1, page_count))
                                    .size(12.0).color(color_mode.text_muted()).monospace())
                                    .fill(color_mode.statusbar_for(palette_idx))
                                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                                    .corner_radius(egui::CornerRadius::same(4)),
                            ).on_hover_text("Ir para página (Ctrl+G)").clicked() {
                                self.goto_open = true;
                                self.goto_input.clear();
                            }
                            ui.add_space(2.0);

                            let r_cn = ui.add(make_icon_btn(icons::CARET_RIGHT, 15.0))
                                .on_hover_text("Próxima");
                            if r_cn.hovered() { ui.painter().rect_filled(r_cn.rect, egui::CornerRadius::same(5), hover_fill); }
                            if r_cn.clicked() && current_page + 1 < page_count {
                                let p = current_page + 1; let c = ctx.clone(); self.go_to_page(&c, p);
                            }

                            let r_cp = ui.add(make_icon_btn(icons::CARET_LEFT, 15.0))
                                .on_hover_text("Anterior");
                            if r_cp.hovered() { ui.painter().rect_filled(r_cp.rect, egui::CornerRadius::same(5), hover_fill); }
                            if r_cp.clicked() && current_page > 0 {
                                let p = current_page - 1; let c = ctx.clone(); self.go_to_page(&c, p);
                            }
                        });
                    });
                });
        }


        // ── Status Bar (barra inferior) ───────────────────────────────────
        if !fs {
            egui::TopBottomPanel::bottom("status_bar")
                .frame(egui::Frame::new()
                    .fill(color_mode.statusbar_for(self.settings.palette.0))
                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                    .inner_margin(egui::Margin::symmetric(14, 3)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // ── Indicador de carregamento ──
                        if self.viewer.is_loading {
                            ui.label(icons::ph(icons::SPINNER_GAP, 11.0, ACCENT_LIGHT));
                            ui.label(egui::RichText::new("Renderizando...").size(11.0).color(ACCENT_LIGHT));
                        } else {
                            ui.label(icons::ph(icons::CHECK, 11.0, color_mode.text_subtle()));
                            ui.label(egui::RichText::new("Pronto").size(11.0).color(color_mode.text_subtle()));
                        }

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // ── Nome do arquivo ──
                        let short_name = if filename.len() > 32 {
                            format!("…{}", &filename[filename.len()-30..])
                        } else { filename.clone() };
                        ui.label(egui::RichText::new(&short_name)
                            .size(11.0).color(color_mode.text_subtle()));

                        // ── Lado direito ──
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Modo
                            let mode_str = match mode {
                                ViewMode::SinglePage => "▣ Página única",
                                ViewMode::Continuous => "☰ Contínuo",
                            };
                            ui.label(egui::RichText::new(mode_str)
                                .size(11.0).color(color_mode.text_subtle()));

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // Zoom
                            ui.label(egui::RichText::new(format!("{zoom_pct}%"))
                                .size(11.0).color(color_mode.text_subtle()).monospace());

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // Página
                            ui.label(egui::RichText::new(
                                format!("Pág. {} / {}", current_page + 1, page_count))
                                .size(11.0).color(color_mode.text_muted()).monospace());
                        });
                    });
                });
        }

        // ── Side Panel (TOC ou Thumbnails) ───────────────────────────────────
        if self.side_panel != SidePanelKind::None {
            let sp_title = match self.side_panel {
                SidePanelKind::Toc    => "Índice",
                SidePanelKind::Thumbs => "Páginas",
                SidePanelKind::None   => "",
            };

            let panel_bg   = color_mode.surface_for(self.settings.palette.0);
            let panel_bord = color_mode.border();
            let panel_text = color_mode.text_primary();
            let panel_muted = color_mode.text_muted();
            let current_pg  = self.current_page;

            let mut goto_page: Option<usize> = None;

            egui::SidePanel::left("side_panel")
                .resizable(true)
                .default_width(SIDE_PANEL_WIDTH)
                .width_range(150.0..=380.0)
                .frame(
                    egui::Frame::new()
                        .fill(panel_bg)
                        .stroke(egui::Stroke::new(1.0, panel_bord))
                        .inner_margin(egui::Margin::symmetric(6, 6)),
                )
                .show(ctx, |ui| {
                    // Cabeçalho
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(sp_title)
                                .size(12.0)
                                .strong()
                                .color(panel_text),
                        );
                    });
                    ui.add(egui::Separator::default().spacing(6.0));

                    match self.side_panel {
                        SidePanelKind::Toc => {
                            let bookmarks = self.bookmarks.clone();
                            let pc = self.thumb_cache.as_ref().map_or(
                                self.bookmarks.len(), |tc| tc.page_count
                            );
                            goto_page = side_panel::show_toc(
                                ui,
                                &bookmarks,
                                current_pg,
                                pc,
                                panel_bg,
                                panel_text,
                                panel_muted,
                                ACCENT_PRIMARY,
                            );
                        }
                        SidePanelKind::Thumbs => {
                            if let Some(ref mut tc) = self.thumb_cache {
                                goto_page = side_panel::show_thumbs(
                                    ui,
                                    tc,
                                    current_pg,
                                    panel_bg,
                                    panel_muted,
                                    panel_text,
                                    ACCENT_PRIMARY,
                                    matches!(self.color_mode, ColorMode::Dark),
                                    self.color_mode.page_tint(),
                                );
                            }
                        }
                        SidePanelKind::None => {}
                    }
                });

            // Navega para a página clicada no painel
            if let Some(p) = goto_page {
                let ctx2 = ctx.clone();
                self.go_to_page(&ctx2, p);
            }
        }

        // ── Área de leitura ───────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(color_mode.bg_for(self.settings.palette.0)))
            .show(ctx, |ui| {
                // Captura largura disponível para Fit Width
                self.available_width = ui.available_width();

                if let Some(ref err) = self.render_error.clone() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.0 - 20.0);
                        ui.label(egui::RichText::new(format!("⚠ {err}"))
                            .size(14.0).color(egui::Color32::from_rgb(239, 68, 68)));
                    });
                } else if let Some(engine) = &self.engine {
                    let pc       = self.page_count;
                    let cp       = self.current_page;
                    let tint     = self.color_mode.page_tint_for(self.settings.palette.0);
                    let border   = self.color_mode.page_border();
                    // Sincroniza flag de inversão com o color_mode atual
                    self.viewer.invert_colors = matches!(self.color_mode, ColorMode::Dark);
                    let viewer_ptr = &mut self.viewer as *mut PdfViewer;
                    // SAFETY: engine e viewer são campos distintos de MariApp — sem aliasing
                    unsafe { (*viewer_ptr).show(ui, engine, cp, pc, tint, border) };

                    // ── Seleção de texto (estilo Word: linear por índice) ──────
                    // ── Seleção de texto (estilo Word) ──────────────────────
                    let page_rect = self.viewer.last_page_rect;
                    if page_rect.is_positive() {
                        // Carrega chars se a página mudou
                        if self.text_chars_page != cp {
                            self.text_chars = engine.extract_chars_with_bounds(cp)
                                .unwrap_or_default();
                            self.text_chars_page = cp;
                            self.sel_anchor = None;
                            self.sel_focus  = None;
                        }

                        if !self.text_chars.is_empty() {
                            let (pdf_w, pdf_h) = engine.page_size_pts(cp);

                            // ── Helpers de coord ──────────────────────────────
                            // Tela → PDF (normalizado → pontos)
                            let screen_to_pdf = |pos: egui::Pos2| -> (f32, f32) {
                                let nx = (pos.x - page_rect.left()) / page_rect.width().max(1.0);
                                let ny = 1.0 - (pos.y - page_rect.top()) / page_rect.height().max(1.0);
                                (nx * pdf_w, ny * pdf_h)
                            };

                            // Char mais próximo do ponto (em coords de tela)
                            let nearest_idx = |pos: egui::Pos2| -> usize {
                                let (px, py) = screen_to_pdf(pos);
                                self.text_chars
                                    .iter()
                                    .enumerate()
                                    .min_by_key(|(_, c)| {
                                        let cx = (c.left + c.right)   * 0.5;
                                        let cy = (c.bottom + c.top)   * 0.5;
                                        // Penaliza 3× mais diferença vertical → prioriza mesma linha
                                        (((cx - px).abs() + (cy - py).abs() * 3.0) * 1000.0) as i64
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(0)
                            };

                            // ── Detecção de drag ──────────────────────────────
                            let drag_resp = ui.interact(
                                page_rect,
                                egui::Id::new("text_sel_drag"),
                                egui::Sense::click_and_drag(),
                            );

                            if drag_resp.drag_started() {
                                let pos = drag_resp.interact_pointer_pos()
                                    .unwrap_or(page_rect.center());
                                self.sel_anchor    = Some(nearest_idx(pos));
                                self.sel_focus     = self.sel_anchor;
                                self.selected_text = None;
                            }
                            if drag_resp.dragged() {
                                if let Some(pos) = drag_resp.interact_pointer_pos() {
                                    self.sel_focus = Some(nearest_idx(pos));
                                }
                            }
                            if drag_resp.drag_stopped() {
                                // ao soltar: consolida texto e copia
                                if let (Some(a), Some(f)) = (self.sel_anchor, self.sel_focus) {
                                    let lo = a.min(f);
                                    let hi = (a.max(f) + 1).min(self.text_chars.len());
                                    let mut text = String::new();
                                    let mut last_r = f32::MIN;
                                    for ch in &self.text_chars[lo..hi] {
                                        if ch.left > last_r + 1.5 && !text.is_empty() {
                                            text.push(' ');
                                        }
                                        text.push(ch.ch);
                                        last_r = ch.right;
                                    }
                                    let clean = text.trim().to_string();
                                    if !clean.is_empty() {
                                        ctx.copy_text(clean.clone());
                                        self.selected_text = Some(clean);
                                        let now = ctx.input(|i| i.time);
                                        self.toast = Some(Toast::info("Texto copiado!", 2.0, now));
                                    }
                                }
                            }
                            // Clique simples → limpa
                            if drag_resp.clicked() {
                                self.sel_anchor    = None;
                                self.sel_focus     = None;
                                self.selected_text = None;
                            }
                            // Cursor I-beam
                            if drag_resp.hovered() {
                                ctx.set_cursor_icon(egui::CursorIcon::Text);
                            }

                            // ── Highlight: recalculado CADA frame (resolve resize) ──
                            if let (Some(a), Some(f)) = (self.sel_anchor, self.sel_focus) {
                                let lo = a.min(f);
                                let hi = (a.max(f) + 1).min(self.text_chars.len());

                                if lo < hi {
                                    // Converte cada char para Rect de tela (usando page_rect atual)
                                    let char_rect = |ch: &TextCharInfo| -> egui::Rect {
                                        let lx = page_rect.left() + ch.left  / pdf_w * page_rect.width();
                                        let rx = page_rect.left() + ch.right / pdf_w * page_rect.width();
                                        // PDF y cresce pra cima → inverte
                                        let ty = page_rect.bottom() - ch.top    / pdf_h * page_rect.height();
                                        let by = page_rect.bottom() - ch.bottom / pdf_h * page_rect.height();
                                        egui::Rect::from_min_max(egui::pos2(lx, ty), egui::pos2(rx, by))
                                    };

                                    // Agrupa chars por linha usando Y-center (bucket de 4px)
                                    // Cada bucket → um único rect fundido para toda a linha
                                    let mut line_rects: Vec<egui::Rect> = Vec::new();
                                    let mut cur_rect:   Option<egui::Rect> = None;
                                    let mut cur_cy:     f32 = f32::NAN;
                                    const LINE_MERGE_PX: f32 = 5.0;

                                    for ch in &self.text_chars[lo..hi] {
                                        let r = char_rect(ch);
                                        if r.width() <= 0.0 || r.height() <= 0.0 { continue; }
                                        let cy = r.center().y;

                                        if cur_rect.is_none() || (cy - cur_cy).abs() > LINE_MERGE_PX {
                                            // Nova linha — salva a anterior e começa nova
                                            if let Some(prev) = cur_rect {
                                                line_rects.push(prev);
                                            }
                                            cur_rect = Some(r);
                                            cur_cy   = cy;
                                        } else {
                                            // Mesma linha — estende o rect
                                            cur_rect = Some(cur_rect.unwrap().union(r));
                                        }
                                    }
                                    if let Some(last) = cur_rect {
                                        line_rects.push(last);
                                    }

                                    // ── Highlight (mais transparente) ────────────────
                                    let hl_color = match self.color_mode {
                                        ColorMode::Dark =>
                                            egui::Color32::from_rgba_unmultiplied(55, 130, 255, 55),
                                        ColorMode::Light | ColorMode::Sepia =>
                                            egui::Color32::from_rgba_unmultiplied(30, 100, 220, 65),
                                    };
                                    let painter = ui.painter();
                                    for r in &line_rects {
                                        painter.rect_filled(*r, egui::CornerRadius::ZERO, hl_color);
                                    }

                                    // ── Handles de seleção ────────────────────────
                                    let handle_color = egui::Color32::from_rgba_unmultiplied(55, 130, 255, 230);
                                    let handle_r     = 5.0_f32;  // raio do círculo
                                    let stem_h       = 12.0_f32; // altura da haste

                                    if let (Some(first_r), Some(last_r)) =
                                        (line_rects.first(), line_rects.last())
                                    {
                                        // ── Handle INÍCIO: topo-esquerdo do primeiro rect ─
                                        let start_pt = egui::pos2(first_r.left(), first_r.top());
                                        // Haste desce acima do círculo (acima da linha)
                                        painter.line_segment(
                                            [start_pt, egui::pos2(start_pt.x, start_pt.y - stem_h)],
                                            egui::Stroke::new(2.0, handle_color),
                                        );
                                        painter.circle_filled(
                                            egui::pos2(start_pt.x, start_pt.y - stem_h - handle_r),
                                            handle_r,
                                            handle_color,
                                        );

                                        // Área de interação para arrastar o handle de início
                                        let sh_rect = egui::Rect::from_center_size(
                                            egui::pos2(start_pt.x, start_pt.y - stem_h - handle_r),
                                            egui::vec2(handle_r * 3.5, handle_r * 3.5),
                                        );
                                        let sh_resp = ui.interact(
                                            sh_rect,
                                            egui::Id::new("sel_handle_start"),
                                            egui::Sense::drag(),
                                        );
                                        if sh_resp.dragged() {
                                            if let Some(pos) = sh_resp.interact_pointer_pos() {
                                                self.sel_anchor = Some(nearest_idx(pos));
                                            }
                                        }
                                        if sh_resp.hovered() {
                                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                        }

                                        // ── Handle FIM: base-direita do último rect ───────
                                        let end_pt = egui::pos2(last_r.right(), last_r.bottom());
                                        painter.line_segment(
                                            [end_pt, egui::pos2(end_pt.x, end_pt.y + stem_h)],
                                            egui::Stroke::new(2.0, handle_color),
                                        );
                                        painter.circle_filled(
                                            egui::pos2(end_pt.x, end_pt.y + stem_h + handle_r),
                                            handle_r,
                                            handle_color,
                                        );

                                        let eh_rect = egui::Rect::from_center_size(
                                            egui::pos2(end_pt.x, end_pt.y + stem_h + handle_r),
                                            egui::vec2(handle_r * 3.5, handle_r * 3.5),
                                        );
                                        let eh_resp = ui.interact(
                                            eh_rect,
                                            egui::Id::new("sel_handle_end"),
                                            egui::Sense::drag(),
                                        );
                                        if eh_resp.dragged() {
                                            if let Some(pos) = eh_resp.interact_pointer_pos() {
                                                self.sel_focus = Some(nearest_idx(pos));
                                            }
                                        }
                                        if eh_resp.hovered() {
                                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                        }
                                    }
                                }
                            }
                        } // end if !text_chars.is_empty()
                    }   // end if page_rect.is_positive()
                }
            });

        // ── Hint Tela Cheia ────────────────────────────────────────────────
        if fs {
            egui::Area::new(egui::Id::new("fs_hint"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("F11 → sair da tela cheia")
                        .size(11.0).color(egui::Color32::from_rgba_premultiplied(200, 200, 220, 100)));
                });
        }
    }

    // ── Overlay: Ir para Página ───────────────────────────────────────────────

    fn ui_goto_overlay(&mut self, ctx: &egui::Context) {
        let pc = self.page_count;

        egui::Area::new(egui::Id::new("goto_overlay"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG_SURFACE)
                    .stroke(egui::Stroke::new(1.5, ACCENT_PRIMARY))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        color:  egui::Color32::from_rgba_premultiplied(0, 0, 0, 120),
                        spread: 8,
                        blur:   12,
                        offset: [0, 4],
                    })
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Ir para página:")
                                .size(13.0).color(TEXT_PRIMARY));
                            ui.add_space(8.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.goto_input)
                                    .desired_width(80.0)
                                    .hint_text(format!("1–{pc}"))
                                    .char_limit(6),
                            );
                            resp.request_focus();

                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc   = ui.input(|i| i.key_pressed(egui::Key::Escape));

                            if enter {
                                if let Ok(n) = self.goto_input.trim().parse::<usize>() {
                                    let page = n.saturating_sub(1).min(pc.saturating_sub(1));
                                    let c = ctx.clone(); self.go_to_page(&c, page);
                                }
                                self.goto_open = false;
                            }
                            if esc { self.goto_open = false; }
                        });
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Enter → confirmar  •  Esc → fechar")
                            .size(10.0).color(TEXT_SUBTLE));
                    });
            });
    }

    fn ui_search_bar(&mut self, ctx: &egui::Context) {
        // Snapshot imutável do que precisamos por referência
        let total_hits  = self.search.hits.len();
        let current_idx = self.search.current_hit;
        let is_running  = self.search.is_running;
        let query_snap  = self.search.query.clone();

        let mut new_query   = query_snap.clone();
        let mut do_search   = false;
        let mut go_next     = false;
        let mut go_prev     = false;
        let mut do_close    = false;

        egui::Area::new(egui::Id::new("search_bar"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG_SURFACE)
                    .stroke(egui::Stroke::new(1.5, ACCENT_PRIMARY))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        color:  egui::Color32::from_rgba_premultiplied(0, 0, 0, 140),
                        spread: 8,
                        blur:   14,
                        offset: [0, 4],
                    })
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.horizontal(|ui| {
                            // Ícone de busca
                            ui.label(egui::RichText::new("🔍").size(13.0));
                            ui.add_space(4.0);

                            // Campo de busca
                            let edit_resp = ui.add(
                                egui::TextEdit::singleline(&mut new_query)
                                    .desired_width(180.0)
                                    .hint_text("Buscar no PDF…")
                                    .font(egui::TextStyle::Body),
                            );
                            // Auto-foca ao abrir
                            if self.search_focused {
                                edit_resp.request_focus();
                                self.search_focused = false;
                            }

                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let shift = ui.input(|i| i.modifiers.shift);
                            if enter && !shift { go_next = true; do_search = new_query != query_snap || total_hits == 0; }
                            if enter &&  shift { go_prev = true; }

                            // Contador de resultados
                            if total_hits > 0 {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(
                                    format!("{}/{}", current_idx + 1, total_hits)
                                ).size(11.0).color(TEXT_MUTED));
                            } else if is_running {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("…").size(11.0).color(TEXT_MUTED));
                            } else if !query_snap.is_empty() {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("Sem resultados")
                                    .size(11.0).color(egui::Color32::from_rgb(200, 80, 80)));
                            }

                            ui.add_space(8.0);

                            // Botão anterior ◀
                            if ui.small_button("◀").clicked() { go_prev = true; }
                            // Botão próximo ▶
                            if ui.small_button("▶").clicked() { go_next = true; }

                            ui.add_space(4.0);
                            if ui.small_button("✕").clicked() { do_close = true; }
                        });
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("Enter → próx  •  Shift+Enter → ant  •  Esc → fechar")
                            .size(9.0).color(TEXT_SUBTLE));
                    });
            });

        // ── Aplica mutações após render ───────────────────────────────────────

        // Atualiza query se mudou
        if new_query != query_snap {
            self.search.query = new_query.clone();
        }

        // Dispara busca se Enter apertado ou query mudou
        if do_search || (go_next && self.search.hits.is_empty() && !new_query.is_empty()) {
            if let Some(engine) = &self.engine {
                self.search.hits        = search_all_pages(engine, &new_query);
                self.search.current_hit = 0;
                self.search.is_running  = false;
            }
        }

        if go_next && !self.search.hits.is_empty() { self.search.go_next(); }
        if go_prev && !self.search.hits.is_empty() { self.search.go_prev(); }

        // Navega para a página do hit atual
        if (go_next || go_prev) && !self.search.hits.is_empty() {
            if let Some(hit) = self.search.current() {
                let pg = hit.page_index;
                let c = ctx.clone(); self.go_to_page(&c, pg);
            }
        }

        if do_close {
            self.search_open = false;
            self.search.clear();
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn do_toggle_mode(&mut self, ctx: &egui::Context) {
        match self.viewer.mode {
            ViewMode::SinglePage => {
                self.viewer.mode    = ViewMode::Continuous;
                self.viewer.texture = None;
                // Inicia background cache ao entrar no modo Contínuo
                if let Some(path) = self.current_file.clone() {
                    let scale = self.viewer.zoom * 1.5;
                    self.viewer.init_cache(path, scale);
                }
            }
            ViewMode::Continuous => {
                self.viewer.mode = ViewMode::SinglePage;
                self.viewer.tex_cache.clear();
                self.viewer.page_cache.clear();
                let c = ctx.clone(); self.render_current_page(&c);
            }
        }
    }
}

fn configure_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill           = BG_BASE;
    v.window_fill          = BG_SURFACE;
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.window_stroke        = egui::Stroke::new(1.0, BORDER_SUBTLE);
    v.override_text_color  = Some(TEXT_PRIMARY);

    // ── Scrollbar temática ───────────────────────────────────────────────────
    // Cor do handle (polegar) via widgets: violet suave → violet ao hover
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(65, 48, 128);
    v.widgets.hovered.bg_fill  = ACCENT_PRIMARY;
    v.widgets.active.bg_fill   = ACCENT_LIGHT;

    ctx.set_visuals(v);

    // Ajusta largura e margem da scrollbar via Style
    ctx.style_mut(|s| {
        s.spacing.scroll.bar_width        = 7.0;
        s.spacing.scroll.bar_inner_margin = 2.0;
        s.spacing.scroll.bar_outer_margin = 1.0;
    });
}

/// Converte erro do pdfium em mensagem amigável para o usuário
fn error_to_user_message(e: &anyhow::Error) -> String {
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
