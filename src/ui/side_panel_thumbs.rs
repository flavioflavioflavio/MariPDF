// ui/side_panel_thumbs.rs — Painel de Thumbnails do MariPDF
// Extraído de side_panel.rs (era 432L → agora 2 arquivos ≤250L)

use eframe::egui;
use crate::pdf::thumb_cache::ThumbCache;

/// Renderiza o painel de thumbnails com virtual scrolling.
/// Cada thumb ocupa `avail_w` de largura; altura proporcional ao bitmap real.
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
    let label_area_h = 26.0_f32;
    let gap          = 12.0_f32;

    let fallback_img_h = (avail_w * 297.0 / 210.0).min(320.0);

    let img_heights: Vec<f32> = (0..page_count)
        .map(|p| {
            if let Some((bmp_w, bmp_h)) = cache.get_cached_dimensions(p) {
                (avail_w * bmp_h as f32 / (bmp_w as f32).max(1.0)).clamp(30.0, 500.0)
            } else {
                fallback_img_h
            }
        })
        .collect();

    let item_heights: Vec<f32> = img_heights.iter().map(|&h| h + label_area_h + gap).collect();
    let total_h: f32 = item_heights.iter().sum();

    let mut y_offsets = Vec::with_capacity(page_count);
    let mut acc = 0.0_f32;
    for &h in &item_heights { y_offsets.push(acc); acc += h; }

    egui::ScrollArea::vertical()
        .id_salt("thumbs_scroll")
        .auto_shrink([false; 2])
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_height(total_h);

            let first = y_offsets.iter().rposition(|&y| y <= viewport.min.y).unwrap_or(0);
            let last  = y_offsets
                .iter().position(|&y| y > viewport.max.y)
                .map(|i| i.saturating_sub(1))
                .unwrap_or(page_count.saturating_sub(1));

            let buf_first = first.saturating_sub(2);
            let buf_last  = (last + 2).min(page_count.saturating_sub(1));
            for p in buf_first..=buf_last { cache.request(p); }

            for p in first..=last {
                let img_h = img_heights[p];
                let y_top = y_offsets[p] - viewport.min.y;
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(margin, y_top),
                    egui::vec2(avail_w, img_h + label_area_h + gap),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(item_rect), |ui| {
                    render_thumb_item(
                        ui, cache, p, current_page,
                        avail_w, img_h,
                        color_panel_bg, color_muted, color_text, accent,
                        invert_colors, page_tint, &mut goto,
                    );
                });
            }
        });

    goto
}

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

    let img_rect = egui::Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(thumb_w, thumb_h),
    );
    let resp = ui.allocate_rect(img_rect, egui::Sense::click());

    if let Some(rendered) = cache.get(page) {
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

        ui.painter().rect_filled(img_rect, egui::CornerRadius::same(0), color_panel_bg);
        ui.painter().rect_filled(
            draw_rect.translate(egui::vec2(2.0, 2.0)),
            egui::CornerRadius::same(0),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, 18),
        );

        let tex_name = if invert_colors { format!("thumb_{page}_inv") } else { format!("thumb_{page}") };
        let color_img = if invert_colors {
            let mut rgba = rendered.rgba.clone();
            crate::ui::viewer::invert_rgba(&mut rgba);
            egui::ColorImage::from_rgba_unmultiplied([rendered.width as usize, rendered.height as usize], &rgba)
        } else {
            egui::ColorImage::from_rgba_unmultiplied([rendered.width as usize, rendered.height as usize], &rendered.rgba)
        };

        let tex = ui.ctx().load_texture(tex_name, color_img, egui::TextureOptions::LINEAR);
        ui.painter().image(tex.id(), draw_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), page_tint);
        ui.painter().rect_stroke(draw_rect, egui::CornerRadius::same(0),
            egui::Stroke::new(border_w, border_color), egui::StrokeKind::Outside);
    } else {
        let placeholder_color = egui::Color32::from_rgba_unmultiplied(
            (color_panel_bg.r() as i16 + 18).min(255) as u8,
            (color_panel_bg.g() as i16 + 18).min(255) as u8,
            (color_panel_bg.b() as i16 + 15).min(255) as u8, 255,
        );
        ui.painter().rect_filled(img_rect, egui::CornerRadius::same(0), placeholder_color);
        ui.painter().rect_stroke(img_rect, egui::CornerRadius::same(0),
            egui::Stroke::new(border_w, border_color), egui::StrokeKind::Outside);
        ui.painter().text(img_rect.center(), egui::Align2::CENTER_CENTER, "…",
            egui::FontId::proportional(14.0), color_muted);
    }

    let label_y  = img_rect.max.y + 8.0;
    let label_cx = img_rect.center().x;
    let label_color = if is_current { accent } else { color_muted };
    ui.painter().text(egui::pos2(label_cx, label_y), egui::Align2::CENTER_CENTER,
        format!("{}", page + 1), egui::FontId::proportional(11.0), label_color);

    let div_color = egui::Color32::from_rgba_unmultiplied(color_text.r(), color_text.g(), color_text.b(), 15);
    let div_y = label_y + 11.0;
    ui.painter().line_segment(
        [egui::pos2(img_rect.left() + 16.0, div_y), egui::pos2(img_rect.right() - 16.0, div_y)],
        egui::Stroke::new(1.0, div_color),
    );

    if resp.clicked() { *goto = Some(page); }
    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
}
