// app_nav.rs — Renderização, zoom, fit e troca de modo de visualização
// Extraído de app.rs para manter abaixo de 250 linhas.

use crate::app::{MariApp, MAX_ZOOM, MIN_ZOOM};
use crate::app_color_mode::ColorMode;
use crate::ui::viewer::{invert_rgba, ViewMode};
use eframe::egui;

impl MariApp {
    pub fn render_current_page(&mut self, ctx: &egui::Context) {
        self.viewer.is_loading = true;
        let Some(engine) = &self.engine else {
            self.viewer.is_loading = false;
            return;
        };
        let scale = self.viewer.zoom * self.settings.render_quality.scale();
        match engine.render_page(self.current_page, scale) {
            Ok(mut rendered) => {
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

    pub fn go_to_page(&mut self, ctx: &egui::Context, page: usize) {
        let page = page.min(self.page_count.saturating_sub(1));
        if self.current_page == page && self.viewer.texture.is_some() {
            return;
        }
        self.current_page = page;
        if self.viewer.mode == ViewMode::SinglePage {
            self.render_current_page(ctx);
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

    pub fn invalidate_render(&mut self, ctx: &egui::Context) {
        if self.viewer.mode == ViewMode::SinglePage {
            self.viewer.texture = None;
            self.render_current_page(ctx);
        } else {
            self.viewer.tex_cache.clear();
            self.viewer.page_cache.clear();
            if let Some(path) = self.current_file.clone() {
                let scale = self.viewer.zoom * self.settings.render_quality.scale();
                self.viewer.init_cache(path, scale);
            }
        }
    }

    pub fn fit_width(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        let (page_w, _) = engine.page_size_pts(self.current_page);
        let margin = self.settings.page_margin.px() * 2.0;
        let usable = (self.available_width - margin).max(100.0);
        self.set_zoom(ctx, usable / page_w.max(1.0));
    }

    pub fn fit_page(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        let (page_w, page_h) = engine.page_size_pts(self.current_page);
        let margin = self.settings.page_margin.px() * 2.0;
        let usable_w = (self.available_width - margin).max(100.0);
        let usable_h = (ctx.screen_rect().height() - margin).max(100.0);
        let zoom_w = usable_w / page_w.max(1.0);
        let zoom_h = usable_h / page_h.max(1.0);
        self.set_zoom(ctx, zoom_w.min(zoom_h));
    }

    pub fn do_toggle_mode(&mut self, ctx: &egui::Context) {
        match self.viewer.mode {
            ViewMode::SinglePage => {
                self.viewer.mode = ViewMode::Continuous;
                self.viewer.texture = None;
                if let Some(path) = self.current_file.clone() {
                    let scale = self.viewer.zoom * self.settings.render_quality.scale();
                    self.viewer.init_cache(path, scale);
                }
            }
            ViewMode::Continuous => {
                self.viewer.mode = ViewMode::SinglePage;
                self.viewer.tex_cache.clear();
                self.viewer.page_cache.clear();
                let c = ctx.clone();
                self.render_current_page(&c);
            }
        }
    }
}
