// app_toolbar.rs — Barra de ferramentas do leitor PDF
// Extraído de app_reader.rs para manter abaixo de 250 linhas.

use eframe::egui;
use crate::app::MariApp;
use crate::app_constants::{ACCENT_PRIMARY, ACCENT_LIGHT};
use crate::app_color_mode::ColorMode;
use crate::app_reader::SidePanelKind;
use crate::ui::viewer::ViewMode;
use crate::ui::icons;

impl MariApp {
    pub(crate) fn show_toolbar(
        &mut self, ctx: &egui::Context,
        filename: &str, page_count: usize, current_page: usize,
        zoom_pct: u32, mode: ViewMode, color_mode: ColorMode,
        tb_alpha: u8, dw: f32,
    ) {
        let palette_idx = self.settings.palette.0;
        let accent = color_mode.accent_for(palette_idx);
        let hover_fill        = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 18);
        let hover_fill_strong = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 35);

        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::new()
                .fill({
                    let s = color_mode.surface_for(palette_idx);
                    egui::Color32::from_rgba_premultiplied(s.r(), s.g(), s.b(), ((s.a() as f32) * self.toolbar_opacity) as u8)
                })
                .stroke(egui::Stroke::new(dw, egui::Color32::from_rgba_premultiplied(
                    color_mode.border().r(), color_mode.border().g(), color_mode.border().b(), tb_alpha,
                )))
                .inner_margin(egui::Margin::symmetric(12, 5)))
            .show(ctx, |ui| {
                let make_icon_btn = |ic: &str, size: f32| -> egui::Button<'_> {
                    egui::Button::new(icons::ph(ic, size, color_mode.text_muted()))
                        .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE)
                };
                ui.horizontal(|ui| {
                    // ← Voltar
                    let r = ui.add(egui::Button::new(icons::ph(icons::ARROW_LEFT, 17.0, color_mode.text_muted()))
                        .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE))
                        .on_hover_text("Voltar (fecha PDF)");
                    if r.hovered() { ui.painter().rect_filled(r.rect, egui::CornerRadius::same(5), hover_fill_strong); ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                    if r.clicked() { self.save_session(); self.pending_close = true; }

                    ui.add_space(4.0); ui.separator(); ui.add_space(6.0);

                    // Side Panel
                    let toc_color   = if self.side_panel == SidePanelKind::Toc   { accent } else { color_mode.text_muted() };
                    let thumb_color = if self.side_panel == SidePanelKind::Thumbs { accent } else { color_mode.text_muted() };

                    let r_toc = ui.add(egui::Button::new(icons::ph(icons::LIST, 16.0, toc_color))
                        .fill(if self.side_panel == SidePanelKind::Toc { egui::Color32::from_rgba_unmultiplied(accent.r(),accent.g(),accent.b(),20) } else { egui::Color32::TRANSPARENT })
                        .stroke(egui::Stroke::NONE)).on_hover_text("Índice / TOC");
                    if r_toc.hovered() && self.side_panel != SidePanelKind::Toc { ui.painter().rect_filled(r_toc.rect, egui::CornerRadius::same(5), hover_fill); ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                    if r_toc.clicked() { self.side_panel = if self.side_panel == SidePanelKind::Toc { SidePanelKind::None } else { SidePanelKind::Toc }; }

                    let r_th = ui.add(egui::Button::new(icons::ph(icons::SQUARES_FOUR, 16.0, thumb_color))
                        .fill(if self.side_panel == SidePanelKind::Thumbs { egui::Color32::from_rgba_unmultiplied(accent.r(),accent.g(),accent.b(),20) } else { egui::Color32::TRANSPARENT })
                        .stroke(egui::Stroke::NONE)).on_hover_text("Miniaturas de página");
                    if r_th.hovered() && self.side_panel != SidePanelKind::Thumbs { ui.painter().rect_filled(r_th.rect, egui::CornerRadius::same(5), hover_fill); ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                    if r_th.clicked() { self.side_panel = if self.side_panel == SidePanelKind::Thumbs { SidePanelKind::None } else { SidePanelKind::Thumbs }; }

                    ui.add_space(4.0); ui.separator(); ui.add_space(6.0);

                    let name = if filename.len() > 40 { format!("…{}", &filename[filename.len()-38..]) } else { filename.to_string() };
                    ui.label(egui::RichText::new(name).size(13.0).color(color_mode.text_primary()));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Modo de cor
                        if ui.add(egui::Button::new(egui::RichText::new(color_mode.label()).size(11.0).color(color_mode.text_muted()))
                            .fill(color_mode.statusbar_for(palette_idx)).stroke(egui::Stroke::new(1.0, color_mode.border()))
                            .corner_radius(egui::CornerRadius::same(5))).on_hover_text("Alternar modo de cor").clicked() {
                            self.color_mode = self.color_mode.next();
                            self.viewer.tex_cache.clear(); self.viewer.texture = None;
                            self.viewer.loaded_page = None; self.pending_rerender = true;
                        }
                        ui.add_space(2.0);

                        // Configurações
                        let rs = ui.add(egui::Button::new(icons::ph(icons::GEAR, 15.0, color_mode.text_muted()))
                            .fill(color_mode.statusbar_for(palette_idx)).stroke(egui::Stroke::new(1.0, color_mode.border()))
                            .corner_radius(egui::CornerRadius::same(5))).on_hover_text("Configurações");
                        if rs.hovered() { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                        if rs.clicked() { self.settings_open = !self.settings_open; }
                        ui.add_space(4.0);

                        // Toggle modo
                        let (mode_icon, hint_mode) = match mode {
                            ViewMode::SinglePage => (icons::SQUARE_HALF, "Contínuo (Ctrl+Shift+M)"),
                            ViewMode::Continuous  => (icons::ROWS,       "Página única (Ctrl+Shift+M)"),
                        };
                        if ui.add(egui::Button::new(icons::ph(mode_icon, 15.0, color_mode.text_muted()))
                            .fill(color_mode.statusbar_for(palette_idx)).stroke(egui::Stroke::new(1.0, color_mode.border()))
                            .corner_radius(egui::CornerRadius::same(5))).on_hover_text(hint_mode).clicked() {
                            let c = ctx.clone(); self.do_toggle_mode(&c);
                        }
                        ui.add_space(4.0); ui.separator(); ui.add_space(4.0);

                        // Fit Page / Fit Width
                        let r_fw = ui.add(make_icon_btn(icons::FRAME_CORNERS, 15.0)).on_hover_text("Fit Page (Ctrl+0)");
                        if r_fw.hovered() { ui.painter().rect_filled(r_fw.rect, egui::CornerRadius::same(5), hover_fill); ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                        if r_fw.clicked() { let c = ctx.clone(); self.fit_page(&c); }
                        let r_fh = ui.add(make_icon_btn(icons::ARROWS_OUT, 15.0)).on_hover_text("Fit Width (W)");
                        if r_fh.hovered() { ui.painter().rect_filled(r_fh.rect, egui::CornerRadius::same(5), hover_fill); ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                        if r_fh.clicked() { let c = ctx.clone(); self.fit_width(&c); }
                        ui.add_space(4.0); ui.separator(); ui.add_space(4.0);

                        // Zoom
                        ui.label(egui::RichText::new(format!("{zoom_pct}%")).size(12.0).color(color_mode.text_muted()).monospace());
                        ui.add_space(2.0);
                        let r_zm = ui.add(make_icon_btn(icons::MINUS, 15.0));
                        if r_zm.hovered() { ui.painter().rect_filled(r_zm.rect, egui::CornerRadius::same(5), hover_fill); }
                        if r_zm.clicked() { self.apply_zoom(ctx, -0.15); }
                        let r_zr = ui.add(make_icon_btn(icons::ARROW_COUNTER_CW, 15.0)).on_hover_text("100%");
                        if r_zr.hovered() { ui.painter().rect_filled(r_zr.rect, egui::CornerRadius::same(5), hover_fill); }
                        if r_zr.clicked() { self.set_zoom(ctx, 1.0); }
                        let r_zp = ui.add(make_icon_btn(icons::PLUS, 15.0));
                        if r_zp.hovered() { ui.painter().rect_filled(r_zp.rect, egui::CornerRadius::same(5), hover_fill); }
                        if r_zp.clicked() { self.apply_zoom(ctx, 0.15); }
                        ui.add_space(4.0); ui.separator(); ui.add_space(4.0);

                        // Navegação
                        if ui.add(egui::Button::new(egui::RichText::new(format!("{}/{}", current_page + 1, page_count))
                            .size(12.0).color(color_mode.text_muted()).monospace())
                            .fill(color_mode.statusbar_for(palette_idx)).stroke(egui::Stroke::new(1.0, color_mode.border()))
                            .corner_radius(egui::CornerRadius::same(4))).on_hover_text("Ir para página (Ctrl+G)").clicked() {
                            self.goto_open = true; self.goto_input.clear();
                        }
                        ui.add_space(2.0);
                        let r_cn = ui.add(make_icon_btn(icons::CARET_RIGHT, 15.0)).on_hover_text("Próxima");
                        if r_cn.hovered() { ui.painter().rect_filled(r_cn.rect, egui::CornerRadius::same(5), hover_fill); }
                        if r_cn.clicked() && current_page + 1 < page_count { let p = current_page + 1; let c = ctx.clone(); self.go_to_page(&c, p); }
                        let r_cp = ui.add(make_icon_btn(icons::CARET_LEFT, 15.0)).on_hover_text("Anterior");
                        if r_cp.hovered() { ui.painter().rect_filled(r_cp.rect, egui::CornerRadius::same(5), hover_fill); }
                        if r_cp.clicked() && current_page > 0 { let p = current_page - 1; let c = ctx.clone(); self.go_to_page(&c, p); }
                    });
                });
            });
    }
}
