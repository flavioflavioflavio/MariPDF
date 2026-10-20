// app_text_sel.rs — Seleção e cópia de texto sobre a página (BF-411).
//
// Contrato:
//   • arrastar seleciona (não copia sozinho); Shift+clique estende;
//   • duplo clique seleciona a palavra, triplo clique a linha;
//   • Ctrl+A seleciona a página inteira; Ctrl+C copia; Esc/clique limpam;
//   • botão direito oferece Copiar e Selecionar tudo.
// As regras de texto/palavra/linha vivem em text_selection.rs (testadas).

use crate::app::{MariApp, Toast};
use crate::app_color_mode::ColorMode;
use crate::pdf::engine::TextCharInfo;
use crate::t;
use crate::text_selection::{line_range, ordered, selection_text, word_range};
use crate::ui::page_rotation::PageMapping;
use eframe::egui;

impl MariApp {
    /// Texto atualmente selecionado na página visível, se houver.
    pub(crate) fn selected_text(&self) -> Option<String> {
        if self.text_chars_page != self.current_page {
            return None;
        }
        let (lo, hi) = ordered(self.sel_anchor?, self.sel_focus?);
        let text = selection_text(&self.text_chars, lo, hi);
        (!text.is_empty()).then_some(text)
    }

    pub(crate) fn clear_selection(&mut self) {
        self.sel_anchor = None;
        self.sel_focus = None;
    }

