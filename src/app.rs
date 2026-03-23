// app.rs — Estado global e dispatcher de UI do MariPDF
// Funções de UI extraídas para: app_color_mode.rs, app_home.rs, app_reader.rs, app_overlays.rs

use eframe::egui;
use std::path::PathBuf;
use crate::pdf::engine::{PdfEngine, PdfLinkInfo, TextCharInfo, get_pdfium, BookmarkEntry};
use crate::pdf::persistence::{FileSession, PersistenceManager};
use crate::pdf::text_search::{TextSearch};
use crate::pdf::thumb_cache::ThumbCache;
use crate::ui::viewer::{PdfViewer, ViewMode, invert_rgba};
use crate::settings::AppSettings;
pub use crate::app_color_mode::ColorMode;
pub use crate::app_reader::SidePanelKind;

// ── Paleta base (Dark) usada nas constantes de UI ────────────────────────────
pub const BG_BASE:       egui::Color32 = egui::Color32::from_rgb(14, 14, 16);
pub const BG_SURFACE:    egui::Color32 = egui::Color32::from_rgb(22, 22, 26);
pub const ACCENT_PRIMARY:egui::Color32 = egui::Color32::from_rgb(255, 106, 26);
pub const ACCENT_LIGHT:  egui::Color32 = egui::Color32::from_rgb(255, 165, 80);
pub const TEXT_PRIMARY:  egui::Color32 = egui::Color32::from_rgb(250, 250, 252);
pub const TEXT_MUTED:    egui::Color32 = egui::Color32::from_rgb(120, 120, 140);
pub const TEXT_SUBTLE:   egui::Color32 = egui::Color32::from_rgb(70, 70, 85);
pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(40, 40, 50);

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 5.0;

// ── Toast ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToastKind { Info, Error }

#[derive(Clone)]
pub struct Toast {
    message: String,
    kind:    ToastKind,
    expires: f64,
}

impl Toast {
    pub fn info(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Info, expires: now + duration_secs }
    }
    pub fn error(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Error, expires: now + duration_secs }
    }
    fn is_alive(&self, now: f64) -> bool { now < self.expires }
    fn alpha(&self, now: f64) -> f32 { ((self.expires - now).max(0.0) / 0.5).min(1.0) as f32 }
}

// ── Estado global ─────────────────────────────────────────────────────────────

pub struct MariApp {
    pub version: &'static str,

    // Documento
    pub current_file:  Option<PathBuf>,
    pub engine:        Option<PdfEngine>,
    pub current_page:  usize,
    pub page_count:    usize,
    pub render_error:  Option<String>,

    // Viewer
    pub viewer: PdfViewer,

    // UI state
    pub drop_zone_hovered: bool,
    pub fullscreen:        bool,
    pub color_mode:        ColorMode,

    // Overlay GoTo
    pub goto_open:  bool,
    pub goto_input: String,

    // Fit Width
    pub available_width: f32,

    // Persistência
    pub persistence:  PersistenceManager,
    pub pending_open: Option<PathBuf>,

    // Toast
    pub toast: Option<Toast>,

    // Busca (Ctrl+F)
    pub search:         TextSearch,
    pub search_open:    bool,
    pub search_focused: bool,

    // Seleção de texto
    pub sel_anchor:      Option<usize>,
    pub sel_focus:       Option<usize>,
    pub text_chars:      Vec<TextCharInfo>,
    pub text_chars_page: usize,
    pub selected_text:   Option<String>,

    // Links
    pub page_links: Vec<PdfLinkInfo>,
    pub links_page: usize,

    // Flags
    pub pending_rerender:  bool,
    pub toolbar_opacity:   f32,
    pub pending_close:     bool,

    // Side Panel
    pub side_panel:  SidePanelKind,
    pub bookmarks:   Vec<BookmarkEntry>,
    pub thumb_cache: Option<ThumbCache>,

    // Configurações
    pub settings:          AppSettings,
    pub settings_open:     bool,
    pub needs_font_reload: bool,
    pub needs_render_reload: bool,
}

