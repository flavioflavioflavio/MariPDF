// app_reader.rs — UI do leitor PDF: área central, status bar e side panel
// Toolbar extraída para app_toolbar.rs. Seleção de texto para app_text_sel.rs.

use crate::app::{MariApp, ACCENT_LIGHT, ACCENT_PRIMARY};
use crate::app_color_mode::ColorMode;
use crate::ui::icons;
use crate::ui::side_panel;
use crate::ui::side_panel::SIDE_PANEL_WIDTH;
use crate::ui::viewer::ViewMode;
use eframe::egui;

/// Tipo de painel lateral
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidePanelKind {
    None,
    Toc,
    Thumbs,
}

impl MariApp {
    pub(crate) fn ui_reader(&mut self, ctx: &egui::Context) {
        self.sync_raster_density(ctx);
        if self.frame.active {
            self.ui_frame(ctx);
            return;
        }
        let path = self.current_file.clone().unwrap();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| crate::t!("doc-untitled"));
        let page_count = self.page_count;
        let current_page = self.current_page;
        let zoom_pct = (self.viewer.zoom * 100.0) as u32;
        let mode = self.viewer.mode;
        let color_mode = self.color_mode;
        let fs = self.fullscreen;

        // Toolbar fade suave baseado na posição do mouse
        {
            let mouse_y = ctx.input(|i| i.pointer.hover_pos().map(|p| p.y).unwrap_or(999.0));
            let target_opacity: f32 = if mouse_y < 52.0 { 1.0 } else { 0.18 };
            let dt = ctx.input(|i| i.unstable_dt).min(0.1);
            let speed = if target_opacity > self.toolbar_opacity {
                10.0
            } else {
                4.0
            };
            self.toolbar_opacity += (target_opacity - self.toolbar_opacity) * speed * dt;
            self.toolbar_opacity = self.toolbar_opacity.clamp(0.0, 1.0);
            if (self.toolbar_opacity - target_opacity).abs() > 0.005 {
                ctx.request_repaint();
            }
        }
        let tb_alpha = (self.toolbar_opacity * 255.0) as u8;
        let dw = color_mode.divider_width();

        if !fs {
            self.show_toolbar(
                ctx,
                &filename,
                page_count,
                current_page,
                zoom_pct,
                mode,
                color_mode,
                tb_alpha,
                dw,
            );
        }
        if !fs {
            self.show_status_bar(
                ctx,
                &filename,
                page_count,
                current_page,
                zoom_pct,
                mode,
                color_mode,
            );
        }
        self.show_side_panel(ctx, color_mode, current_page);

