// ui/viewer.rs — Viewer de PDF com modos Single-Page e Contínuo
//
// Responsabilidades:
//   • Single-Page: 1 página por vez; scroll do mouse muda de página
//   • Continuous: scroll vertical por todas as páginas (lazy rendering)
//   • Zoom via Ctrl+Scroll em ambos os modos
//   • Sinais para o app.rs: zoom_delta, page_delta

use crate::pdf::document_worker::{DocumentWorkerHandle, JobPriority};
use crate::pdf::engine::RenderedPage;
use crate::pdf::page_cache::{LruPageCache, DEFAULT_CPU_CACHE_BYTES};
use crate::ui::page_layout::{LayoutAnchor, PageLayout};
use crate::ui::texture_cache::GpuTextureCache;
use eframe::egui;

pub use crate::ui::viewer_helpers::invert_rgba;
use crate::ui::viewer_helpers::{
    draw_page_tex, fit_page, paint_search_highlight, show_spinner, upload_texture,
};

// ── Modo de visualização ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    /// Uma página por vez; scroll do mouse → muda página
    SinglePage,
    /// Todas as páginas em sequência; scroll normal
    Continuous,
}

// ── Destaque de busca ────────────────────────────────────────────────────────

/// Ocorrência de busca ativa, em coordenadas PDF, desenhada sobre a página.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHighlight {
    pub page_index: usize,
    pub page_size: (f32, f32),
    /// `[left, bottom, right, top]` em pontos PDF.
    pub rects: Vec<[f32; 4]>,
}

// ── Viewer ───────────────────────────────────────────────────────────────────

pub struct PdfViewer {
    // Modo atual
    pub mode: ViewMode,

    // ── Single-page state ──
    pub(crate) texture: Option<egui::TextureHandle>,
    pub(crate) loaded_page: Option<usize>,

    // ── Cache LRU alimentado pelo scheduler global ──
    /// Cache de texturas já enviadas para a GPU
    pub(crate) tex_cache: GpuTextureCache,
    /// Cache LRU de bitmaps vindos do scheduler global do documento.
    pub(crate) page_cache: LruPageCache,
    /// Índice de offsets/alturas usado pela virtualização contínua.
    pub(crate) page_layout: PageLayout,
    /// Página com maior área visível no viewport contínuo.
    pub(crate) visible_page: Option<usize>,
    /// Navegação programática pendente no modo contínuo.
    pub(crate) pending_scroll_page: Option<usize>,
    /// Navegação programática para um ponto dentro da página (busca).
    pub(crate) pending_scroll_anchor: Option<LayoutAnchor>,
    /// Salto animado em andamento (Scroll Suave ligado).
    pub(crate) scroll_tween: Option<crate::ui::scroll_motion::ScrollTween>,
    /// Âncora mantida ao recalcular alturas/zoom.
    pub(crate) continuous_anchor: Option<LayoutAnchor>,
    pub(crate) continuous_viewport_height: f32,
    pub(crate) continuous_scroll_offset: Option<f32>,

    // ── Sinais para o app.rs ──
    /// Delta de zoom acumulado por Ctrl+Scroll (processado em app.rs)
    pub zoom_delta: f32,
    /// Pedido de mudança de página em modo Single (±1 por tick de scroll)
    pub page_delta: i32,
    /// Retângulo da última página renderizada em coordenadas de tela.
    /// Usado pelo app.rs para hit-test de links e cursor contextual.
    pub last_page_rect: egui::Rect,
    /// Ocorrência de busca ativa; definida pelo app a cada frame.
    pub search_highlight: Option<SearchHighlight>,

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
            tex_cache: GpuTextureCache::new(),
            page_cache: LruPageCache::new_sync(DEFAULT_CPU_CACHE_BYTES),
            page_layout: PageLayout::new(),
            visible_page: None,
            pending_scroll_page: None,
            pending_scroll_anchor: None,
            scroll_tween: None,
            continuous_anchor: None,
            continuous_viewport_height: 0.0,
            continuous_scroll_offset: None,
            zoom_delta: 0.0,
            page_delta: 0,
            last_page_rect: egui::Rect::NOTHING,
            search_highlight: None,
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
    /// Deve ser chamado depois de `reset()` com o documento aberto.
    pub fn init_cache(&mut self, scheduler: DocumentWorkerHandle, page_count: usize) {
        self.page_cache = LruPageCache::new(DEFAULT_CPU_CACHE_BYTES, scheduler);
        self.tex_cache.clear();
        self.page_layout.ensure_page_count(page_count);
    }

