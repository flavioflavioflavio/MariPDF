// app_home.rs — Tela inicial (Home) do MariPDF
//
// Uma coluna única e centrada: logo, título, CTA e a lista de recentes
// compartilham a mesma largura e a mesma margem, então tudo alinha. As cores
// vêm da paleta ativa (o acento padrão é o violeta do ícone).

use crate::app::MariApp;
use crate::app_color_mode::ColorMode;
use crate::ui::icons;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Largura da coluna de conteúdo.
const COLUMN_MIN: f32 = 340.0;
const COLUMN_MAX: f32 = 560.0;
/// Altura exibida da logo horizontal (o PNG tem 3× para alta densidade).
const LOGO_HEIGHT: f32 = 44.0;
const CTA_SIZE: egui::Vec2 = egui::vec2(220.0, 42.0);
const ROW_HEIGHT: f32 = 58.0;
const ROW_PAD_X: f32 = 14.0;
const TILE_SIZE: f32 = 36.0;
const REMOVE_SIZE: f32 = 28.0;

/// Cores da Home resolvidas a partir do tema e da paleta.
#[derive(Clone, Copy)]
struct HomeColors {
    bg: egui::Color32,
    surface: egui::Color32,
    border: egui::Color32,
    text: egui::Color32,
    muted: egui::Color32,
    subtle: egui::Color32,
    accent: egui::Color32,
    /// Acento como cor de ícone/texto (coral no escuro).
    ink: egui::Color32,
    on_accent: egui::Color32,
    danger: egui::Color32,
    divider: f32,
}

impl HomeColors {
    fn new(cm: ColorMode, palette: u8) -> Self {
        let accent = cm.accent_for(palette);
        let bg = cm.bg_for(palette);
        // No escuro o card afunda (mais escuro que o fundo); nos claros ele
        // sobe (mais claro), com borda sólida derivada do texto.
        let (surface, border) = match cm {
            ColorMode::Dark => (cm.surface_for(palette), cm.border()),
            ColorMode::Light => (lighten(bg, 0.7), cm.text_muted().gamma_multiply(0.22)),
            ColorMode::Sepia => (lighten(bg, 0.45), cm.text_muted().gamma_multiply(0.22)),
        };
        Self {
            bg,
            surface,
            border,
            text: cm.text_primary(),
            muted: cm.text_muted(),
            subtle: cm.text_subtle(),
            accent,
            ink: cm.accent_ink_for(palette),
            on_accent: readable_on(accent),
            danger: cm.danger(),
            divider: cm.divider_width(),
        }
    }
}

/// Texto branco em acentos escuros, quase preto nos claros (azul, verde).
fn readable_on(fill: egui::Color32) -> egui::Color32 {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * lin(fill.r()) + 0.7152 * lin(fill.g()) + 0.0722 * lin(fill.b());
    if luminance > 0.30 {
        egui::Color32::from_rgb(18, 16, 24)
    } else {
        egui::Color32::WHITE
    }
}

fn lighten(color: egui::Color32, t: f32) -> egui::Color32 {
    let mix = |c: u8| (c as f32 + (255.0 - c as f32) * t).round() as u8;
    egui::Color32::from_rgb(mix(color.r()), mix(color.g()), mix(color.b()))
}