        // ── Área Central de Leitura ───────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(color_mode.bg_for(self.settings.palette.0)))
            .show(ctx, |ui| {
                self.available_width = ui.available_width();
                if let Some(ref err) = self.render_error.clone() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.0 - 20.0);
                        ui.label(
                            egui::RichText::new(format!("⚠ {err}"))
                                .size(14.0)
                                .color(egui::Color32::from_rgb(239, 68, 68)),
                        );
                    });
                } else if let Some(engine) = &self.engine {
                    let pc = self.page_count;
                    let cp = self.current_page;
                    let tint = self.color_mode.page_tint_for(self.settings.palette.0);
                    let border = self.color_mode.page_border();
                    self.viewer.invert_colors = matches!(self.color_mode, ColorMode::Dark);
                    self.viewer.page_margin = self.settings.page_margin.px();
                    self.viewer.smooth_scroll = self.settings.smooth_scroll;
                    self.viewer.search_highlight = self.active_search_highlight();
                    self.viewer.show(ui, cp, pc, tint, border);

                    if self.viewer.mode == ViewMode::Continuous {
                        if let Some(visible_page) = self.viewer.dominant_continuous_page() {
                            if self.current_page != visible_page {
                                if self.current_page.abs_diff(visible_page) > 32 {
                                    tracing::info!(
                                        "continuous: salto para página dominante {}",
                                        visible_page + 1
                                    );
                                } else {
                                    tracing::debug!(
                                        "continuous: página dominante {}",
                                        visible_page + 1
                                    );
                                }
                                self.current_page = visible_page;
                            }
                        }
                    }

                    if let Some(error) = self.viewer.take_error() {
                        self.render_error = Some(error);
                    }

                    let page_rect = self.viewer.last_page_rect;
                    if self.viewer.mode == ViewMode::SinglePage
                        && self.viewer.loaded_page == Some(cp)
                        && page_rect.is_positive()
                    {
                        if self.text_chars_page != cp {
                            if let Some(worker) = self.document_worker.as_mut() {
                                worker.request_chars(cp);
                            }
                        }
                        if self.text_chars_page == cp && !self.text_chars.is_empty() {
                            let (pdf_w, pdf_h) = engine.page_size_pts(cp);
                            self.handle_text_selection(ctx, ui, page_rect, pdf_w, pdf_h);
                        }
                    }
                }
            });

        if fs {
            egui::Area::new(egui::Id::new("fs_hint"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(crate::t!("reader-fullscreen-hint"))
                            .size(11.0)
                            .color(egui::Color32::from_rgba_premultiplied(200, 200, 220, 100)),
                    );
                });
        }
    }

    // ── Status Bar ────────────────────────────────────────────────────────────
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn show_status_bar(
        &mut self,
        ctx: &egui::Context,
        filename: &str,
        page_count: usize,
        current_page: usize,
        zoom_pct: u32,
        mode: ViewMode,
        color_mode: ColorMode,
    ) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::new()
                    .fill(color_mode.statusbar_for(self.settings.palette.0))
                    .stroke(egui::Stroke::new(1.0, color_mode.border()))
                    .inner_margin(egui::Margin::symmetric(14, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.viewer.is_loading {
                        ui.label(icons::ph(icons::SPINNER_GAP, 11.0, ACCENT_LIGHT));
                        ui.label(
                            egui::RichText::new(crate::t!("status-rendering"))
                                .size(11.0)
                                .color(ACCENT_LIGHT),
                        );
                    } else {
                        ui.label(icons::ph(icons::CHECK, 11.0, color_mode.text_subtle()));
                        ui.label(
                            egui::RichText::new(crate::t!("status-ready"))
                                .size(11.0)
                                .color(color_mode.text_subtle()),
                        );
                    }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);
                    let short = if filename.len() > 32 {
                        format!("…{}", &filename[filename.len() - 30..])
                    } else {
                        filename.to_string()
                    };
                    ui.label(
                        egui::RichText::new(&short)
                            .size(11.0)
                            .color(color_mode.text_subtle()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_str = crate::t!(match mode {
                            ViewMode::SinglePage => "status-mode-single",
                            ViewMode::Continuous => "status-mode-continuous",
                        });
                        ui.label(
                            egui::RichText::new(mode_str)
                                .size(11.0)
                                .color(color_mode.text_subtle()),
                        );
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("{zoom_pct}%"))
                                .size(11.0)
                                .color(color_mode.text_subtle())
                                .monospace(),
                        );
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(crate::t!(
                                "status-page",
                                current => current_page + 1,
                                total => page_count,
                            ))
                            .size(11.0)
                            .color(color_mode.text_muted())
                            .monospace(),
                        );
                    });
                });
            });
    }

    // ── Side Panel ────────────────────────────────────────────────────────────
    pub(crate) fn show_side_panel(
        &mut self,
        ctx: &egui::Context,
        color_mode: ColorMode,
        current_page: usize,
    ) {
        use crate::app_reader::SidePanelKind;
        if self.side_panel == SidePanelKind::None {
            return;
        }

        let sp_title = match self.side_panel {
            SidePanelKind::Toc => crate::t!("side-toc-title"),
            SidePanelKind::Thumbs => crate::t!("side-thumbs-title"),
            SidePanelKind::None => String::new(),
        };
        let panel_bg = color_mode.surface_for(self.settings.palette.0);
        let panel_bord = color_mode.border();
        let panel_text = color_mode.text_primary();
        let panel_muted = color_mode.text_muted();
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
                        goto_page = side_panel::show_toc(
                            ui,
                            &bookmarks,
                            current_page,
                            self.page_count,
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
                                current_page,
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

        if let Some(p) = goto_page {
            let c = ctx.clone();
            self.go_to_page(&c, p);
        }
    }
}
