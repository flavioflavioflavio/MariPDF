// app_text_sel.rs — Lógica de seleção de texto sobre o PDF
// Extraído de app_reader.rs para manter abaixo de 250 linhas.

use eframe::egui;
use crate::app::{MariApp, Toast};
use crate::app_color_mode::ColorMode;
use crate::pdf::engine::TextCharInfo;

impl MariApp {
    pub(crate) fn handle_text_selection(
        &mut self, ctx: &egui::Context, ui: &mut egui::Ui,
        page_rect: egui::Rect, pdf_w: f32, pdf_h: f32,
    ) {
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
                    self.toast = Some(Toast::info("Texto copiado!", 2.0, now));
                }
            }
        }
        if drag_resp.clicked() { self.sel_anchor = None; self.sel_focus = None; self.selected_text = None; }
        if drag_resp.hovered()  { ctx.set_cursor_icon(egui::CursorIcon::Text); }

        // ── Highlight de seleção ──────────────────────────────────────────────
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
                    ColorMode::Dark => egui::Color32::from_rgba_unmultiplied(55, 130, 255, 55),
                    _               => egui::Color32::from_rgba_unmultiplied(30, 100, 220, 65),
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