impl MariApp {
    pub(crate) fn ui_home(&mut self, ctx: &egui::Context) {
        let colors = HomeColors::new(self.color_mode, self.settings.palette.0);
        let recents = if self.settings.track_history {
            self.persistence.recent_files(8)
        } else {
            Vec::new()
        };

        // Rodapé fixo: atalhos e versão, centralizados.
        egui::TopBottomPanel::bottom("home_footer")
            .frame(
                egui::Frame::new()
                    .fill(colors.bg)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(crate::t!("home-shortcuts", version => self.version))
                            .size(10.5)
                            .color(colors.subtle),
                    );
                });
            });

        let mut open_path: Option<PathBuf> = None;
        let mut remove_path: Option<PathBuf> = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(colors.bg))
            .show(ctx, |ui| {
                let avail = ui.available_size();
                let column = (avail.x - 48.0).clamp(COLUMN_MIN.min(avail.x), COLUMN_MAX);
                let side = ((avail.x - column) / 2.0).max(0.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Sem recentes o hero fica no centro óptico; com
                        // recentes, sobe para dar lugar à lista.
                        let hero_height = 200.0;
                        let list_height = if recents.is_empty() {
                            0.0
                        } else {
                            44.0 + recents.len() as f32 * ROW_HEIGHT
                        };
                        let top = ((avail.y - hero_height - list_height) * 0.42).max(36.0);
                        ui.add_space(top);

                        ui.horizontal(|ui| {
                            ui.add_space(side);
                            ui.vertical(|ui| {
                                ui.set_width(column);
                                self.home_hero(ui, &colors, column);
                                if let Some(err) = self.render_error.clone() {
                                    ui.add_space(14.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("⚠ {err}"))
                                                .size(12.0)
                                                .color(colors.danger),
                                        );
                                    });
                                }
                                if !recents.is_empty() {
                                    ui.add_space(40.0);
                                    recents_header(ui, &colors);
                                    ui.add_space(10.0);
                                    let action = recents_list(ui, &colors, column, &recents);
                                    match action {
                                        Some(RowAction::Open(path)) => open_path = Some(path),
                                        Some(RowAction::Remove(path)) => remove_path = Some(path),
                                        None => {}
                                    }
                                }
                                ui.add_space(32.0);
                            });
                        });
                    });
            });

        if let Some(path) = open_path {
            self.pending_open = Some(path);
        }
        if let Some(path) = remove_path {
            self.persistence.remove_recent(&path);
        }

        // Botões flutuantes (tema + configurações) em app_home_btns.rs
        self.ui_home_buttons(ctx);
    }

    /// Logo, nome, tagline, CTA e dica de arrastar.
    fn home_hero(&mut self, ui: &mut egui::Ui, colors: &HomeColors, column: f32) {
        ui.vertical_centered(|ui| {
            ui.add(brand_logo(ui.ctx(), LOGO_HEIGHT));
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(crate::t!("app-tagline"))
                    .size(13.0)
                    .color(colors.muted),
            );
            ui.add_space(26.0);

            let cta_width = CTA_SIZE.x.min(column);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(cta_width, CTA_SIZE.y), egui::Sense::click());
            let hover = ui
                .ctx()
                .animate_bool_with_time(response.id, response.hovered(), 0.12);
            let fill = lighten(colors.accent, hover * 0.12);
            let painter = ui.painter();
            painter.rect_filled(rect, egui::CornerRadius::same(10), fill);
            let icon = painter.layout_no_wrap(
                icons::FOLDER_OPEN.to_string(),
                egui::FontId::new(17.0, egui::FontFamily::Name("Icons".into())),
                colors.on_accent,
            );
            let label = painter.layout_no_wrap(
                crate::t!("home-open-btn"),
                egui::FontId::proportional(14.5),
                colors.on_accent,
            );
            let gap = 8.0;
            let total = icon.size().x + gap + label.size().x;
            let x = rect.center().x - total / 2.0;
            painter.galley(
                egui::pos2(x, rect.center().y - icon.size().y / 2.0),
                icon,
                colors.on_accent,
            );
            painter.galley(
                egui::pos2(
                    x + total - label.size().x,
                    rect.center().y - label.size().y / 2.0,
                ),
                label,
                colors.on_accent,
            );
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                self.open_file_dialog();
            }

            ui.add_space(12.0);
            let (hint, color) = if self.drop_zone_hovered {
                (crate::t!("home-drop-release"), colors.ink)
            } else {
                (crate::t!("home-drop-hint"), colors.muted)
            };
            ui.label(egui::RichText::new(hint).size(11.5).color(color));
        });
    }
}

/// Logo horizontal da marca na altura pedida, mantendo a proporção.
pub(crate) fn brand_logo(ctx: &egui::Context, height: f32) -> egui::Image<'static> {
    let texture = logo_texture(ctx);
    let [w, h] = texture.size();
    let width = height * w as f32 / h.max(1) as f32;
    egui::Image::new(&texture).fit_to_exact_size(egui::vec2(width, height))
}

