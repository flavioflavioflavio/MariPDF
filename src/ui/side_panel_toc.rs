// ui/side_panel_toc.rs — Painel de Índice (TOC) do MariPDF
// Extraído de side_panel.rs para respeitar o hardcap de 250L.

use eframe::egui;
use crate::pdf::engine::BookmarkEntry;

/// Indentação horizontal por nível de profundidade no TOC
const DEPTH_INDENT: f32 = 14.0;

/// Altura estimada de cada item no TOC automático (em px)
const AUTO_TOC_ITEM_H: f32 = 28.0;

/// Renderiza o painel de Índice.
///
/// Se `bookmarks` estiver vazio E `page_count > 0`, gera um índice automático
/// com entradas "Página N" para cada página do documento.
///
/// Retorna `Some(page)` se o usuário clicou num item (navegação).
pub fn show_toc(
    ui:           &mut egui::Ui,
    bookmarks:    &[BookmarkEntry],
    current_page: usize,
    page_count:   usize,
    color_bg:     egui::Color32,
    color_text:   egui::Color32,
    color_muted:  egui::Color32,
    accent:       egui::Color32,
) -> Option<usize> {
    let mut goto = None;

    if bookmarks.is_empty() {
        if page_count == 0 { return None; }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⚙ Gerado automaticamente").size(9.5).color(color_muted));
        });
        ui.add_space(3.0);

        let total_h = AUTO_TOC_ITEM_H * page_count as f32;

        egui::ScrollArea::vertical()
            .id_salt("toc_auto_scroll")
            .auto_shrink([false; 2])
            .show_viewport(ui, |ui, viewport| {
                ui.set_min_height(total_h);

                let first = ((viewport.min.y / AUTO_TOC_ITEM_H) as usize).saturating_sub(1);
                let last  = (((viewport.max.y / AUTO_TOC_ITEM_H) as usize) + 1)
                                .min(page_count.saturating_sub(1));

                for p in first..=last {
                    let y_top = p as f32 * AUTO_TOC_ITEM_H - viewport.min.y;
                    let is_current = p == current_page;
                    let bg = if is_current {
                        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 28)
                    } else { egui::Color32::TRANSPARENT };
                    let label_color = if is_current { accent } else { color_text };

                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(0.0, y_top),
                            egui::vec2(ui.available_width(), AUTO_TOC_ITEM_H),
                        )),
                        |ui| {
                            egui::Frame::new()
                                .fill(bg)
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(8, 5))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    let label = egui::Label::new(
                                        egui::RichText::new(format!("Página {}", p + 1))
                                            .size(12.0).color(label_color),
                                    ).sense(egui::Sense::click());
                                    let resp = ui.add(label);
                                    if resp.clicked() { goto = Some(p); }
                                    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
                                });
                        },
                    );
                }
            });

        return goto;
    }

    // ── Índice interno do PDF ──────────────────────────────────────────────────
    egui::ScrollArea::vertical()
        .id_salt("toc_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;

            for entry in bookmarks {
                let indent = entry.depth as f32 * DEPTH_INDENT;
                let is_current = entry.page.map_or(false, |p| p == current_page);
                let bg = if is_current {
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 28)
                } else { color_bg };
                let text_color = if is_current { accent } else { color_text };
                let font_size  = if entry.depth == 0 { 12.5 } else { 11.5 };

                egui::Frame::new()
                    .fill(bg)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(4, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(indent);
                            if entry.depth > 0 {
                                ui.label(egui::RichText::new("›").size(font_size).color(color_muted));
                            }
                            let label = egui::Label::new(
                                egui::RichText::new(&entry.title).size(font_size).color(text_color),
                            ).truncate().sense(egui::Sense::click());
                            let resp = ui.add(label);
                            if let Some(page) = entry.page {
                                if resp.clicked() { goto = Some(page); }
                                if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
                                resp.on_hover_text(format!("Página {}", page + 1));
                            }
                        });
                    });
            }
        });

    goto
}
