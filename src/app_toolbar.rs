// app_toolbar.rs — Barra de ferramentas do leitor PDF
// Três zonas: documento (esquerda), navegação de página (centro) e
// visualização (direita). Um único estilo de botão, ícones Phosphor e
// tooltips via Fluent. A zona direita é desenhada primeiro para que o centro
// e o nome do arquivo se ajustem ao espaço que sobra em janelas estreitas.

use crate::app::MariApp;
use crate::app_color_mode::ColorMode;
use crate::app_reader::SidePanelKind;
use crate::pdf::thumb_cache::ThumbCache;
use crate::t;
use crate::ui::icons;
use crate::ui::viewer::ViewMode;
use eframe::egui;

const ROW_HEIGHT: f32 = 28.0;
const BTN_SIZE: egui::Vec2 = egui::vec2(32.0, ROW_HEIGHT);
const ICON_SIZE: f32 = 16.0;
/// Largura do grupo central `‹ n / N ›`.
const PAGE_GROUP_WIDTH: f32 = 148.0;
const ZONE_GAP: f32 = 12.0;
/// Menor largura útil para ainda mostrar o nome do arquivo.
const MIN_NAME_WIDTH: f32 = 64.0;

/// Cores da toolbar derivadas do tema atual.
#[derive(Clone, Copy)]
struct Tok {
    icon: egui::Color32,
    text: egui::Color32,
    subtle: egui::Color32,
    accent: egui::Color32,
    hover: egui::Color32,
    group_fill: egui::Color32,
    border: egui::Color32,
}

impl Tok {
    fn new(cm: ColorMode, palette: u8) -> Self {
        Self {
            icon: cm.text_muted(),
            text: cm.text_primary(),
            subtle: cm.text_subtle(),
            accent: cm.accent_for(palette),
            hover: cm.surface_hover(),
            group_fill: cm.statusbar_for(palette),
            border: cm.border(),
        }
    }
}

/// Ações coletadas durante o desenho e aplicadas depois, fora dos closures.
#[derive(Clone, Copy, PartialEq, Debug)]
enum ToolbarAction {
    Back,
    ToggleToc,
    ToggleThumbs,
    PrevPage,
    NextPage,
    GoTo,
    ZoomOut,
    ZoomIn,
    Zoom100,
    FitWidth,
    FitPage,
    SetMode(ViewMode),
    Float,
    CycleTheme,
    Settings,
    About,
}

impl MariApp {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn show_toolbar(
        &mut self,
        ctx: &egui::Context,
        filename: &str,
        page_count: usize,
        current_page: usize,
        zoom_pct: u32,
        mode: ViewMode,
        color_mode: ColorMode,
        tb_alpha: u8,
        dw: f32,
    ) {
        let palette = self.settings.palette.0;
        let tok = Tok::new(color_mode, palette);
        let surface = color_mode.surface_for(palette);
        let fill = surface.gamma_multiply(self.toolbar_opacity.max(0.0));
        let border = color_mode.border().gamma_multiply(tb_alpha as f32 / 255.0);
        let side_panel = self.side_panel;
        let mut actions: Vec<ToolbarAction> = Vec::new();

        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(fill)
                    .stroke(egui::Stroke::new(dw, border))
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ctx, |ui| {
                let (row, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                    egui::Sense::hover(),
                );

                // ── Direita: zoom · modo · Flutuante · tema/config/sobre ──
                let mut right = zone(ui, row, egui::Layout::right_to_left(egui::Align::Center));
                right_zone(&mut right, &tok, color_mode, zoom_pct, mode, &mut actions);
                let right_left = right.min_rect().left();

                // ── Centro: ‹ n / N › ────────────────────────────────────
                let center_x = row
                    .center()
                    .x
                    .min(right_left - ZONE_GAP - PAGE_GROUP_WIDTH * 0.5);
                let center_rect = egui::Rect::from_center_size(
                    egui::pos2(center_x, row.center().y),
                    egui::vec2(PAGE_GROUP_WIDTH, ROW_HEIGHT),
                );
                let mut center = zone(
                    ui,
                    center_rect,
                    egui::Layout::left_to_right(egui::Align::Center),
                );
                page_group(&mut center, &tok, current_page, page_count, &mut actions);

                // ── Esquerda: voltar · painéis · nome ────────────────────
                let left_rect = egui::Rect::from_min_max(
                    row.min,
                    egui::pos2(center_rect.left() - ZONE_GAP, row.max.y),
                );
                let mut left = zone(
                    ui,
                    left_rect,
                    egui::Layout::left_to_right(egui::Align::Center),
                );
                left_zone(&mut left, &tok, side_panel, filename, &mut actions);
            });

