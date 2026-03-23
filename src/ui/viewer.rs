// ui/viewer.rs — Viewer de PDF com modos Single-Page e Contínuo
//
// Responsabilidades:
//   • Single-Page: 1 página por vez; scroll do mouse muda de página
//   • Continuous: scroll vertical por todas as páginas (lazy rendering via PageCache)
//   • Zoom via Ctrl+Scroll em ambos os modos
//   • Expõe sinais para o app.rs (zoom_delta, page_delta)

use eframe::egui;
use std::collections::HashMap;
use crate::pdf::engine::{PdfEngine, RenderedPage};
use crate::pdf::page_cache::{LruPageCache, DEFAULT_CAPACITY};
use std::path::PathBuf;

/// Quantas páginas adjacentes pré-renderizar agressivamente no modo Contínuo
const PREFETCH_AHEAD:  usize = 3;
const PREFETCH_BEHIND: usize = 1;

// ── Constantes ───────────────────────────────────────────────────────────────
const PAGE_GAP: f32 = 20.0;

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
    pub(crate) texture:     Option<egui::TextureHandle>,
    pub(crate) loaded_page: Option<usize>,

    // ── Continuous-mode: cache LRU com background thread ──
    /// Cache de texturas já enviadas para a GPU
    pub(crate) tex_cache:  HashMap<usize, egui::TextureHandle>,
    /// Cache LRU de bitmaps vindos da thread de background
    pub(crate) page_cache: LruPageCache,
    /// Caminho do arquivo atual (para criar novo cache ao trocar de doc)
    pub(crate) pdf_path:   Option<PathBuf>,

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
}


impl PdfViewer {
    pub fn new() -> Self {
        Self {
            mode:           ViewMode::SinglePage,
            texture:        None,
            loaded_page:    None,
            tex_cache:      HashMap::new(),
            page_cache:     LruPageCache::new_sync(DEFAULT_CAPACITY),
            pdf_path:       None,
            zoom_delta:     0.0,
            page_delta:     0,
            last_page_rect: egui::Rect::NOTHING,
            zoom:           1.0,
            scroll_accum:   0.0,
            is_loading:     false,
            invert_colors:  false,
        }
    }

    /// Inicializa o cache de background ao abrir um novo documento.
    /// Deve ser chamado depois de `reset()` com o caminho do arquivo aberto.
    pub fn init_cache(&mut self, pdf_path: PathBuf, scale: f32) {
        self.pdf_path  = Some(pdf_path.clone());
        self.page_cache = LruPageCache::new(DEFAULT_CAPACITY, pdf_path, scale);
        self.tex_cache.clear();
    }

    /// Reset completo ao abrir novo documento
    pub fn reset(&mut self) {
        self.texture        = None;
        self.loaded_page    = None;
        self.tex_cache.clear();
        self.page_cache     = LruPageCache::new_sync(DEFAULT_CAPACITY);
        self.pdf_path       = None;
        self.zoom_delta     = 0.0;
        self.page_delta     = 0;
        self.last_page_rect = egui::Rect::NOTHING;
        self.zoom           = 1.0;
        self.scroll_accum   = 0.0;
        self.is_loading     = false;
        self.invert_colors   = false;
    }

    // ── API para Single-Page ─────────────────────────────────────────────────

    pub fn load_page(&mut self, ctx: &egui::Context, page: &RenderedPage, idx: usize) {
        self.texture     = Some(upload_texture(ctx, page, idx));
        self.loaded_page = Some(idx);
    }

    // ── Ponto de entrada principal ───────────────────────────────────────────

    /// Renderiza o viewer. Parâmetros:
    /// - `engine`: preciso para render lazy no modo Contínuo
    /// - `current_page`: página ativa no modo Single
    /// - `page_count`: total de páginas
    /// - `page_tint`: cor multiplicada sobre o bitmap (WHITE = sem filtro, Sepia = amarelo quente)
    /// - `page_border`: cor da borda fina ao redor da página (adaptada ao tema)
    pub fn show(
        &mut self,
        ui:           &mut egui::Ui,
        engine:       &PdfEngine,
        current_page: usize,
        page_count:   usize,
        page_tint:    egui::Color32,
        page_border:  egui::Color32,
    ) {
        match self.mode {
            ViewMode::SinglePage  => self.show_single(ui, current_page, page_count, page_tint, page_border),
            ViewMode::Continuous  => self.show_continuous(ui, engine, page_count, page_tint, page_border),
        }
    }

