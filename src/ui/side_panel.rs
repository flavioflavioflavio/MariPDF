// ui/side_panel.rs — Side panels: TOC (Índice) e Thumbnails
//
// Design:
//   • TocPanel     → bookmarks internos; se ausentes, gera índice automático por página
//   • ThumbsPanel  → virtual scrolling, proporção REAL de cada página, tema-aware
//   • Ambos retornam Option<usize> para navegação em app.rs

use eframe::egui;
use crate::pdf::engine::BookmarkEntry;
use crate::pdf::thumb_cache::ThumbCache;

// ── Constantes de layout ──────────────────────────────────────────────────────

/// Largura padrão do painel lateral
pub const SIDE_PANEL_WIDTH: f32 = 220.0;

/// Indentação horizontal por nível de profundidade no TOC
const DEPTH_INDENT: f32 = 14.0;

/// Altura estimada de cada item no TOC automático (em px)
const AUTO_TOC_ITEM_H: f32 = 28.0;

// ── TocPanel ─────────────────────────────────────────────────────────────────

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
        // ── Índice automático por número de página ────────────────────────
        if page_count == 0 {
            return None;
        }

        // Badge sutil indicando geração automática
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⚙ Gerado automaticamente")
                    .size(9.5)
                    .color(color_muted),
            );
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
                        egui::Color32::from_rgba_unmultiplied(
                            accent.r(), accent.g(), accent.b(), 28,
                        )
                    } else {
                        egui::Color32::TRANSPARENT
                    };

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
                                            .size(12.0)
                                            .color(label_color),
                                    )
                                    .sense(egui::Sense::click());

                                    let resp = ui.add(label);
                                    if resp.clicked() { goto = Some(p); }
                                    if resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                });
                        },
                    );
                }
            });

        return goto;
    }

    // ── Índice interno do PDF ─────────────────────────────────────────────
    egui::ScrollArea::vertical()
        .id_salt("toc_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;

            for entry in bookmarks {
                let indent = entry.depth as f32 * DEPTH_INDENT;
                let is_current = entry.page.map_or(false, |p| p == current_page);

                let bg = if is_current {
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(), accent.g(), accent.b(), 28,
                    )
                } else {
                    color_bg
                };

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
                                ui.label(
                                    egui::RichText::new("›")
                                        .size(font_size)
                                        .color(color_muted),
                                );
                            }

                            let label = egui::Label::new(
                                egui::RichText::new(&entry.title)
                                    .size(font_size)
                                    .color(text_color),
                            )
                            .truncate()
                            .sense(egui::Sense::click());

                            let resp = ui.add(label);

                            if let Some(page) = entry.page {
                                if resp.clicked() { goto = Some(page); }
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                resp.on_hover_text(format!("Página {}", page + 1));
                            }
                        });
                    });
            }
        });

    goto
}

// ── ThumbsPanel ───────────────────────────────────────────────────────────────