        for action in actions {
            self.apply_toolbar_action(ctx, action);
        }
    }

    fn apply_toolbar_action(&mut self, ctx: &egui::Context, action: ToolbarAction) {
        match action {
            ToolbarAction::Back => {
                self.save_session();
                self.pending_close = true;
            }
            ToolbarAction::ToggleToc => self.toggle_side_panel(SidePanelKind::Toc),
            ToolbarAction::ToggleThumbs => self.toggle_side_panel(SidePanelKind::Thumbs),
            ToolbarAction::PrevPage if self.current_page > 0 => {
                let page = self.current_page - 1;
                self.go_to_page(ctx, page);
            }
            ToolbarAction::NextPage if self.current_page + 1 < self.page_count => {
                let page = self.current_page + 1;
                self.go_to_page(ctx, page);
            }
            ToolbarAction::PrevPage | ToolbarAction::NextPage => {}
            ToolbarAction::GoTo => {
                self.search_open = false;
                self.goto_open = true;
                self.goto_input.clear();
            }
            ToolbarAction::ZoomOut => self.apply_zoom(ctx, -0.15),
            ToolbarAction::ZoomIn => self.apply_zoom(ctx, 0.15),
            ToolbarAction::Zoom100 => self.set_zoom(ctx, 1.0),
            ToolbarAction::FitWidth => self.fit_width(ctx),
            ToolbarAction::FitPage => self.fit_page(ctx),
            ToolbarAction::SetMode(mode) => {
                if self.viewer.mode != mode {
                    self.do_toggle_mode(ctx);
                }
            }
            ToolbarAction::Float => self.enter_frame_mode(ctx),
            ToolbarAction::CycleTheme => {
                self.color_mode = self.color_mode.next();
                self.viewer.tex_cache.clear();
                self.viewer.texture = None;
                self.viewer.loaded_page = None;
                self.pending_rerender = true;
            }
            ToolbarAction::Settings => self.settings_open = !self.settings_open,
            ToolbarAction::About => self.about.open = !self.about.open,
        }
    }

    fn toggle_side_panel(&mut self, kind: SidePanelKind) {
        self.side_panel = if self.side_panel == kind {
            SidePanelKind::None
        } else {
            kind
        };
        if self.side_panel == SidePanelKind::Thumbs && self.thumb_cache.is_none() {
            if let Some(worker) = &self.document_worker {
                self.thumb_cache = Some(ThumbCache::new(self.page_count, worker.handle()));
            }
        }
    }
}

// ── Zonas ─────────────────────────────────────────────────────────────────────

fn zone(ui: &mut egui::Ui, rect: egui::Rect, layout: egui::Layout) -> egui::Ui {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    child.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
    child
}