    // ── Modo Single-Page ─────────────────────────────────────────────────────

    fn show_single(&mut self, ui: &mut egui::Ui, current_page: usize, page_count: usize, page_tint: egui::Color32, page_border: egui::Color32) {
        // Lê AMBOS os deltas antes de qualquer widget consumir o evento:
        //   • scroll_delta     = valor bruto por frame (1 notch = ~50px, instantâneo)
        //   • smooth_scroll_delta = suavizado ao longo de vários frames
        let (ctrl, raw_y, smooth_y) = ui.input(|i| (
            i.modifiers.ctrl,
            i.raw_scroll_delta.y,      // 1 notch → valor inteiro imediato (ex: 120px)
            i.smooth_scroll_delta.y,   // suavizado → usado para zoom fluido
        ));

        if ctrl {
            // Ctrl + Scroll → zoom (usa smooth para não dar solavancos)
            if smooth_y != 0.0 {
                self.zoom_delta   = smooth_y.signum() * 0.15;
                self.scroll_accum = 0.0;
            }
        } else {
            // Scroll simples → muda página
            // raw_delta: 1 notch de mouse wheel = ~50 px no Windows
            // Threshold baixo (5 px) → 1 notch = 1 página, sem acumular
            if raw_y < -5.0 {
                self.page_delta   =  1; // próxima
                self.scroll_accum =  0.0;
            } else if raw_y > 5.0 {
                self.page_delta   = -1; // anterior
                self.scroll_accum =  0.0;
            }
        }

        let available = ui.available_size();

        if let Some(ref tex) = self.texture {
            let tex_size   = tex.size_vec2();
            let (w, h)     = fit_page(tex_size, available.x - 48.0);
            let page_size  = egui::vec2(w, h);

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
                    ui.label(egui::RichText::new(
                        format!("\u{2014} {}/{} \u{2014}", current_page + 1, page_count)
                    ).size(11.0).color(egui::Color32::from_rgb(70, 70, 90)));
                });
                self.last_page_rect = page_rect;
            } else {
                // Página maior que viewport: ScrollArea de pan sem capturar wheel
                let mut page_rect = egui::Rect::NOTHING;
                egui::ScrollArea::vertical()
                    .id_salt("single_pan")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add_space(PAGE_GAP);
                        ui.vertical_centered(|ui| {
                            draw_page_tex(&mut page_rect, ui, tex, page_size, page_tint, page_border);
                        });
                        ui.add_space(PAGE_GAP);
                    });
                self.last_page_rect = page_rect;
            }
        } else {
            show_spinner(ui, available.y);
        }
    }

    // ── Modo Contínuo ────────────────────────────────────────────────────────

    fn show_continuous(
        &mut self,
        ui:         &mut egui::Ui,
        engine:     &PdfEngine,
        page_count: usize,
        page_tint:  egui::Color32,
        page_border: egui::Color32,
    ) {
        // Ctrl+Scroll → zoom
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if ctrl && scroll_y != 0.0 {
            self.zoom_delta = scroll_y.signum() * 0.15;
        }

        let scale       = self.zoom * 1.5;
        let available_w = ui.available_width();
        let ctx         = ui.ctx().clone();

        // ── Drena bitmaps prontos da thread de background ────────────────────
        if self.page_cache.drain_ready() {
            ctx.request_repaint();
        }

        // Converte RenderedPage disponibilizados pelo cache em TextureHandle
        for idx in 0..page_count {
            if !self.tex_cache.contains_key(&idx) {
                // Checa se o bitmap já chegou (sem bloquear)
                if let Some(bitmap) = self.page_cache.get(idx) {
                    if self.invert_colors {
                        let mut inverted: RenderedPage = (*bitmap).clone();
                        invert_rgba(&mut inverted.rgba);
                        let tex = upload_texture(&ctx, &inverted, idx);
                        self.tex_cache.insert(idx, tex);
                    } else {
                        let tex = upload_texture(&ctx, bitmap, idx);
                        self.tex_cache.insert(idx, tex);
                    }
                }
            }
        }

        egui::ScrollArea::vertical()
            .id_salt("continuous_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(PAGE_GAP);

                let est_page_h  = self.estimate_page_height(available_w);
                let scroll_top  = ui.min_rect().top();
                let vp_h        = ctx.screen_rect().height();

                let first_visible = ((scroll_top.abs() / (est_page_h + PAGE_GAP)) as usize)
                    .saturating_sub(PREFETCH_BEHIND + 1);
                let last_visible  = (first_visible
                    + (vp_h / (est_page_h + PAGE_GAP)).ceil() as usize
                    + PREFETCH_AHEAD + PREFETCH_BEHIND + 2)
                    .min(page_count.saturating_sub(1));

                // Expulsa texturas fora da janela do viewport estendida
                let cache_first = first_visible.saturating_sub(PREFETCH_BEHIND);
                let cache_last  = last_visible;
                self.tex_cache.retain(|&idx, _| idx >= cache_first && idx <= cache_last);
                self.page_cache.evict_range(cache_first, cache_last);

                // Solicita pré-render das páginas vizinhas ao viewport
                for idx in first_visible..=last_visible {
                    if idx < page_count {
                        self.page_cache.prefetch(idx, scale);
                    }
                }

                for idx in 0..page_count {
                    // Fallback síncrono se a página está visível mas ainda não chegou
                    let is_visible = idx >= first_visible && idx <= last_visible;
                    if is_visible && !self.tex_cache.contains_key(&idx) && self.pdf_path.is_none() {
                        if let Ok(mut rendered) = engine.render_page(idx, scale) {
                            if self.invert_colors { invert_rgba(&mut rendered.rgba); }
                            let tex = upload_texture(ui.ctx(), &rendered, idx);
                            self.tex_cache.insert(idx, tex);
                        }
                    }

                    ui.vertical_centered(|ui| {
                        if let Some(tex) = self.tex_cache.get(&idx) {
                            let (w, h) = fit_page(tex.size_vec2(), available_w - 48.0);
                            let mut page_rect = egui::Rect::NOTHING;
                            draw_page_tex(&mut page_rect, ui, tex, egui::vec2(w, h), page_tint, page_border);
                            // guarda o rect da última página visível (visível mais próxima do centro)
                            if self.last_page_rect == egui::Rect::NOTHING { self.last_page_rect = page_rect; }
                        } else {
                            // Skeleton animado — pulsa suavemente enquanto a página renderiza
                            let ph = est_page_h;
                            let pw = (available_w - 48.0).min(800.0);
                            let (_, rect) = ui.allocate_space(egui::vec2(pw, ph));

                            let t = ui.ctx().input(|i| i.time);
                            // Pulso com fase diferente por página para não piscarem juntas
                            let phase = (idx as f64) * 0.4;
                            let alpha = ((t + phase).sin() * 0.15 + 0.70) as f32; // 0.55..0.85
                            let base = egui::Color32::from_rgba_premultiplied(
                                (38.0 * alpha) as u8,
                                (38.0 * alpha) as u8,
                                (50.0 * alpha) as u8,
                                255,
                            );
                            ui.painter().rect_filled(rect, egui::CornerRadius::same(6), base);

                            // Stripes 3 linhas fictícias de texto (shimmer)
                            let stripe_color = egui::Color32::from_rgba_premultiplied(
                                (60.0 * alpha) as u8, (60.0 * alpha) as u8,
                                (80.0 * alpha) as u8, 255,
                            );
                            let stripe_h = 10.0;
                            let stripe_gap = 18.0;
                            for i in 0..3 {
                                let y = rect.top() + 32.0 + (i as f32) * stripe_gap;
                                let width = pw * if i == 2 { 0.5 } else { 0.72 };
                                let sr = egui::Rect::from_min_size(
                                    egui::pos2(rect.left() + 20.0, y),
                                    egui::vec2(width, stripe_h),
                                );
                                ui.painter().rect_filled(sr, egui::CornerRadius::same(4), stripe_color);
                            }

                            if is_visible { ui.ctx().request_repaint(); }
                        }
                    });

                    if idx + 1 < page_count { ui.add_space(PAGE_GAP); }
                }
                ui.add_space(PAGE_GAP);
            });
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn estimate_page_height(&self, available_w: f32) -> f32 {
        // Usa a primeira textura disponível no tex_cache para estimar
        if let Some(tex) = self.tex_cache.values().next() {
            let size = tex.size_vec2();
            let w = size.x.min(available_w - 48.0);
            return size.y * (w / size.x);
        }
        if let Some(ref tex) = self.texture {
            let size = tex.size_vec2();
            let w = size.x.min(available_w - 48.0);
            return size.y * (w / size.x);
        }
        1100.0
    }

}