    /// Reset completo ao abrir novo documento
    pub fn reset(&mut self) {
        self.texture = None;
        self.loaded_page = None;
        self.tex_cache.clear();
        self.page_cache = LruPageCache::new_sync(DEFAULT_CPU_CACHE_BYTES);
        self.page_layout.clear();
        self.visible_page = None;
        self.pending_scroll_page = None;
        self.pending_scroll_anchor = None;
        self.scroll_tween = None;
        self.search_highlight = None;
        self.continuous_anchor = None;
        self.continuous_viewport_height = 0.0;
        self.continuous_scroll_offset = None;
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
        self.page_layout.observe_bitmap(
            idx,
            page.width,
            page.height,
            self.zoom * self.render_scale,
        );
        self.texture = Some(upload_texture(ctx, page, idx));
        self.loaded_page = Some(idx);
    }

    pub fn request_continuous_page(&mut self, page: usize) {
        self.pending_scroll_page = Some(page);
    }

    /// Centraliza no viewport contínuo o ponto `page_fraction` (0 = topo) da página.
    pub fn reveal_continuous(&mut self, page: usize, page_fraction: f32) {
        self.pending_scroll_page = None;
        self.pending_scroll_anchor = Some(LayoutAnchor {
            page,
            page_fraction: page_fraction.clamp(0.0, 1.0),
        });
    }

    pub fn dominant_continuous_page(&self) -> Option<usize> {
        self.visible_page
    }

    pub fn request_single_page(&mut self, page_index: usize, page_count: usize) {
        let scale = self.zoom * self.render_scale;
        self.page_cache
            .prefetch(page_index, scale, JobPriority::Immediate);
        if page_index + 1 < page_count {
            self.page_cache
                .prefetch(page_index + 1, scale, JobPriority::Visible);
        }
        if page_index > 0 {
            self.page_cache
                .prefetch(page_index - 1, scale, JobPriority::Visible);
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

    /// Consome bitmaps prontos e mantém `texture` na página atual sem bloquear.
    /// A última textura válida continua disponível enquanto a nova renderiza.
    pub(crate) fn sync_single_texture(
        &mut self,
        ctx: &egui::Context,
        current_page: usize,
        page_count: usize,
    ) {
        if self.page_cache.drain_ready() {
            ctx.request_repaint();
        }

        if self.loaded_page != Some(current_page) {
            if let Some(mut page) = self.page_cache.get(current_page).cloned() {
                if self.invert_colors {
                    invert_rgba(&mut page.rgba);
                }
                self.load_page(ctx, &page, current_page);
                self.is_loading = false;
                self.last_error = None;
            } else if let Some(error) = self.page_cache.take_failure(current_page) {
                self.is_loading = false;
                self.last_error = Some(error);
            } else {
                self.request_single_page(current_page, page_count);
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
    }

    fn show_single(
        &mut self,
        ui: &mut egui::Ui,
        current_page: usize,
        page_count: usize,
        page_tint: egui::Color32,
        page_border: egui::Color32,
    ) {
        self.sync_single_texture(ui.ctx(), current_page, page_count);

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
                    paint_search_highlight(
                        ui,
                        page_rect,
                        self.search_highlight.as_ref(),
                        self.loaded_page,
                    );
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
                            paint_search_highlight(
                                ui,
                                page_rect,
                                self.search_highlight.as_ref(),
                                self.loaded_page,
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
}