fn left_zone(
    ui: &mut egui::Ui,
    tok: &Tok,
    side_panel: SidePanelKind,
    filename: &str,
    actions: &mut Vec<ToolbarAction>,
) {
    if tool_button(ui, icons::ARROW_LEFT, false, true, tok)
        .on_hover_text(t!("toolbar-back-hint"))
        .clicked()
    {
        actions.push(ToolbarAction::Back);
    }
    divider(ui, tok);
    if tool_button(ui, icons::LIST, side_panel == SidePanelKind::Toc, true, tok)
        .on_hover_text(t!("toolbar-toc-hint"))
        .clicked()
    {
        actions.push(ToolbarAction::ToggleToc);
    }
    if tool_button(
        ui,
        icons::SQUARES_FOUR,
        side_panel == SidePanelKind::Thumbs,
        true,
        tok,
    )
    .on_hover_text(t!("toolbar-thumbs-hint"))
    .clicked()
    {
        actions.push(ToolbarAction::ToggleThumbs);
    }

    let name_width = ui.available_width() - 8.0;
    if name_width >= MIN_NAME_WIDTH {
        ui.add_space(8.0);
        ui.allocate_ui(egui::vec2(name_width, ROW_HEIGHT), |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(filename).size(13.0).color(tok.text))
                    .truncate(),
            )
            .on_hover_text(filename);
        });
    }
}

fn page_group(
    ui: &mut egui::Ui,
    tok: &Tok,
    current_page: usize,
    page_count: usize,
    actions: &mut Vec<ToolbarAction>,
) {
    group(ui, tok, |ui| {
        if tool_button(ui, icons::CARET_LEFT, false, current_page > 0, tok)
            .on_hover_text(t!("toolbar-prev-hint"))
            .clicked()
        {
            actions.push(ToolbarAction::PrevPage);
        }
        let label = egui::RichText::new(format!("{} / {}", current_page + 1, page_count))
            .size(12.5)
            .monospace()
            .color(tok.text);
        let width = PAGE_GROUP_WIDTH - BTN_SIZE.x * 2.0 - 12.0;
        if text_button(ui, label, width, tok)
            .on_hover_text(t!("toolbar-goto-hint"))
            .clicked()
        {
            actions.push(ToolbarAction::GoTo);
        }
        if tool_button(
            ui,
            icons::CARET_RIGHT,
            false,
            current_page + 1 < page_count,
            tok,
        )
        .on_hover_text(t!("toolbar-next-hint"))
        .clicked()
        {
            actions.push(ToolbarAction::NextPage);
        }
    });
}

/// Desenhada da direita para a esquerda: o último item fica mais à esquerda.
fn right_zone(
    ui: &mut egui::Ui,
    tok: &Tok,
    color_mode: ColorMode,
    zoom_pct: u32,
    mode: ViewMode,
    actions: &mut Vec<ToolbarAction>,
) {
    if tool_button(ui, icons::INFO, false, true, tok)
        .on_hover_text(t!("toolbar-about-hint"))
        .clicked()
    {
        actions.push(ToolbarAction::About);
    }
    if tool_button(ui, icons::GEAR, false, true, tok)
        .on_hover_text(t!("toolbar-settings-hint"))
        .clicked()
    {
        actions.push(ToolbarAction::Settings);
    }
    let theme_icon = match color_mode {
        ColorMode::Dark => icons::MOON,
        ColorMode::Light => icons::SUN,
        ColorMode::Sepia => icons::LAMP_PENDANT,
    };
    if tool_button(ui, theme_icon, false, true, tok)
        .on_hover_text(t!("toolbar-color-hint"))
        .clicked()
    {
        actions.push(ToolbarAction::CycleTheme);
    }
    divider(ui, tok);

    if tool_button(ui, icons::PICTURE_IN_PICTURE, false, true, tok)
        .on_hover_text(t!("frame-enter"))
        .clicked()
    {
        actions.push(ToolbarAction::Float);
    }
    divider(ui, tok);

    // Grupos herdam o layout da direita para a esquerda: itens em ordem inversa.
    group(ui, tok, |ui| {
        for (glyph, value, hint) in [
            (icons::ROWS, ViewMode::Continuous, "toolbar-mode-continuous"),
            (icons::FILE, ViewMode::SinglePage, "toolbar-mode-single"),
        ] {
            if tool_button(ui, glyph, mode == value, true, tok)
                .on_hover_text(t!(hint))
                .clicked()
            {
                actions.push(ToolbarAction::SetMode(value));
            }
        }
    });
    ui.add_space(6.0);

    group(ui, tok, |ui| {
        if tool_button(ui, icons::PLUS, false, true, tok)
            .on_hover_text(t!("toolbar-zoom-in-hint"))
            .clicked()
        {
            actions.push(ToolbarAction::ZoomIn);
        }
        zoom_menu(ui, tok, zoom_pct, actions);
        if tool_button(ui, icons::MINUS, false, true, tok)
            .on_hover_text(t!("toolbar-zoom-out-hint"))
            .clicked()
        {
            actions.push(ToolbarAction::ZoomOut);
        }
    });
}