// ── Helpers livres ────────────────────────────────────────────────────────────

fn upload_texture(ctx: &egui::Context, page: &RenderedPage, idx: usize) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [page.width as usize, page.height as usize],
        &page.rgba,
    );
    ctx.load_texture(
        format!("pdf_{idx}"),
        image,
        egui::TextureOptions::LINEAR,
    )
}

/// Calcula (display_w, display_h) respeitando max_w
fn fit_page(tex_size: egui::Vec2, max_w: f32) -> (f32, f32) {
    let w = tex_size.x.min(max_w.max(100.0));
    let h = tex_size.y * (w / tex_size.x);
    (w, h)
}

fn draw_page_tex(
    viewer: &mut egui::Rect,  // out: rect da página para o caller
    ui: &mut egui::Ui,
    tex: &egui::TextureHandle,
    size: egui::Vec2,
    tint: egui::Color32,
    border: egui::Color32,
) {
    let (_, rect) = ui.allocate_space(size);
    *viewer = rect;  // grava para hit-test de links
    // Cursor I-beam sobre a área da página
    if let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
        if rect.contains(pos) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }
    // Sombra suave e difusa (3 camadas de baixa opacidade, offsets crescentes)
    for (offset, alpha) in [(3.0_f32, 18_u8), (6.0, 12), (10.0, 6)] {
        ui.painter().rect_filled(
            rect.translate(egui::vec2(offset * 0.4, offset)),
            egui::CornerRadius::same(2),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, alpha),
        );
    }
    // Página com tint multiplicado (WHITE = sem alteração)
    ui.painter().image(
        tex.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        tint,
    );
    // Borda fina adaptada ao tema
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(0.5, border),
        egui::StrokeKind::Outside,
    );
}

fn show_spinner(ui: &mut egui::Ui, available_h: f32) {
    ui.vertical_centered(|ui| {
        ui.add_space(available_h / 2.0 - 24.0);
        ui.spinner();
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Renderizando...")
            .size(13.0).color(egui::Color32::from_rgb(90, 90, 110)));
    });
}

/// Inversão suave (dark mode PDF): mapeia branco → grafite escuro e preto → off-white.
/// Fórmula: `38 + ((255 - x) * 205 / 255)`, mantendo Alpha intacto.
/// - Branco (255) → 38 (grafite escuro, não preto absoluto)
/// - Preto  (0)   → 243 (off-white, não branco cru)
/// - Meios-tons preservam contraste de leitura.
pub fn invert_rgba(rgba: &mut Vec<u8>) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = 38 + ((255u16 - pixel[0] as u16) * 205 / 255) as u8; // R
        pixel[1] = 38 + ((255u16 - pixel[1] as u16) * 205 / 255) as u8; // G
        pixel[2] = 38 + ((255u16 - pixel[2] as u16) * 205 / 255) as u8; // B
        // pixel[3] = Alpha — não altera
    }
}