impl MariApp {
    pub fn new(cc: &eframe::CreationContext<'_>, cli_file: Option<PathBuf>, initial_color_mode: ColorMode) -> Self {
        let _ = cc;
        Self {
            version:             env!("CARGO_PKG_VERSION"),
            current_file:        None, engine: None, current_page: 0, page_count: 0, render_error: None,
            viewer:              PdfViewer::new(),
            drop_zone_hovered:   false, fullscreen: false, color_mode: initial_color_mode,
            goto_open:           false, goto_input: String::new(),
            available_width:     800.0,
            persistence:         PersistenceManager::load(), pending_open: cli_file,
            toast:               None,
            search:              TextSearch::new(), search_open: false, search_focused: false,
            selected_text:       None, sel_anchor: None, sel_focus: None,
            text_chars:          Vec::new(), text_chars_page: usize::MAX,
            page_links:          Vec::new(), links_page: usize::MAX,
            pending_rerender:    false, toolbar_opacity: 1.0, pending_close: false,
            side_panel:          SidePanelKind::None, bookmarks: Vec::new(), thumb_cache: None,
            settings:            AppSettings::load(), settings_open: false,
            needs_font_reload:   false, needs_render_reload: false,
        }
    }

    // ── Abertura de arquivo ───────────────────────────────────────────────────

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"]).set_title("Abrir PDF — MariPDF").pick_file()
        {
            self.pending_open = Some(path);
        }
    }

    fn open_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        tracing::info!("Abrindo: {}", path.display());
        match get_pdfium().and_then(|p| PdfEngine::open(&path, p)) {
            Ok(engine) => {
                let page_count    = engine.page_count();
                self.page_count   = page_count;
                self.bookmarks    = engine.extract_bookmarks();
                self.engine       = Some(engine);
                self.current_file = Some(path.clone());
                self.render_error = None;
                self.viewer.reset();
                self.thumb_cache  = Some(ThumbCache::new(path.clone(), page_count));
                if self.viewer.mode == ViewMode::Continuous {
                    let scale = self.viewer.zoom * 1.5;
                    self.viewer.init_cache(path.clone(), scale);
                }
                if let Some(session) = self.persistence.get(&path) {
                    self.viewer.zoom  = session.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                    self.current_page = session.last_page.min(page_count.saturating_sub(1));
                } else { self.current_page = 0; self.viewer.zoom = 1.0; }
                if self.viewer.mode == ViewMode::SinglePage { self.render_current_page(ctx); }
            }
            Err(e) => {
                let msg = error_to_user_message(&e);
                tracing::error!("{e}");
                let now = ctx.input(|i| i.time);
                self.toast = Some(Toast::error(msg, 5.0, now));
                self.render_error = None; self.current_file = None;
            }
        }
    }

    pub fn save_session(&mut self) {
        if let Some(ref path) = self.current_file.clone() {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            self.persistence.save(path, FileSession { last_page: self.current_page, zoom: self.viewer.zoom, last_opened: now_unix });
        }
    }

    // ── Toast ─────────────────────────────────────────────────────────────────

    fn ui_toast(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if let Some(ref t) = self.toast.clone() {
            if t.is_alive(now) {
                let alpha = t.alpha(now);
                let msg   = t.message.clone();
                let (bg, stroke_c, icon, icon_c, text_c) = match t.kind {
                    ToastKind::Error => (
                        egui::Color32::from_rgba_unmultiplied(35, 12, 12,   (245.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(180, 50, 50,  (200.0 * alpha) as u8),
                        "⚠",
                        egui::Color32::from_rgba_unmultiplied(239, 100, 80, (255.0 * alpha) as u8),
                        egui::Color32::from_rgba_unmultiplied(230, 200, 200,(255.0 * alpha) as u8),
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
                        egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, stroke_c))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(icon).size(14.0).color(icon_c));
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new(&msg).size(12.0).color(text_c));
                                });
                            });
                    });
                ctx.request_repaint();
            } else { self.toast = None; }
        }
    }

    // ── Renderização e navegação ──────────────────────────────────────────────

    pub fn render_current_page(&mut self, ctx: &egui::Context) {
        self.viewer.is_loading = true;
        let Some(engine) = &self.engine else { self.viewer.is_loading = false; return };
        match engine.render_page(self.current_page, self.viewer.zoom * 1.5) {
            Ok(mut rendered) => {
                if matches!(self.color_mode, ColorMode::Dark) { invert_rgba(&mut rendered.rgba); }
                self.viewer.load_page(ctx, &rendered, self.current_page);
                self.render_error = None;
            }
            Err(e) => { tracing::error!("{e}"); self.render_error = Some(format!("{e}")); }
        }
        self.viewer.is_loading = false;
    }

    pub fn go_to_page(&mut self, ctx: &egui::Context, page: usize) {
        let page = page.min(self.page_count.saturating_sub(1));
        if self.current_page == page && self.viewer.texture.is_some() { return; }
        self.current_page = page;
        if self.viewer.mode == ViewMode::SinglePage { self.render_current_page(ctx); }
    }

    pub fn apply_zoom(&mut self, ctx: &egui::Context, delta: f32) {
        let new_zoom = (self.viewer.zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_zoom - self.viewer.zoom).abs() > 0.005 { self.viewer.zoom = new_zoom; self.invalidate_render(ctx); }
    }

    pub fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if (self.viewer.zoom - zoom).abs() > 0.005 { self.viewer.zoom = zoom; self.invalidate_render(ctx); }
    }

    pub fn invalidate_render(&mut self, ctx: &egui::Context) {
        if self.viewer.mode == ViewMode::SinglePage {
            self.viewer.texture = None; self.render_current_page(ctx);
        } else {
            self.viewer.tex_cache.clear(); self.viewer.page_cache.clear();
            if let Some(path) = self.current_file.clone() { let scale = self.viewer.zoom * 1.5; self.viewer.init_cache(path, scale); }
        }
    }

    pub fn fit_width(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        if let Ok(rendered) = engine.render_page(self.current_page, 1.5) {
            let usable = (self.available_width - 48.0).max(100.0);
            let page_w_logical = rendered.width as f32 / 1.5;
            self.set_zoom(ctx, usable / page_w_logical);
        }
    }

    pub fn fit_page(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        if let Ok(rendered) = engine.render_page(self.current_page, 1.5) {
            let usable_w = (self.available_width - 48.0).max(100.0);
            let usable_h = ctx.screen_rect().height() - 48.0;
            let zoom_w   = usable_w / (rendered.width  as f32 / 1.5);
            let zoom_h   = usable_h / (rendered.height as f32 / 1.5);
            self.set_zoom(ctx, zoom_w.min(zoom_h));
        }
    }

    pub fn do_toggle_mode(&mut self, ctx: &egui::Context) {
        match self.viewer.mode {
            ViewMode::SinglePage => {
                self.viewer.mode = ViewMode::Continuous; self.viewer.texture = None;
                if let Some(path) = self.current_file.clone() {
                    let scale = self.viewer.zoom * 1.5; self.viewer.init_cache(path, scale);
                }
            }
            ViewMode::Continuous => {
                self.viewer.mode = ViewMode::SinglePage;
                self.viewer.tex_cache.clear(); self.viewer.page_cache.clear();
                let c = ctx.clone(); self.render_current_page(&c);
            }
        }
    }
}

