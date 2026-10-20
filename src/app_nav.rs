// app_nav.rs — Renderização, zoom, fit e troca de modo de visualização
// Extraído de app.rs para manter abaixo de 250 linhas.

use crate::app::{MariApp, MAX_ZOOM, MIN_ZOOM};
use crate::ui::viewer::ViewMode;
use eframe::egui;

impl MariApp {
    pub fn render_current_page(&mut self, ctx: &egui::Context) {
        if self.engine.is_none() {
            return;
        }
        self.render_error = None;
        // Antes do 1º frame do viewer, a largura do painel central (ou o seu
        // padrão) é a melhor estimativa da área útil para limitar o bitmap.
        if self.viewer.usable_width <= 0.0 {
            self.viewer.usable_width =
                (self.available_width - self.settings.page_margin.px() * 2.0).max(0.0);
        }
        self.viewer.page_margin = self.settings.page_margin.px();
        self.viewer.smooth_scroll = self.settings.smooth_scroll;
        self.viewer
            .request_single_page(self.current_page, self.page_count);
        ctx.request_repaint();
    }

    pub fn go_to_page(&mut self, ctx: &egui::Context, page: usize) {
        let page = page.min(self.page_count.saturating_sub(1));
        if self.current_page == page
            && self.viewer.mode == ViewMode::SinglePage
            && self.viewer.texture.is_some()
        {
            return;
        }
        self.current_page = page;
        match self.viewer.mode {
            ViewMode::SinglePage => self.render_current_page(ctx),
            ViewMode::Continuous => {
                self.viewer.request_continuous_page(page);
                ctx.request_repaint();
            }
        }
    }

    pub fn apply_zoom(&mut self, ctx: &egui::Context, delta: f32) {
        let new_zoom = (self.viewer.zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_zoom - self.viewer.zoom).abs() > 0.005 {
            self.viewer.zoom = new_zoom;
            self.invalidate_render(ctx);
        }
    }

    pub fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if (self.viewer.zoom - zoom).abs() > 0.005 {
            self.viewer.zoom = zoom;
            self.invalidate_render(ctx);
        }
    }

    /// Mantém a densidade de rasterização (DPI da tela × qualidade) em dia;
    /// bitmaps em cache de outra densidade são descartados.
    pub(crate) fn sync_raster_density(&mut self, ctx: &egui::Context) {
        let density = ctx.pixels_per_point() * self.settings.render_quality.scale();
        if (self.viewer.render_scale - density).abs() > 0.01 {
            self.viewer.render_scale = density;
            if self.engine.is_some() {
                self.invalidate_render(ctx);
            }
        }
    }

    pub fn invalidate_render(&mut self, ctx: &egui::Context) {
        self.viewer.tex_cache.clear();
        self.viewer.loaded_page = None;
        if let Some(worker) = &self.document_worker {
            self.viewer.init_cache(worker.handle(), self.page_count);
        }
        if self.viewer.mode == ViewMode::SinglePage {
            self.render_current_page(ctx);
        }
    }

    pub fn fit_width(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        let (page_w, _) = engine.page_size_pts(self.current_page);
        let margin = self.settings.page_margin.px() * 2.0;
        let usable = (self.available_width - margin).max(100.0);
        self.set_zoom(
            ctx,
            usable / crate::ui::page_scale::display_scale(page_w.max(1.0)),
        );
    }

    pub fn fit_page(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        let (page_w, page_h) = engine.page_size_pts(self.current_page);
        let margin = self.settings.page_margin.px() * 2.0;
        let usable_w = (self.available_width - margin).max(100.0);
        let usable_h = (ctx.screen_rect().height() - margin).max(100.0);
        let zoom_w = usable_w / crate::ui::page_scale::display_scale(page_w.max(1.0));
        let zoom_h = usable_h / crate::ui::page_scale::display_scale(page_h.max(1.0));
        self.set_zoom(ctx, zoom_w.min(zoom_h));
    }

    pub fn do_toggle_mode(&mut self, ctx: &egui::Context) {
        match self.viewer.mode {
            ViewMode::SinglePage => {
                tracing::info!("viewer: modo contínuo");
                self.viewer.mode = ViewMode::Continuous;
                self.viewer.texture = None;
                self.viewer.request_continuous_page(self.current_page);
                ctx.request_repaint();
            }
            ViewMode::Continuous => {
                tracing::info!("viewer: modo página única");
                self.viewer.mode = ViewMode::SinglePage;
                self.viewer.tex_cache.clear();
                self.viewer.loaded_page = None;
                let c = ctx.clone();
                self.render_current_page(&c);
            }
        }
    }
}