/// Logo embutida (fundo transparente), carregada uma vez por contexto.
fn logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("brand_logo_texture");
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let image = image::load_from_memory(include_bytes!("../assets/logo-horizontal.png"))
        .map(|img| img.to_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    let size = [image.width() as usize, image.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    let texture = ctx.load_texture("brand_logo", color_image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}

fn recents_header(ui: &mut egui::Ui, colors: &HomeColors) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(icons::ph(icons::CLOCK, 13.0, colors.muted));
        ui.add_space(2.0);
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &crate::t!("home-recents"),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(10.5),
                color: colors.muted,
                extra_letter_spacing: 1.2,
                ..Default::default()
            },
        );
        ui.label(job);
    });
}

enum RowAction {
    Open(PathBuf),
    Remove(PathBuf),
}

/// Lista de recentes num único bloco com divisórias, linhas de altura fixa.
fn recents_list(
    ui: &mut egui::Ui,
    colors: &HomeColors,
    width: f32,
    recents: &[(PathBuf, crate::pdf::persistence::FileSession)],
) -> Option<RowAction> {
    let height = recents.len() as f32 * ROW_HEIGHT;
    let (block, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(block.expand(1.0));
    painter.rect_filled(block, egui::CornerRadius::same(12), colors.surface);
    painter.rect_stroke(
        block,
        egui::CornerRadius::same(12),
        egui::Stroke::new(colors.divider, colors.border),
        egui::StrokeKind::Inside,
    );

    let mut action = None;
    for (index, (path, session)) in recents.iter().enumerate() {
        let row = egui::Rect::from_min_size(
            block.min + egui::vec2(0.0, index as f32 * ROW_HEIGHT),
            egui::vec2(width, ROW_HEIGHT),
        );
        if index > 0 {
            let y = row.top();
            painter.hline(
                (row.left() + ROW_PAD_X + TILE_SIZE + 12.0)..=(row.right() - ROW_PAD_X),
                y,
                egui::Stroke::new(colors.divider, colors.border),
            );
        }
        if let Some(row_action) = recent_row(ui, &painter, colors, row, path, session) {
            action = Some(row_action);
        }
    }
    action
}

fn recent_row(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    colors: &HomeColors,
    row: egui::Rect,
    path: &Path,
    session: &crate::pdf::persistence::FileSession,
) -> Option<RowAction> {
    let id = ui.id().with(("recent", path));
    let response = ui.interact(row, id, egui::Sense::click());
    let remove_rect = egui::Rect::from_center_size(
        egui::pos2(row.right() - ROW_PAD_X - REMOVE_SIZE / 2.0, row.center().y),
        egui::vec2(REMOVE_SIZE, REMOVE_SIZE),
    );
    // Registrado depois da linha: o X recebe o clique por cima dela.
    let remove = ui
        .interact(remove_rect, id.with("remove"), egui::Sense::click())
        .on_hover_text(crate::t!("home-remove-hint"));
    let row_hovered = response.hovered() || remove.hovered();
    let hover = ui.ctx().animate_bool_with_time(id, row_hovered, 0.10);

    if hover > 0.0 {
        painter.rect_filled(
            row.shrink(4.0),
            egui::CornerRadius::same(9),
            colors.accent.gamma_multiply(0.09 * hover),
        );
    }

    // Ícone do arquivo
    let tile = egui::Rect::from_center_size(
        egui::pos2(row.left() + ROW_PAD_X + TILE_SIZE / 2.0, row.center().y),
        egui::vec2(TILE_SIZE, TILE_SIZE),
    );
    painter.rect_filled(
        tile,
        egui::CornerRadius::same(9),
        colors.accent.gamma_multiply(0.16),
    );
    painter.text(
        tile.center(),
        egui::Align2::CENTER_CENTER,
        icons::FILE_PDF,
        egui::FontId::new(19.0, egui::FontFamily::Name("Icons".into())),
        colors.ink,
    );

    // Página onde parou (pílula) e espaço reservado para o X
    let page = painter.layout_no_wrap(
        crate::t!("home-page-pill", page => session.last_page + 1),
        egui::FontId::proportional(11.0),
        colors.muted,
    );
    let pill_right = row.right() - ROW_PAD_X - hover * (REMOVE_SIZE + 6.0);
    let pill = egui::Rect::from_min_size(
        egui::pos2(pill_right - page.size().x - 16.0, row.center().y - 11.0),
        egui::vec2(page.size().x + 16.0, 22.0),
    );
    painter.rect_filled(pill, egui::CornerRadius::same(11), colors.bg);
    painter.galley(pill.center() - page.size() / 2.0, page, colors.muted);

    // Nome e pasta, elididos para caber até a pílula
    let text_left = tile.right() + 12.0;
    let pill_rest_left = row.right() - ROW_PAD_X - pill.width();
    let text_width = (pill_rest_left - 12.0 - text_left).max(40.0);
    let name = file_stem(path);
    let folder = folder_label(path);
    let name_galley = ui.fonts(|f| f.layout_job(single_line(&name, 13.5, colors.text, text_width)));
    let folder = elide_start(ui, &folder, 11.0, text_width);
    let folder_galley =
        ui.fonts(|f| f.layout_job(single_line(&folder, 11.0, colors.muted, text_width)));
    let block_height = name_galley.size().y + 2.0 + folder_galley.size().y;
    let top = row.center().y - block_height / 2.0;
    painter.galley(egui::pos2(text_left, top), name_galley.clone(), colors.text);
    painter.galley(
        egui::pos2(text_left, top + name_galley.size().y + 2.0),
        folder_galley,
        colors.muted,
    );

    // X só aparece com a linha em foco
    if hover > 0.0 {
        let danger_hover = remove.hovered();
        if danger_hover {
            painter.rect_filled(
                remove_rect,
                egui::CornerRadius::same(7),
                colors.danger.gamma_multiply(0.16),
            );
        }
        let color = if danger_hover {
            colors.danger
        } else {
            colors.muted
        };
        painter.text(
            remove_rect.center(),
            egui::Align2::CENTER_CENTER,
            icons::X,
            egui::FontId::new(15.0, egui::FontFamily::Name("Icons".into())),
            color.gamma_multiply(hover),
        );
    }

    if row_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if remove.clicked() {
        Some(RowAction::Remove(path.to_path_buf()))
    } else if response.clicked() {
        Some(RowAction::Open(path.to_path_buf()))
    } else {
        None
    }
}

/// Uma linha, cortada com reticências se passar da largura.
fn single_line(
    text: &str,
    size: f32,
    color: egui::Color32,
    max_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: egui::FontId::proportional(size),
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// Corta o início (`…\pasta\final`): o fim do caminho é o que identifica.
fn elide_start(ui: &egui::Ui, text: &str, size: f32, max_width: f32) -> String {
    let width = |s: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(
                s.to_owned(),
                egui::FontId::proportional(size),
                egui::Color32::WHITE,
            )
            .size()
            .x
        })
    };
    if width(text) <= max_width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    // Busca binária pelo maior sufixo que cabe com a reticência.
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        let candidate: String = std::iter::once('…')
            .chain(chars[mid..].iter().copied())
            .collect();
        if width(&candidate) <= max_width {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    std::iter::once('…')
        .chain(chars[lo..].iter().copied())
        .collect()
}

