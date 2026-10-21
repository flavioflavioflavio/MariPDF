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
        // Linhas agrupadas no espaço do PDF (independe da rotação) e só
        // depois levadas para a tela.
        let pdf_lines = selection_lines(&chars[lo..=hi]);
        let line_rects: Vec<egui::Rect> = pdf_lines
            .iter()
            .map(|line| mapping.rect_to_screen(*line))
            .collect();

        let hl_color = match self.color_mode {
            ColorMode::Dark => egui::Color32::from_rgba_unmultiplied(196, 60, 92, 70),
            _ => egui::Color32::from_rgba_unmultiplied(122, 28, 48, 60),
        };
        let painter = ui.painter().clone();
        for r in &line_rects {
            painter.rect_filled(*r, egui::CornerRadius::ZERO, hl_color);
        }

        // Alças no início (canto superior esquerdo da 1ª linha, haste para
        // cima) e no fim (canto inferior direito da última, haste para baixo),
        // tudo em coordenadas do PDF e levado pela rotação.
        let (Some(first), Some(last)) = (pdf_lines.first(), pdf_lines.last()) else {
            return;
        };
        let start = mapping.screen_point(first[0], first[3]);
        let up = mapping.screen_point(first[0], first[3] + 1.0) - start;
        let end = mapping.screen_point(last[2], last[1]);
        let down = mapping.screen_point(last[2], last[1] - 1.0) - end;
        if let Some(index) = self.selection_handle(
            ctx,
            ui,
            &painter,
            start,
            up.normalized(),
            "sel_handle_start",
        ) {
            self.sel_anchor = Some(nearest_char(chars, index, &mapping));
        }
        if let Some(index) =
            self.selection_handle(ctx, ui, &painter, end, down.normalized(), "sel_handle_end")
        {
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
        direction: egui::Vec2,
        id: &str,
    ) -> Option<egui::Pos2> {
        let color = egui::Color32::from_rgba_unmultiplied(160, 36, 62, 235);
        let radius = 5.0_f32;
        let stem = 12.0_f32;
        let knob = anchor + direction * (stem + radius);
        painter.line_segment(
            [anchor, anchor + direction * stem],
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

/// Agrupa caracteres em linhas pelo centro vertical no PDF e devolve o
/// retângulo `[left, bottom, right, top]` de cada linha.
fn selection_lines(chars: &[TextCharInfo]) -> Vec<[f32; 4]> {
    const LINE_MERGE_PTS: f32 = 3.0;
    let mut lines: Vec<[f32; 4]> = Vec::new();
    let mut current: Option<([f32; 4], f32)> = None;
    for ch in chars {
        if ch.right <= ch.left || ch.top <= ch.bottom {
            continue;
        }
        let rect = [ch.left, ch.bottom, ch.right, ch.top];
        let center = (ch.bottom + ch.top) * 0.5;
        current = match current {
            Some((line, line_center)) if (center - line_center).abs() <= LINE_MERGE_PTS => Some((
                [
                    line[0].min(rect[0]),
                    line[1].min(rect[1]),
                    line[2].max(rect[2]),
                    line[3].max(rect[3]),
                ],
                line_center,
            )),
            previous => {
                lines.extend(previous.map(|(line, _)| line));
                Some((rect, center))
            }
        };
    }
    lines.extend(current.map(|(line, _)| line));
    lines
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::page_rotation::Rotation;

    fn ch(left: f32, bottom: f32) -> TextCharInfo {
        TextCharInfo {
            ch: 'a',
            left,
            bottom,
            right: left + 6.0,
            top: bottom + 10.0,
        }
    }

    #[test]
    fn lines_are_grouped_in_pdf_space() {
        let chars = [ch(10.0, 700.0), ch(16.0, 700.5), ch(10.0, 680.0)];
        let lines = selection_lines(&chars);
        assert_eq!(
            lines,
            vec![[10.0, 700.0, 22.0, 710.5], [10.0, 680.0, 16.0, 690.0]]
        );
    }

    #[test]
    fn start_handle_points_away_from_the_text_in_every_rotation() {
        let page = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 600.0));
        for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
            let m = PageMapping::new(page, (600.0, 600.0), rotation);
            let start = m.screen_point(10.0, 710.0 - 110.0);
            let up = (m.screen_point(10.0, 601.0) - start).normalized();
            // A haste aponta para fora da linha: o centro do glifo fica atrás.
            let glyph_center = m.screen_point(13.0, 595.0);
            assert!((glyph_center - start).dot(up) < 0.0, "{rotation:?}");
        }
    }
}