/// `100% ▾` abre um menu com zoom real e os ajustes à janela.
fn zoom_menu(ui: &mut egui::Ui, tok: &Tok, zoom_pct: u32, actions: &mut Vec<ToolbarAction>) {
    let label = egui::RichText::new(format!("{zoom_pct}%"))
        .size(12.5)
        .monospace()
        .color(tok.text);
    let resp = text_button(ui, label, 58.0, tok).on_hover_text(t!("toolbar-zoom-menu-hint"));
    let popup_id = ui.make_persistent_id("toolbar_zoom_menu");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup_below_widget(
        ui,
        popup_id,
        &resp,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(180.0);
            for (glyph, key, action) in [
                (
                    icons::ARROW_COUNTER_CW,
                    "toolbar-zoom-100",
                    ToolbarAction::Zoom100,
                ),
                (
                    icons::ARROWS_OUT_LINE_H,
                    "toolbar-fit-width-hint",
                    ToolbarAction::FitWidth,
                ),
                (
                    icons::FRAME_CORNERS,
                    "toolbar-fit-page-hint",
                    ToolbarAction::FitPage,
                ),
            ] {
                let mut text = egui::text::LayoutJob::default();
                text.append(
                    glyph,
                    0.0,
                    egui::TextFormat::simple(
                        egui::FontId::new(14.0, egui::FontFamily::Name("Icons".into())),
                        tok.icon,
                    ),
                );
                text.append(
                    &t!(key),
                    10.0,
                    egui::TextFormat::simple(egui::FontId::proportional(13.0), tok.text),
                );
                if ui.add(egui::Button::new(text).frame(false)).clicked() {
                    actions.push(action);
                }
            }
        },
    );
}

// ── Widgets ───────────────────────────────────────────────────────────────────

/// Contêiner discreto que agrupa controles relacionados.
fn group(ui: &mut egui::Ui, tok: &Tok, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(tok.group_fill)
        .stroke(egui::Stroke::new(1.0, tok.border))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            add(ui);
        });
}

/// Botão de ícone: fundo só no hover ou quando ativo, sempre por baixo do glifo.
fn tool_button(
    ui: &mut egui::Ui,
    glyph: &str,
    active: bool,
    enabled: bool,
    tok: &Tok,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(BTN_SIZE, sense);
    let bg = if active {
        tok.accent.gamma_multiply(0.16)
    } else if enabled && resp.hovered() {
        tok.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    let color = match (enabled, active) {
        (false, _) => tok.subtle,
        (true, true) => tok.accent,
        (true, false) if resp.hovered() => tok.text,
        (true, false) => tok.icon,
    };
    ui.painter()
        .rect_filled(rect.shrink(1.0), egui::CornerRadius::same(6), bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::new(ICON_SIZE, egui::FontFamily::Name("Icons".into())),
        color,
    );
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Botão de texto com o mesmo hover dos botões de ícone.
fn text_button(ui: &mut egui::Ui, label: egui::RichText, width: f32, tok: &Tok) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect.shrink(1.0), egui::CornerRadius::same(6), tok.hover);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, tok.text);
    resp
}

fn divider(ui: &mut egui::Ui, tok: &Tok) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(13.0, ROW_HEIGHT), egui::Sense::hover());
    let x = rect.center().x;
    ui.painter().line_segment(
        [
            egui::pos2(x, rect.top() + 6.0),
            egui::pos2(x, rect.bottom() - 6.0),
        ],
        egui::Stroke::new(1.0, tok.border),
    );
}
