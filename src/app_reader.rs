// app_reader.rs — UI do leitor PDF: toolbar, status bar, side panel e área de leitura
// Extraído de app.rs (L1304-1969).

use eframe::egui;
use crate::app::{MariApp, ACCENT_PRIMARY, ACCENT_LIGHT};
use crate::app_color_mode::ColorMode;
use crate::ui::viewer::ViewMode;
use crate::ui::side_panel;
use crate::ui::side_panel::SIDE_PANEL_WIDTH;
use crate::ui::icons;
use crate::pdf::engine::{TextCharInfo};

/// Tipo de painel lateral
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidePanelKind { None, Toc, Thumbs }

impl MariApp {
    pub(crate) fn ui_reader(&mut self, ctx: &egui::Context) {
        let path  = self.current_file.clone().unwrap();
        let filename = path.file_name()
            .and_then(|n| n.to_str()).unwrap_or("Documento").to_string();
        let page_count   = self.page_count;
        let current_page = self.current_page;
        let zoom_pct     = (self.viewer.zoom * 100.0) as u32;
        let mode         = self.viewer.mode;
        let color_mode   = self.color_mode;
        let fs           = self.fullscreen;

        // Toolbar fade suave baseado na posição do mouse
        {
            let mouse_y = ctx.input(|i| i.pointer.hover_pos().map(|p| p.y).unwrap_or(999.0));
            let target_opacity: f32 = if mouse_y < 52.0 { 1.0 } else { 0.18 };
            let dt    = ctx.input(|i| i.unstable_dt).min(0.1);
            let speed = if target_opacity > self.toolbar_opacity { 10.0 } else { 4.0 };
            self.toolbar_opacity += (target_opacity - self.toolbar_opacity) * speed * dt;
            self.toolbar_opacity = self.toolbar_opacity.clamp(0.0, 1.0);
            if (self.toolbar_opacity - target_opacity).abs() > 0.005 { ctx.request_repaint(); }
        }
        let tb_alpha = (self.toolbar_opacity * 255.0) as u8;
        let dw = color_mode.divider_width();

        // ── Toolbar ──────────────────────────────────────────────────────────
        if !fs {
            self.show_toolbar(ctx, &filename, page_count, current_page, zoom_pct, mode, color_mode, tb_alpha, dw);
        }

        // ── Status Bar ───────────────────────────────────────────────────────
        if !fs {
            self.show_status_bar(ctx, &filename, page_count, current_page, zoom_pct, mode, color_mode);
        }

        // ── Side Panel ───────────────────────────────────────────────────────
        self.show_side_panel(ctx, color_mode, current_page);

        // ── Área Central de Leitura ──────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(color_mode.bg_for(self.settings.palette.0)))
            .show(ctx, |ui| {
                self.available_width = ui.available_width();
                if let Some(ref err) = self.render_error.clone() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.0 - 20.0);
                        ui.label(egui::RichText::new(format!("⚠ {err}"))
                            .size(14.0).color(egui::Color32::from_rgb(239, 68, 68)));
                    });
                } else if let Some(engine) = &self.engine {
                    let pc     = self.page_count;
                    let cp     = self.current_page;
                    let tint   = self.color_mode.page_tint_for(self.settings.palette.0);
                    let border = self.color_mode.page_border();
                    self.viewer.invert_colors = matches!(self.color_mode, ColorMode::Dark);
                    let viewer_ptr: *mut crate::ui::viewer::PdfViewer = &mut self.viewer;
                    // SAFETY: engine e viewer são campos distintos de MariApp — sem aliasing
                    unsafe { (*viewer_ptr).show(ui, engine, cp, pc, tint, border) };

                    let page_rect = self.viewer.last_page_rect;
                    if page_rect.is_positive() {
                        if self.text_chars_page != cp {
                            self.text_chars      = engine.extract_chars_with_bounds(cp).unwrap_or_default();
                            self.text_chars_page = cp;
                            self.sel_anchor      = None;
                            self.sel_focus       = None;
                        }
                        if !self.text_chars.is_empty() {
                            let (pdf_w, pdf_h) = engine.page_size_pts(cp);
                            self.handle_text_selection(ctx, ui, page_rect, pdf_w, pdf_h);
                        }
                    }
                }
            });

        // Hint tela cheia
        if fs {
            egui::Area::new(egui::Id::new("fs_hint"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("F11 → sair da tela cheia")
                        .size(11.0).color(egui::Color32::from_rgba_premultiplied(200, 200, 220, 100)));
                });
        }
    }

    // ── Toolbar ──────────────────────────────────────────────────────────────
    fn show_toolbar(
        &mut self, ctx: &egui::Context,
        filename: &str, page_count: usize, current_page: usize,
        zoom_pct: u32, mode: ViewMode, color_mode: ColorMode,
        tb_alpha: u8, dw: f32,
    ) {
        let palette_idx = self.settings.palette.0;
        let accent = color_mode.accent_for(palette_idx);
        let hover_fill = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 18);
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

                    // Side Panel buttons
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
                            ViewMode::Continuous => (icons::ROWS,        "Página única (Ctrl+Shift+M)"),
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

    // ── Status Bar ───────────────────────────────────────────────────────────
    fn show_status_bar(&mut self, ctx: &egui::Context, filename: &str, page_count: usize, current_page: usize, zoom_pct: u32, mode: ViewMode, color_mode: ColorMode) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::new()
                .fill(color_mode.statusbar_for(self.settings.palette.0))
                .stroke(egui::Stroke::new(1.0, color_mode.border()))
                .inner_margin(egui::Margin::symmetric(14, 3)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.viewer.is_loading {
                        ui.label(icons::ph(icons::SPINNER_GAP, 11.0, ACCENT_LIGHT));
                        ui.label(egui::RichText::new("Renderizando...").size(11.0).color(ACCENT_LIGHT));
                    } else {
                        ui.label(icons::ph(icons::CHECK, 11.0, color_mode.text_subtle()));
                        ui.label(egui::RichText::new("Pronto").size(11.0).color(color_mode.text_subtle()));
                    }
                    ui.add_space(12.0); ui.separator(); ui.add_space(12.0);
                    let short = if filename.len() > 32 { format!("…{}", &filename[filename.len()-30..]) } else { filename.to_string() };
                    ui.label(egui::RichText::new(&short).size(11.0).color(color_mode.text_subtle()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_str = match mode { ViewMode::SinglePage => "▣ Página única", ViewMode::Continuous => "☰ Contínuo" };
                        ui.label(egui::RichText::new(mode_str).size(11.0).color(color_mode.text_subtle()));
                        ui.add_space(10.0); ui.separator(); ui.add_space(10.0);
                        ui.label(egui::RichText::new(format!("{zoom_pct}%")).size(11.0).color(color_mode.text_subtle()).monospace());
                        ui.add_space(10.0); ui.separator(); ui.add_space(10.0);
                        ui.label(egui::RichText::new(format!("Pág. {} / {}", current_page + 1, page_count)).size(11.0).color(color_mode.text_muted()).monospace());
                    });
                });
            });
    }

    // ── Side Panel ───────────────────────────────────────────────────────────
    fn show_side_panel(&mut self, ctx: &egui::Context, color_mode: ColorMode, current_page: usize) {
        if self.side_panel == SidePanelKind::None { return; }

        let sp_title = match self.side_panel { SidePanelKind::Toc => "Índice", SidePanelKind::Thumbs => "Páginas", SidePanelKind::None => "" };
        let panel_bg    = color_mode.surface_for(self.settings.palette.0);
        let panel_bord  = color_mode.border();
        let panel_text  = color_mode.text_primary();
        let panel_muted = color_mode.text_muted();
        let mut goto_page: Option<usize> = None;

        egui::SidePanel::left("side_panel")
            .resizable(true).default_width(SIDE_PANEL_WIDTH).width_range(150.0..=380.0)
            .frame(egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, panel_bord)).inner_margin(egui::Margin::symmetric(6, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label(egui::RichText::new(sp_title).size(12.0).strong().color(panel_text)); });
                ui.add(egui::Separator::default().spacing(6.0));
                match self.side_panel {
                    SidePanelKind::Toc => {
                        let bookmarks = self.bookmarks.clone();
                        let pc = self.thumb_cache.as_ref().map_or(self.bookmarks.len(), |tc| tc.page_count);
                        goto_page = side_panel::show_toc(ui, &bookmarks, current_page, pc, panel_bg, panel_text, panel_muted, ACCENT_PRIMARY);
                    }
                    SidePanelKind::Thumbs => {
                        if let Some(ref mut tc) = self.thumb_cache {
                            goto_page = side_panel::show_thumbs(ui, tc, current_page, panel_bg, panel_muted, panel_text, ACCENT_PRIMARY,
                                matches!(self.color_mode, ColorMode::Dark), self.color_mode.page_tint());
                        }
                    }
                    SidePanelKind::None => {}
                }
            });

        if let Some(p) = goto_page { let c = ctx.clone(); self.go_to_page(&c, p); }
    }

    // ── Seleção de Texto ─────────────────────────────────────────────────────
    fn handle_text_selection(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, page_rect: egui::Rect, pdf_w: f32, pdf_h: f32) {
        let chars = &self.text_chars;

        let screen_to_pdf = |pos: egui::Pos2| -> (f32, f32) {
            let nx = (pos.x - page_rect.left()) / page_rect.width().max(1.0);
            let ny = 1.0 - (pos.y - page_rect.top()) / page_rect.height().max(1.0);
            (nx * pdf_w, ny * pdf_h)
        };
        let nearest_idx = |pos: egui::Pos2| -> usize {
            let (px, py) = screen_to_pdf(pos);
            chars.iter().enumerate()
                .min_by_key(|(_, c)| {
                    let cx = (c.left + c.right)  * 0.5;
                    let cy = (c.bottom + c.top)  * 0.5;
                    (((cx - px).abs() + (cy - py).abs() * 3.0) * 1000.0) as i64
                }).map(|(i, _)| i).unwrap_or(0)
        };

        let drag_resp = ui.interact(page_rect, egui::Id::new("text_sel_drag"), egui::Sense::click_and_drag());

        if drag_resp.drag_started() {
            let pos = drag_resp.interact_pointer_pos().unwrap_or(page_rect.center());
            self.sel_anchor = Some(nearest_idx(pos)); self.sel_focus = self.sel_anchor; self.selected_text = None;
        }
        if drag_resp.dragged() {
            if let Some(pos) = drag_resp.interact_pointer_pos() { self.sel_focus = Some(nearest_idx(pos)); }
        }
        if drag_resp.drag_stopped() {
            if let (Some(a), Some(f)) = (self.sel_anchor, self.sel_focus) {
                let lo = a.min(f);
                let hi = (a.max(f) + 1).min(chars.len());
                let mut text = String::new();
                let mut last_r = f32::MIN;
                for ch in &chars[lo..hi] {
                    if ch.left > last_r + 1.5 && !text.is_empty() { text.push(' '); }
                    text.push(ch.ch); last_r = ch.right;
                }
                let clean = text.trim().to_string();
                if !clean.is_empty() {
                    ctx.copy_text(clean.clone());
                    self.selected_text = Some(clean);
                    let now = ctx.input(|i| i.time);
                    use crate::app::Toast;
                    self.toast = Some(Toast::info("Texto copiado!", 2.0, now));
                }
            }
        }
        if drag_resp.clicked() { self.sel_anchor = None; self.sel_focus = None; self.selected_text = None; }
        if drag_resp.hovered()  { ctx.set_cursor_icon(egui::CursorIcon::Text); }

        // Highlight de seleção
        if let (Some(a), Some(f)) = (self.sel_anchor, self.sel_focus) {
            let lo = a.min(f);
            let hi = (a.max(f) + 1).min(chars.len());
            if lo < hi {
                let char_rect = |ch: &TextCharInfo| -> egui::Rect {
                    let lx = page_rect.left() + ch.left  / pdf_w * page_rect.width();
                    let rx = page_rect.left() + ch.right / pdf_w * page_rect.width();
                    let ty = page_rect.bottom() - ch.top    / pdf_h * page_rect.height();
                    let by = page_rect.bottom() - ch.bottom / pdf_h * page_rect.height();
                    egui::Rect::from_min_max(egui::pos2(lx, ty), egui::pos2(rx, by))
                };
                let mut line_rects: Vec<egui::Rect> = Vec::new();
                let mut cur_rect: Option<egui::Rect> = None;
                let mut cur_cy = f32::NAN;
                const LINE_MERGE_PX: f32 = 5.0;
                for ch in &chars[lo..hi] {
                    let r = char_rect(ch);
                    if r.width() <= 0.0 || r.height() <= 0.0 { continue; }
                    let cy = r.center().y;
                    if cur_rect.is_none() || (cy - cur_cy).abs() > LINE_MERGE_PX {
                        if let Some(prev) = cur_rect { line_rects.push(prev); }
                        cur_rect = Some(r); cur_cy = cy;
                    } else { cur_rect = Some(cur_rect.unwrap().union(r)); }
                }
                if let Some(last) = cur_rect { line_rects.push(last); }

                let hl_color = match self.color_mode {
                    ColorMode::Dark  => egui::Color32::from_rgba_unmultiplied(55, 130, 255, 55),
                    _                => egui::Color32::from_rgba_unmultiplied(30, 100, 220, 65),
                };
                let painter = ui.painter();
                for r in &line_rects { painter.rect_filled(*r, egui::CornerRadius::ZERO, hl_color); }

                // Handles de seleção
                let handle_color = egui::Color32::from_rgba_unmultiplied(55, 130, 255, 230);
                let handle_r = 5.0_f32;
                let stem_h   = 12.0_f32;
                if let (Some(first_r), Some(last_r)) = (line_rects.first(), line_rects.last()) {
                    let start_pt = egui::pos2(first_r.left(), first_r.top());
                    painter.line_segment([start_pt, egui::pos2(start_pt.x, start_pt.y - stem_h)], egui::Stroke::new(2.0, handle_color));
                    painter.circle_filled(egui::pos2(start_pt.x, start_pt.y - stem_h - handle_r), handle_r, handle_color);
                    let sh_rect = egui::Rect::from_center_size(egui::pos2(start_pt.x, start_pt.y - stem_h - handle_r), egui::vec2(handle_r * 3.5, handle_r * 3.5));
                    let sh_resp = ui.interact(sh_rect, egui::Id::new("sel_handle_start"), egui::Sense::drag());
                    if sh_resp.dragged() { if let Some(pos) = sh_resp.interact_pointer_pos() { self.sel_anchor = Some(nearest_idx(pos)); } }
                    if sh_resp.hovered() { ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }

                    let end_pt = egui::pos2(last_r.right(), last_r.bottom());
                    painter.line_segment([end_pt, egui::pos2(end_pt.x, end_pt.y + stem_h)], egui::Stroke::new(2.0, handle_color));
                    painter.circle_filled(egui::pos2(end_pt.x, end_pt.y + stem_h + handle_r), handle_r, handle_color);
                    let eh_rect = egui::Rect::from_center_size(egui::pos2(end_pt.x, end_pt.y + stem_h + handle_r), egui::vec2(handle_r * 3.5, handle_r * 3.5));
                    let eh_resp = ui.interact(eh_rect, egui::Id::new("sel_handle_end"), egui::Sense::drag());
                    if eh_resp.dragged() { if let Some(pos) = eh_resp.interact_pointer_pos() { self.sel_focus = Some(nearest_idx(pos)); } }
                    if eh_resp.hovered() { ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
                }
            }
        }
    }
}
