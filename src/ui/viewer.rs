// ui/viewer.rs — Viewer de PDF com modos Single-Page e Contínuo
//
// Responsabilidades:
//   • Single-Page: 1 página por vez; scroll do mouse muda de página
//   • Continuous: scroll vertical por todas as páginas (lazy rendering)
//   • Zoom via Ctrl+Scroll em ambos os modos
//   • Sinais para o app.rs: zoom_delta, page_delta

use crate::pdf::engine::RenderedPage;
use crate::pdf::page_cache::{LruPageCache, DEFAULT_CAPACITY};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub use crate::ui::viewer_helpers::invert_rgba;
use crate::ui::viewer_helpers::{draw_page_tex, fit_page, show_spinner, upload_texture};

// ── Modo de visualização ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Uma página por vez; scroll do mouse → muda página
    SinglePage,
    /// Todas as páginas em sequência; scroll normal
    Continuous,
}

// ── Viewer ───────────────────────────────────────────────────────────────────

pub struct PdfViewer {
    // Modo atual
    pub mode: ViewMode,

    // ── Single-page state ──
    pub(crate) texture: Option<egui::TextureHandle>,
    pub(crate) loaded_page: Option<usize>,

    // ── Continuous-mode: cache LRU com background thread ──
    /// Cache de texturas já enviadas para a GPU
    pub(crate) tex_cache: HashMap<usize, egui::TextureHandle>,
    /// Cache LRU de bitmaps vindos da thread de background
    pub(crate) page_cache: LruPageCache,
    /// Caminho do arquivo atual (para criar novo cache ao trocar de doc)
    pub(crate) pdf_path: Option<PathBuf>,

    // ── Sinais para o app.rs ──
    /// Delta de zoom acumulado por Ctrl+Scroll (processado em app.rs)
    pub zoom_delta: f32,
    /// Pedido de mudança de página em modo Single (±1 por tick de scroll)
    pub page_delta: i32,
    /// Retângulo da última página renderizada em coordenadas de tela.
    /// Usado pelo app.rs para hit-test de links e cursor contextual.
    pub last_page_rect: egui::Rect,

    // ── Parâmetros compartilhados ──
    pub zoom: f32,
    /// Acumulador de scroll para threshold de mudança de página (modo Single)
    scroll_accum: f32,
    /// Indica se há renderização em andamento (usado pela Status Bar)
    pub is_loading: bool,
    /// Se true, inverte as cores RGB do bitmap (dark mode PDF)
    pub invert_colors: bool,
    /// Multiplicador de rasterização derivado da preferência de qualidade.
    pub render_scale: f32,
    /// Margem visual configurada entre página e viewport.
    pub page_margin: f32,
    /// Define se deltas suavizados do egui são usados para zoom/scroll.
    pub smooth_scroll: bool,
    /// Última falha de render pronta para o app apresentar.
    pub(crate) last_error: Option<String>,
}

impl PdfViewer {
    pub fn new() -> Self {
        Self {
            mode: ViewMode::SinglePage,
            texture: None,
            loaded_page: None,
            tex_cache: HashMap::new(),
            page_cache: LruPageCache::new_sync(DEFAULT_CAPACITY),
            pdf_path: None,
            zoom_delta: 0.0,
            page_delta: 0,
            last_page_rect: egui::Rect::NOTHING,
            zoom: 1.0,
            scroll_accum: 0.0,
            is_loading: false,
            invert_colors: false,
            render_scale: 1.5,
            page_margin: 20.0,
            smooth_scroll: true,
            last_error: None,
        }
    }

    /// Inicializa o cache de background ao abrir um novo documento.
    /// Deve ser chamado depois de `reset()` com o caminho do arquivo aberto.
    pub fn init_cache(&mut self, pdf_path: PathBuf, scale: f32) {
        self.pdf_path = Some(pdf_path.clone());
        self.page_cache = LruPageCache::new(DEFAULT_CAPACITY, pdf_path, scale);
        self.tex_cache.clear();
    }

    /// Reset completo ao abrir novo documento
    pub fn reset(&mut self) {
        self.texture = None;
        self.loaded_page = None;
        self.tex_cache.clear();
        self.page_cache = LruPageCache::new_sync(DEFAULT_CAPACITY);
        self.pdf_path = None;
        self.zoom_delta = 0.0;
        self.page_delta = 0;
        self.last_page_rect = egui::Rect::NOTHING;
        self.zoom = 1.0;
        self.scroll_accum = 0.0;
        self.is_loading = false;
        self.invert_colors = false;
        self.last_error = None;
    }

    // ── API para Single-Page ─────────────────────────────────────────────────

    pub fn load_page(&mut self, ctx: &egui::Context, page: &RenderedPage, idx: usize) {
        self.texture = Some(upload_texture(ctx, page, idx));
        self.loaded_page = Some(idx);
    }

    pub fn request_single_page(&mut self, page_index: usize, page_count: usize) {
        let scale = self.zoom * self.render_scale;
        self.page_cache.prefetch(page_index, scale);
        if page_index + 1 < page_count {
            self.page_cache.prefetch(page_index + 1, scale);
        }
        if page_index > 0 {
            self.page_cache.prefetch(page_index - 1, scale);
        }
        self.is_loading = self.loaded_page != Some(page_index);
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.last_error.take()
    }