    /// Ctrl+C / menu Copiar.
    pub(crate) fn copy_selection(&mut self, ctx: &egui::Context) {
        let Some(text) = self.selected_text() else {
            return;
        };
        ctx.copy_text(text);
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast::info(t!("toast-text-copied"), 2.0, now));
    }

    /// Ctrl+A / menu Selecionar tudo. Se os chars da página ainda não
    /// chegaram, pede ao worker e conclui quando o resultado for aplicado.
    pub(crate) fn select_all_text(&mut self, ctx: &egui::Context) {
        if self.text_chars_page != self.current_page {
            self.pending_select_all = true;
            if let Some(worker) = self.document_worker.as_mut() {
                worker.request_chars(self.current_page);
            }
            return;
        }
        self.pending_select_all = false;
        let now = ctx.input(|i| i.time);
        if self.text_chars.is_empty() {
            self.toast = Some(Toast::error(t!("toast-no-text"), 2.5, now));
            return;
        }
        self.sel_anchor = Some(0);
        self.sel_focus = Some(self.text_chars.len() - 1);
        // Sem destaque visual no modo contínuo: avisa que a cópia está pronta.
        if self.viewer.mode == crate::ui::viewer::ViewMode::Continuous {
            self.toast = Some(Toast::info(t!("toast-text-selected"), 3.0, now));
        }
    }

    pub(crate) fn handle_text_selection(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        page_rect: egui::Rect,
        pdf_w: f32,
        pdf_h: f32,
    ) {
        let mapping = PageMapping::new(page_rect, (pdf_w, pdf_h), self.viewer.rotation);
        let nearest = |chars: &[TextCharInfo], pos: egui::Pos2| nearest_char(chars, pos, &mapping);

        let resp = ui.interact(
            page_rect,
            egui::Id::new("text_sel_drag"),
            egui::Sense::click_and_drag(),
        );
        let shift = ui.input(|i| i.modifiers.shift);
        let pointer = resp.interact_pointer_pos();

        if resp.drag_started_by(egui::PointerButton::Primary) {
            let index = nearest(&self.text_chars, pointer.unwrap_or(page_rect.center()));
            if !(shift && self.sel_anchor.is_some()) {
                self.sel_anchor = Some(index);
            }
            self.sel_focus = Some(index);
        }
        if resp.dragged_by(egui::PointerButton::Primary) {
            if let Some(pos) = pointer {
                self.sel_focus = Some(nearest(&self.text_chars, pos));
            }
        }
        if let Some(pos) = pointer.filter(|_| resp.clicked()) {
            let index = nearest(&self.text_chars, pos);
            let range = if resp.triple_clicked() {
                Some(line_range(&self.text_chars, index))
            } else if resp.double_clicked() {
                Some(word_range(&self.text_chars, index))
            } else {
                None
            };
            match range {
                Some((lo, hi)) => {
                    self.sel_anchor = Some(lo);
                    self.sel_focus = Some(hi);
                }
                None if shift && self.sel_anchor.is_some() => self.sel_focus = Some(index),
                None => self.clear_selection(),
            }
        }
        if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Text);
        }

        let has_selection = self.selected_text().is_some();
        let mut copy = false;
        let mut select_all = false;
        resp.context_menu(|ui| {
            if ui
                .add_enabled(has_selection, egui::Button::new(t!("sel-copy")))
                .clicked()
            {
                copy = true;
                ui.close_menu();
            }
            if ui.button(t!("sel-select-all")).clicked() {
                select_all = true;
                ui.close_menu();
            }
        });
        if copy {
            self.copy_selection(ctx);
        }
        if select_all {
            self.select_all_text(ctx);
        }

        self.paint_selection(ctx, ui, page_rect, pdf_w, pdf_h);
    }

    fn paint_selection(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        page_rect: egui::Rect,
        pdf_w: f32,
        pdf_h: f32,
    ) {
        let (Some(a), Some(f)) = (self.sel_anchor, self.sel_focus) else {
            return;
        };
        let chars = &self.text_chars;
        let (lo, hi) = ordered(a, f);
        let hi = hi.min(chars.len().saturating_sub(1));
        if chars.is_empty() || lo > hi {
            return;
        }
        let mapping = PageMapping::new(page_rect, (pdf_w, pdf_h), self.viewer.rotation);
        let char_rect =
            |ch: &TextCharInfo| mapping.rect_to_screen([ch.left, ch.bottom, ch.right, ch.top]);
        let mut line_rects: Vec<egui::Rect> = Vec::new();
        let mut cur_rect: Option<egui::Rect> = None;
        let mut cur_cy = f32::NAN;
        const LINE_MERGE_PX: f32 = 5.0;
        for ch in &chars[lo..=hi] {
            let r = char_rect(ch);
            if r.width() <= 0.0 || r.height() <= 0.0 {
                continue;
            }
            let cy = r.center().y;
            match cur_rect {
                Some(rect) if (cy - cur_cy).abs() <= LINE_MERGE_PX => {
                    cur_rect = Some(rect.union(r));
                }
                previous => {
                    line_rects.extend(previous);
                    cur_rect = Some(r);
                    cur_cy = cy;
                }
            }
        }
        line_rects.extend(cur_rect);

        let hl_color = match self.color_mode {
            ColorMode::Dark => egui::Color32::from_rgba_unmultiplied(196, 60, 92, 70),
            _ => egui::Color32::from_rgba_unmultiplied(122, 28, 48, 60),
        };
        let painter = ui.painter().clone();
        for r in &line_rects {
            painter.rect_filled(*r, egui::CornerRadius::ZERO, hl_color);
        }

        // Alças de seleção para ajustar início e fim.
        let (Some(first_r), Some(last_r)) = (line_rects.first(), line_rects.last()) else {
            return;
        };
        let start = egui::pos2(first_r.left(), first_r.top());
        let end = egui::pos2(last_r.right(), last_r.bottom());
        if let Some(index) =
            self.selection_handle(ctx, ui, &painter, start, -1.0, "sel_handle_start")
        {
            self.sel_anchor = Some(nearest_char(chars, index, &mapping));
        }
        if let Some(index) = self.selection_handle(ctx, ui, &painter, end, 1.0, "sel_handle_end") {
            self.sel_focus = Some(nearest_char(&self.text_chars, index, &mapping));
        }
    }

    /// Desenha uma alça (haste + círculo) e devolve a posição do ponteiro
    /// enquanto ela é arrastada.
    fn selection_handle(
        &self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        anchor: egui::Pos2,
        direction: f32,
        id: &str,
    ) -> Option<egui::Pos2> {
        let color = egui::Color32::from_rgba_unmultiplied(160, 36, 62, 235);
        let radius = 5.0_f32;
        let stem = 12.0_f32;
        let knob = egui::pos2(anchor.x, anchor.y + direction * (stem + radius));
        painter.line_segment(
            [anchor, egui::pos2(anchor.x, anchor.y + direction * stem)],
            egui::Stroke::new(2.0_f32, color),
        );
        painter.circle_filled(knob, radius, color);
        let hit = egui::Rect::from_center_size(knob, egui::vec2(radius * 3.5, radius * 3.5));
        let resp = ui.interact(hit, egui::Id::new(id), egui::Sense::drag());
        if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        resp.dragged()
            .then(|| resp.interact_pointer_pos())
            .flatten()
    }
}

fn nearest_char(chars: &[TextCharInfo], pos: egui::Pos2, mapping: &PageMapping) -> usize {
    let (px, py) = mapping.pdf_point(pos);
    chars
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| {
            let cx = (c.left + c.right) * 0.5;
            let cy = (c.bottom + c.top) * 0.5;
            (((cx - px).abs() + (cy - py).abs() * 3.0) * 1000.0) as i64
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}