fn file_stem(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("PDF");
    match name.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("pdf") => stem.to_owned(),
        _ => name.to_owned(),
    }
}

/// Pasta do arquivo, com o perfil do usuário abreviado para `~`.
fn folder_label(path: &Path) -> String {
    let dir = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    match std::env::var("USERPROFILE") {
        Ok(home) if !home.is_empty() && dir.to_lowercase().starts_with(&home.to_lowercase()) => {
            format!("~{}", &dir[home.len()..])
        }
        _ => dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_on_accent_stays_readable() {
        // Violeta da marca: texto branco; azul/verde claros: texto escuro.
        assert_eq!(
            readable_on(egui::Color32::from_rgb(124, 85, 236)),
            egui::Color32::WHITE
        );
        assert_ne!(
            readable_on(egui::Color32::from_rgb(74, 222, 128)),
            egui::Color32::WHITE
        );
    }

    #[test]
    fn file_stem_drops_only_the_pdf_extension() {
        assert_eq!(file_stem(Path::new(r"C:\docs\Relatório.PDF")), "Relatório");
        assert_eq!(
            file_stem(Path::new(r"C:\docs\v1.2 notas.pdf")),
            "v1.2 notas"
        );
        assert_eq!(
            file_stem(Path::new(r"C:\docs\sem-extensao")),
            "sem-extensao"
        );
    }
}