    // ── Ponto de entrada principal ───────────────────────────────────────────

    /// Renderiza o viewer. Parâmetros:
    /// - `current_page`: página ativa no modo Single
    /// - `page_count`: total de páginas
    /// - `page_tint`: cor multiplicada sobre o bitmap (WHITE = sem filtro, Sepia = amarelo quente)
    /// - `page_border`: cor da borda fina ao redor da página (adaptada ao tema)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        current_page: usize,
        page_count: usize,
        page_tint: egui::Color32,
        page_border: egui::Color32,
    ) {
        match self.mode {
            ViewMode::SinglePage => {
                self.show_single(ui, current_page, page_count, page_tint, page_border)
            }
            ViewMode::Continuous => self.show_continuous(ui, page_count, page_tint, page_border),
        }
    }

    // ── Modo Single-Page ─────────────────────────────────────────────────────

    fn show_single(
        &mut self,
        ui: &mut egui::Ui,
        current_page: usize,
        page_count: usize,
        page_tint: egui::Color32,
        page_border: egui::Color32,
    ) {
        if self.page_cache.drain_ready() {
            ui.ctx().request_repaint();
        }

        if self.loaded_page != Some(current_page) {
            if let Some(mut page) = self.page_cache.get(current_page).cloned() {
                if self.invert_colors {
                    invert_rgba(&mut page.rgba);
                }
                self.load_page(ui.ctx(), &page, current_page);
                self.is_loading = false;
                self.last_error = None;
            } else if let Some(error) = self.page_cache.take_failure(current_page) {
                self.is_loading = false;
                self.last_error = Some(error);
            } else {
                self.request_single_page(current_page, page_count);
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        // Lê AMBOS os deltas antes de qualquer widget consumir o evento:
        //   • scroll_delta     = valor bruto por frame (1 notch = ~50px, instantâneo)
        //   • smooth_scroll_delta = suavizado ao longo de vários frames
        let (ctrl, raw_y, smooth_y) = ui.input(|i| {
            (
                i.modifiers.ctrl,
                i.raw_scroll_delta.y, // 1 notch → valor inteiro imediato (ex: 120px)
                if self.smooth_scroll {
                    i.smooth_scroll_delta.y
                } else {
                    i.raw_scroll_delta.y
                },
            )
        });

        if ctrl {
            // Ctrl + Scroll → zoom (usa smooth para não dar solavancos)
            if smooth_y != 0.0 {
                self.zoom_delta = smooth_y.signum() * 0.15;
                self.scroll_accum = 0.0;
            }
        } else {
            // Scroll simples → muda página
            // raw_delta: 1 notch de mouse wheel = ~50 px no Windows
            // Threshold baixo (5 px) → 1 notch = 1 página, sem acumular
            if raw_y < -5.0 {
                self.page_delta = 1; // próxima
                self.scroll_accum = 0.0;
            } else if raw_y > 5.0 {
                self.page_delta = -1; // anterior
                self.scroll_accum = 0.0;
            }
        }

        let available = ui.available_size();

        if let Some(ref tex) = self.texture {
            let tex_size = tex.size_vec2();
            let (w, h) = fit_page(tex_size, available.x - self.page_margin * 2.0);
            let page_size = egui::vec2(w, h);

            // ── Sem ScrollArea: página centrada no viewport ──────────────
            // Se a página for maior que o viewport (zoom alto),
            // usamos um ScrollArea de PAN mas bloqueamos o scroll do mouse
            // (que já foi capturado acima como mudança de página).
            if h <= available.y - 8.0 {
                // Página cabe na tela — centraliza verticalmente
                let mut page_rect = egui::Rect::NOTHING;
                ui.vertical_centered(|ui| {
                    ui.add_space((available.y - h).max(0.0) / 2.0);
                    draw_page_tex(&mut page_rect, ui, tex, page_size, page_tint, page_border);
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "\u{2014} {}/{} \u{2014}",
                            current_page + 1,
                            page_count
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(70, 70, 90)),
                    );
                });
                self.last_page_rect = page_rect;
            } else {
                // Página maior que viewport: ScrollArea de pan sem capturar wheel
                let mut page_rect = egui::Rect::NOTHING;
                egui::ScrollArea::vertical()
                    .id_salt("single_pan")
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add_space(self.page_margin);
                        ui.vertical_centered(|ui| {
                            draw_page_tex(
                                &mut page_rect,
                                ui,
                                tex,
                                page_size,
                                page_tint,
                                page_border,
                            );
                        });
                        ui.add_space(self.page_margin);
                    });
                self.last_page_rect = page_rect;
            }
        } else {
            show_spinner(ui, available.y);
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    pub(crate) fn estimate_page_height(&self, available_w: f32) -> f32 {
        // Usa a primeira textura disponível no tex_cache para estimar
        if let Some(tex) = self.tex_cache.values().next() {
            let size = tex.size_vec2();
            let w = size.x.min(available_w - self.page_margin * 2.0);
            return size.y * (w / size.x);
        }
        if let Some(ref tex) = self.texture {
            let size = tex.size_vec2();
            let w = size.x.min(available_w - self.page_margin * 2.0);
            return size.y * (w / size.x);
        }
        1100.0
    }
}