// ── eframe::App — loop principal ─────────────────────────────────────────────

impl eframe::App for MariApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) { self.save_session(); }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        configure_visuals(ctx);

        // Fechar documento pendente
        if self.pending_close {
            self.pending_close = false; self.current_file = None; self.engine = None;
            self.viewer.reset(); self.render_error = None; self.search.clear();
            self.search_open = false; self.goto_open = false;
            self.page_links = Vec::new(); self.bookmarks = Vec::new();
            self.thumb_cache = None; self.side_panel = SidePanelKind::None;
        }

        // Drena bitmaps de thumbnails prontos
        if let Some(ref mut tc) = self.thumb_cache { if tc.drain_ready() { ctx.request_repaint(); } }

        // Arquivo pendente
        if let Some(path) = self.pending_open.take() { let c = ctx.clone(); self.open_file(&c, path); }

        // Rerender pendente (troca de tema)
        if self.pending_rerender && self.engine.is_some() {
            self.pending_rerender = false; let c = ctx.clone(); self.render_current_page(&c);
        }

        // Drag & drop
        ctx.input(|i| {
            self.drop_zone_hovered = !i.raw.hovered_files.is_empty();
            if let Some(f) = i.raw.dropped_files.first() {
                if let Some(p) = f.path.clone() {
                    if p.extension().map(|e| e == "pdf").unwrap_or(false) { self.pending_open = Some(p); }
                }
            }
        });

        // Atalhos globais
        let (mut go_next, mut go_prev, mut zoom_in, mut zoom_out) = (false, false, false, false);
        let (mut go_home, mut go_end, mut toggle_fs, mut fit_w)   = (false, false, false, false);
        let (mut fit_p, mut open_goto, mut toggle_mode)            = (false, false, false);
        let (mut open_search, mut search_next, mut search_prev)    = (false, false, false);
        let (mut escape, mut select_all, mut copy_text)            = (false, false, false);

        ctx.input(|i| {
            let ctrl  = i.modifiers.ctrl;
            let shift = i.modifiers.shift;
            go_next     = i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown);
            go_prev     = i.key_pressed(egui::Key::ArrowLeft)  || i.key_pressed(egui::Key::PageUp);
            go_home     = i.key_pressed(egui::Key::Home);
            go_end      = i.key_pressed(egui::Key::End);
            toggle_fs   = i.key_pressed(egui::Key::F11);
            zoom_in     = ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
            zoom_out    = ctrl && i.key_pressed(egui::Key::Minus);
            fit_w       = i.key_pressed(egui::Key::W) && !ctrl;
            fit_p       = ctrl && i.key_pressed(egui::Key::Num0);
            open_goto   = ctrl && i.key_pressed(egui::Key::G);
            toggle_mode = ctrl && shift && i.key_pressed(egui::Key::M);
            open_search = ctrl && i.key_pressed(egui::Key::F);
            search_next = i.key_pressed(egui::Key::F3) && !shift;
            search_prev = i.key_pressed(egui::Key::F3) &&  shift;
            escape      = i.key_pressed(egui::Key::Escape);
            select_all  = ctrl && i.key_pressed(egui::Key::A);
            copy_text   = ctrl && i.key_pressed(egui::Key::C);
        });

        if toggle_fs { self.fullscreen = !self.fullscreen; ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen)); }

        let zoom_delta = std::mem::replace(&mut self.viewer.zoom_delta, 0.0);
        let page_delta = std::mem::replace(&mut self.viewer.page_delta, 0);
        if zoom_delta != 0.0 { self.apply_zoom(ctx, zoom_delta); }
        if zoom_in           { self.apply_zoom(ctx,  0.15); }
        if zoom_out          { self.apply_zoom(ctx, -0.15); }

        if open_goto { if self.search_open { self.search_open = false; } self.goto_open = !self.goto_open; self.goto_input.clear(); }
        if open_search && self.engine.is_some() {
            if self.goto_open { self.goto_open = false; }
            self.search_open = !self.search_open; self.search_focused = self.search_open;
            if !self.search_open { self.search.clear(); }
        }
        if search_next && self.search_open { self.search.go_next(); }
        if search_prev && self.search_open { self.search.go_prev(); }
        if escape { self.goto_open = false; self.search_open = false; self.search.clear(); }
        if (search_next || search_prev) && self.search_open {
            if let Some(hit) = self.search.current() { let pg = hit.page_index; let c = ctx.clone(); self.go_to_page(&c, pg); }
        }

        if fit_w { let c = ctx.clone(); self.fit_width(&c); }
        if fit_p { let c = ctx.clone(); self.fit_page(&c); }

        if self.engine.is_some() {
            if page_delta != 0 && self.viewer.mode == ViewMode::SinglePage {
                let p = (self.current_page as i64 + page_delta as i64).clamp(0, self.page_count as i64 - 1) as usize;
                let c = ctx.clone(); self.go_to_page(&c, p);
            }
            if go_next && self.current_page + 1 < self.page_count { let p = self.current_page + 1; let c = ctx.clone(); self.go_to_page(&c, p); }
            if go_prev && self.current_page > 0                    { let p = self.current_page - 1; let c = ctx.clone(); self.go_to_page(&c, p); }
            if go_home { let c = ctx.clone(); self.go_to_page(&c, 0); }
            if go_end  { let last = self.page_count.saturating_sub(1); let c = ctx.clone(); self.go_to_page(&c, last); }
            if toggle_mode { let c = ctx.clone(); self.do_toggle_mode(&c); }
        }

        if select_all && self.engine.is_some() {
            let now = ctx.input(|i| i.time);
            if let Some(engine) = &self.engine {
                match engine.extract_text(self.current_page) {
                    Ok(text) if !text.trim().is_empty() => {
                        self.selected_text = Some(text);
                        self.toast = Some(Toast::info("Texto selecionado — Ctrl+C para copiar", 3.0, now));
                    }
                    Ok(_)    => { self.toast = Some(Toast::error("Página sem texto extraível", 2.5, now)); }
                    Err(_)   => { self.toast = Some(Toast::error("Falha ao extrair texto",      2.5, now)); }
                }
            }
        }
        if copy_text {
            if let Some(ref text) = self.selected_text.clone() {
                let now = ctx.input(|i| i.time);
                ctx.copy_text(text.clone());
                self.toast = Some(Toast::info("Texto copiado!", 2.0, now));
            }
        }

        // Cache de links
        if self.engine.is_some() && self.links_page != self.current_page {
            self.page_links = self.engine.as_ref().map(|e| e.extract_page_links(self.current_page)).unwrap_or_default();
            self.links_page = self.current_page;
        }
        if !self.page_links.is_empty() {
            let maybe_click = ctx.input(|i| { if i.pointer.button_clicked(egui::PointerButton::Primary) { i.pointer.interact_pos() } else { None } });
            if let Some(click_pos) = maybe_click {
                let page_rect = self.viewer.last_page_rect;
                if page_rect.is_positive() && page_rect.contains(click_pos) && self.page_links.len() == 1 {
                    let uri = self.page_links[0].uri.clone(); let _ = open::that(uri);
                }
            }
        }

        // ── Renderiza UI ─────────────────────────────────────────────────────
        if self.current_file.is_none() { self.ui_home(ctx); } else { self.ui_reader(ctx); }
        if self.goto_open && self.engine.is_some() { self.ui_goto_overlay(ctx); }
        self.ui_toast(ctx);
        if self.search_open && self.engine.is_some() { self.ui_search_bar(ctx); }
        if self.settings_open {
            let mut needs_font   = false;
            let mut needs_render = false;
            crate::ui::settings_panel::show_settings_panel(ctx, &mut self.settings_open, &mut self.settings, &mut self.color_mode, &mut needs_font, &mut needs_render);
            if needs_font   { crate::apply_font_settings(ctx, self.settings.font); self.settings.save(); }
            if needs_render { self.invalidate_render(ctx); self.settings.save(); }
        }
    }
}