/// Renderiza o painel de Thumbnails com virtual scrolling.
///
/// Cada thumb ocupa `avail_w` de largura, com altura proporcional ao bitmap
/// real renderizado (aspect ratio da página, calculado a partir da dimensão
/// do bitmap retornado pelo ThumbCache).
///
/// Retorna `Some(page)` se o usuário clicou num thumbnail.
pub fn show_thumbs(
    ui:              &mut egui::Ui,
    cache:           &mut ThumbCache,
    current_page:    usize,
    color_panel_bg:  egui::Color32,
    color_muted:     egui::Color32,
    color_text:      egui::Color32,
    accent:          egui::Color32,
    invert_colors:   bool,
    page_tint:       egui::Color32,
) -> Option<usize> {
    let mut goto = None;

    let page_count = cache.page_count;
    if page_count == 0 { return None; }

    let margin       = 12.0_f32;
    let avail_w      = (ui.available_width() - margin * 2.0).max(50.0);
    let label_area_h = 26.0_f32; // número da página + divisória
    let gap          = 12.0_f32; // espaço entre items

    // Altura de imagem para uma página sem bitmap em cache (A4 portrait)
    let fallback_img_h = (avail_w * 297.0 / 210.0).min(320.0);

    // ── Calcula altura de cada item usando dimensões REAIS do bitmap ─────────
    // Leitura imutável — sem borrow mutável do cache ainda.
    let img_heights: Vec<f32> = (0..page_count)
        .map(|p| {
            if let Some((bmp_w, bmp_h)) = cache.get_cached_dimensions(p) {
                // Aspect ratio real da página renderizada
                (avail_w * bmp_h as f32 / (bmp_w as f32).max(1.0)).clamp(30.0, 500.0)
            } else {
                fallback_img_h
            }
        })
        .collect();

    let item_heights: Vec<f32> = img_heights.iter().map(|&h| h + label_area_h + gap).collect();
    let total_h: f32 = item_heights.iter().sum();

    // Posições Y acumuladas corretas
    let mut y_offsets = Vec::with_capacity(page_count);
    let mut acc = 0.0_f32;
    for &h in &item_heights {
        y_offsets.push(acc);
        acc += h;
    }

    egui::ScrollArea::vertical()
        .id_salt("thumbs_scroll")
        .auto_shrink([false; 2])
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_height(total_h);

            // Determina o range de páginas visíveis via busca linear rápida
            let first = y_offsets.iter().rposition(|&y| y <= viewport.min.y).unwrap_or(0);
            let last  = y_offsets
                .iter()
                .position(|&y| y > viewport.max.y)
                .map(|i| i.saturating_sub(1))
                .unwrap_or(page_count.saturating_sub(1));

            // Pede render das páginas visíveis + buffer
            let buf_first = first.saturating_sub(2);
            let buf_last  = (last + 2).min(page_count.saturating_sub(1));
            for p in buf_first..=buf_last {
                cache.request(p);
            }

            // Renderiza
            for p in first..=last {
                let img_h = img_heights[p];
                let y_top = y_offsets[p] - viewport.min.y;

                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(margin, y_top),
                    egui::vec2(avail_w, img_h + label_area_h + gap),
                );

                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(item_rect),
                    |ui| {
                    render_thumb_item(
                        ui, cache, p, current_page,
                        avail_w, img_h,
                        color_panel_bg, color_muted, color_text, accent,
                        invert_colors, page_tint,
                        &mut goto,
                    );
                });
            }
        });

    goto
}