// ── Helpers globais ───────────────────────────────────────────────────────────

fn configure_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill           = BG_BASE;
    v.window_fill          = BG_SURFACE;
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.window_stroke        = egui::Stroke::new(1.0, BORDER_SUBTLE);
    v.override_text_color  = Some(TEXT_PRIMARY);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(65, 48, 128);
    v.widgets.hovered.bg_fill  = ACCENT_PRIMARY;
    v.widgets.active.bg_fill   = ACCENT_LIGHT;
    ctx.set_visuals(v);
    ctx.style_mut(|s| { s.spacing.scroll.bar_width = 7.0; s.spacing.scroll.bar_inner_margin = 2.0; s.spacing.scroll.bar_outer_margin = 1.0; });
}

fn error_to_user_message(e: &anyhow::Error) -> String {
    let s = e.to_string().to_lowercase();
    if s.contains("password") || s.contains("encrypted") || s.contains("senha") { return "🔒 PDF protegido por senha — não suportado no momento".into(); }
    if s.contains("corrupt")  || s.contains("invalid")   || s.contains("damaged") { return "⚠ Arquivo PDF corrompido ou inválido".into(); }
    if s.contains("not found") || s.contains("no such file") { return "❌ Arquivo não encontrado".into(); }
    format!("⚠ Erro ao abrir PDF: {e}")
}