/// Renderiza um único thumbnail.
/// `thumb_w` × `thumb_h` é o tamanho desejado da imagem em coordenadas de UI,
/// já calculado com o aspect ratio REAL da página em `show_thumbs`.
fn render_thumb_item(
    ui:             &mut egui::Ui,
    cache:          &mut ThumbCache,
    page:           usize,
    current_page:   usize,
    thumb_w:        f32,
    thumb_h:        f32,
    color_panel_bg: egui::Color32,
    color_muted:    egui::Color32,
    color_text:     egui::Color32,
    accent:         egui::Color32,
    invert_colors:  bool,
    page_tint:      egui::Color32,
    goto:           &mut Option<usize>,
) {
    let is_current   = page == current_page;
    let border_color = if is_current {
        accent
    } else {
        egui::Color32::from_rgba_unmultiplied(color_text.r(), color_text.g(), color_text.b(), 40)
    };
    let border_w = if is_current { 2.5_f32 } else { 1.0_f32 };

    // ── Rect da imagem — exatamente avail_w × img_h (aspect ratio já correto) ─
    let img_rect = egui::Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(thumb_w, thumb_h),
    );
    let resp = ui.allocate_rect(img_rect, egui::Sense::click());

    if let Some(rendered) = cache.get(page) {
        // Calcula o rect real da imagem baseando-se no bitmap —
        // garante que mesmo que thumb_h tenha divergido (clamp), não distorce.
        let bmp_w = rendered.width  as f32;
        let bmp_h = rendered.height as f32;
        let scale = (thumb_w / bmp_w).min(thumb_h / bmp_h);
        let real_w = bmp_w * scale;
        let real_h = bmp_h * scale;
        let ox = (thumb_w - real_w) * 0.5;
        let oy = (thumb_h - real_h) * 0.5;
        let draw_rect = egui::Rect::from_min_size(
            img_rect.min + egui::vec2(ox, oy),
            egui::vec2(real_w, real_h),
        );

        // Fundo do slot (visível nas barras laterais se a página não for 100% wide)
        ui.painter().rect_filled(img_rect, egui::CornerRadius::same(0), color_panel_bg);

        // Sombra sutil
        ui.painter().rect_filled(
            draw_rect.translate(egui::vec2(2.0, 2.0)),
            egui::CornerRadius::same(0),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, 18),
        );

        // Prepara textura (com inversão se dark mode)
        let tex_name = if invert_colors {
            format!("thumb_{page}_inv")
        } else {
            format!("thumb_{page}")
        };

        let color_img = if invert_colors {
            let mut rgba = rendered.rgba.clone();
            crate::ui::viewer::invert_rgba(&mut rgba);
            egui::ColorImage::from_rgba_unmultiplied(
                [rendered.width as usize, rendered.height as usize],
                &rgba,
            )
        } else {
            egui::ColorImage::from_rgba_unmultiplied(
                [rendered.width as usize, rendered.height as usize],
                &rendered.rgba,
            )
        };

        let tex = ui.ctx().load_texture(tex_name, color_img, egui::TextureOptions::LINEAR);

        // Imagem com tamanho real (sem stretch)
        ui.painter().image(
            tex.id(),
            draw_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            page_tint,
        );

        // Borda
        ui.painter().rect_stroke(
            draw_rect,
            egui::CornerRadius::same(0),
            egui::Stroke::new(border_w, border_color),
            egui::StrokeKind::Outside,
        );
    } else {
        // Placeholder tema-aware
        let placeholder_color = egui::Color32::from_rgba_unmultiplied(
            (color_panel_bg.r() as i16 + 18).min(255) as u8,
            (color_panel_bg.g() as i16 + 18).min(255) as u8,
            (color_panel_bg.b() as i16 + 15).min(255) as u8,
            255,
        );
        ui.painter().rect_filled(img_rect, egui::CornerRadius::same(0), placeholder_color);
        ui.painter().rect_stroke(
            img_rect,
            egui::CornerRadius::same(0),
            egui::Stroke::new(border_w, border_color),
            egui::StrokeKind::Outside,
        );
        ui.painter().text(
            img_rect.center(),
            egui::Align2::CENTER_CENTER,
            "…",
            egui::FontId::proportional(14.0),
            color_muted,
        );
    }

    // ── Número da página ───────────────────────────────────────────────────
    let label_y     = img_rect.max.y + 8.0;
    let label_cx    = img_rect.center().x;
    let label_color = if is_current { accent } else { color_muted };

    ui.painter().text(
        egui::pos2(label_cx, label_y),
        egui::Align2::CENTER_CENTER,
        format!("{}", page + 1),
        egui::FontId::proportional(11.0),
        label_color,
    );

    // ── Divisória ──────────────────────────────────────────────────────────
    let div_color = egui::Color32::from_rgba_unmultiplied(
        color_text.r(), color_text.g(), color_text.b(), 15,
    );
    let div_y = label_y + 11.0;
    ui.painter().line_segment(
        [egui::pos2(img_rect.left() + 16.0, div_y), egui::pos2(img_rect.right() - 16.0, div_y)],
        egui::Stroke::new(1.0, div_color),
    );

    if resp.clicked() { *goto = Some(page); }
    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
}

