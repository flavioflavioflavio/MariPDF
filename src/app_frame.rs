// app_frame.rs — Modo Flutuante (BF-415): o PDF vira a própria janela.
// Sem decoração do Windows, toolbar ou status bar; a página ocupa a janela,
// que fica travada na proporção da página. Mover, redimensionar, zoom e menu
// acontecem direto sobre a página, no estilo PureRef.

use crate::app::{MariApp, SidePanelKind, MAX_ZOOM, MIN_ZOOM};
use crate::t;
use crate::ui::viewer::ViewMode;
use crate::ui::viewer_helpers::paint_search_highlight;
use eframe::egui;

mod bar;

/// Espessura da moldura em pontos.
const FRAME_BORDER: f32 = 1.0;
/// Faixa sensível nas bordas para redimensionar.
const RESIZE_MARGIN: f32 = 6.0;
/// Menor lado aceito para a janela em modo Flutuante.
const MIN_FRAME_SIDE: f32 = 160.0;
const MAX_FRAME_ZOOM: f32 = 8.0;
/// Passo dos botões de zoom da barra.
const FRAME_ZOOM_STEP: f32 = 1.25;
/// Tempo sem mudança de tamanho antes de reajustar a proporção.
const SNAP_SETTLE_SECS: f64 = 0.2;

/// Estado do modo Flutuante. Nada aqui é persistido: cada sessão abre normal.
pub struct FrameMode {
    pub active: bool,
    pub always_on_top: bool,
    zoom: f32,
    pan: egui::Vec2,
    restore: Option<FrameRestore>,
    last_size: egui::Vec2,
    size_changed_at: f64,
    shown_page: Option<usize>,
    /// Ferramenta mão: com zoom, arrastar move a página em vez da janela.
    pan_tool: bool,
    /// Barra flutuante expandida (chevron aberto).
    bar_open: bool,
    /// Área da barra no frame anterior, para o fade por hover.
    bar_rect: Option<egui::Rect>,
}

impl Default for FrameMode {
    fn default() -> Self {
        Self {
            active: false,
            always_on_top: false,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            restore: None,
            last_size: egui::Vec2::ZERO,
            size_changed_at: 0.0,
            shown_page: None,
            pan_tool: false,
            bar_open: false,
            bar_rect: None,
        }
    }
}

/// O que o modo normal tinha antes de entrar no modo Flutuante.
struct FrameRestore {
    outer_pos: Option<egui::Pos2>,
    inner_size: Option<egui::Vec2>,
    view_mode: ViewMode,
    side_panel: SidePanelKind,
    viewer_zoom: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FrameAction {
    ToggleBar,
    ZoomIn,
    ZoomOut,
    TogglePan,
    ToggleOnTop,
    Next,
    Prev,
    GoTo,
    Open,
    ResetView,
    Exit,
    Close,
}

impl MariApp {
    pub(crate) fn toggle_frame_mode(&mut self, ctx: &egui::Context) {
        if self.frame.active {
            self.exit_frame_mode(ctx);
        } else {
            self.enter_frame_mode(ctx);
        }
    }

    pub(crate) fn enter_frame_mode(&mut self, ctx: &egui::Context) {
        if self.frame.active || self.engine.is_none() {
            return;
        }
        let (outer, inner) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));
        self.frame.restore = Some(FrameRestore {
            outer_pos: outer.map(|r| r.min),
            inner_size: inner.map(|r| r.size()),
            view_mode: self.viewer.mode,
            side_panel: self.side_panel,
            viewer_zoom: self.viewer.zoom,
        });

        if self.fullscreen {
            self.fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if self.viewer.mode == ViewMode::Continuous {
            self.do_toggle_mode(ctx);
        }
        self.side_panel = SidePanelKind::None;
        self.search_open = false;
        self.search.clear();

        self.frame.active = true;
        self.frame.zoom = 1.0;
        self.frame.pan = egui::Vec2::ZERO;
        self.frame.last_size = egui::Vec2::ZERO;
        self.frame.shown_page = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
            MIN_FRAME_SIDE,
            MIN_FRAME_SIDE,
        )));
        if self.frame.always_on_top {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
        }
        tracing::info!("viewer: modo Flutuante");
        ctx.request_repaint();
    }

    pub(crate) fn exit_frame_mode(&mut self, ctx: &egui::Context) {
        if !self.frame.active {
            return;
        }
        self.frame.active = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
            640.0, 480.0,
        )));

        if let Some(restore) = self.frame.restore.take() {
            if let Some(size) = restore.inner_size {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            }
            if let Some(pos) = restore.outer_pos {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
            self.side_panel = restore.side_panel;
            if self.engine.is_some() {
                self.set_zoom(ctx, restore.viewer_zoom);
                if restore.view_mode == ViewMode::Continuous {
                    self.do_toggle_mode(ctx);
                }
            }
        }
        tracing::info!("viewer: saiu do modo Flutuante");
        ctx.request_repaint();
    }

    pub(crate) fn ui_frame(&mut self, ctx: &egui::Context) {
        let color_mode = self.color_mode;
        let palette = self.settings.palette.0;
        self.viewer.invert_colors = matches!(color_mode, crate::app::ColorMode::Dark);
        self.viewer.render_scale = self.settings.render_quality.scale();
        self.viewer.search_highlight = self.active_search_highlight();
        self.viewer
            .sync_single_texture(ctx, self.current_page, self.page_count);
        if let Some(error) = self.viewer.take_error() {
            self.render_error = Some(error);
        }

        if self.frame.shown_page != Some(self.current_page) {
            self.frame.shown_page = Some(self.current_page);
            self.reset_frame_view();
        }

        let now = ctx.input(|i| i.time);
        let mut action = None;
        let mut base_rect = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(color_mode.bg_for(palette)))
            .show(ctx, |ui| {
                let window = ui.max_rect();
                let inner = window.shrink(FRAME_BORDER);
                let surface = ui.interact(
                    window,
                    egui::Id::new("frame_surface"),
                    egui::Sense::click_and_drag(),
                );

                let resizing = self.handle_frame_resize(ui, window);
                self.handle_frame_pointer(ctx, ui, &surface, resizing);

                if let Some(tex) = self.viewer.texture.clone() {
                    let aspect = tex.size_vec2().x / tex.size_vec2().y.max(1.0);
                    let base = fit_rect(inner, aspect);
                    let page_rect = zoomed_page_rect(base, self.frame.zoom, self.frame.pan);
                    let tint = color_mode.page_tint_for(palette);
                    ui.painter().with_clip_rect(inner).image(
                        tex.id(),
                        page_rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        tint,
                    );
                    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    clipped.set_clip_rect(inner);
                    paint_search_highlight(
                        &clipped,
                        page_rect,
                        self.viewer.search_highlight.as_ref(),
                        self.viewer.loaded_page,
                    );
                    self.viewer.last_page_rect = page_rect;
                    base_rect = Some((base, aspect));
                } else if let Some(err) = &self.render_error {
                    ui.painter().text(
                        inner.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("⚠ {err}"),
                        egui::FontId::proportional(13.0),
                        color_mode.danger(),
                    );
                } else {
                    ui.painter().text(
                        inner.center(),
                        egui::Align2::CENTER_CENTER,
                        "…",
                        egui::FontId::proportional(18.0),
                        color_mode.text_muted(),
                    );
                }

                ui.painter().rect_stroke(
                    window.shrink(FRAME_BORDER * 0.5),
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(FRAME_BORDER, color_mode.border()),
                    egui::StrokeKind::Inside,
                );

                surface.context_menu(|ui| {
                    action = frame_menu(ui, self.frame.always_on_top);
                });
            });

        let bar_action = self.frame_bar(ctx, ctx.screen_rect());

        if let Some((base, aspect)) = base_rect {
            self.keep_frame_resolution(ctx, base);
            self.keep_frame_aspect(ctx, aspect, now);
        }
        for action in [action, bar_action].into_iter().flatten() {
            self.apply_frame_action(ctx, action);
        }
    }

    /// Bordas e cantos iniciam o redimensionamento nativo. Retorna true se o
    /// ponteiro está sobre uma borda (o arraste não deve mover a janela).
    fn handle_frame_resize(&mut self, ui: &egui::Ui, window: egui::Rect) -> bool {
        let Some(pos) = ui.input(|i| i.pointer.hover_pos()) else {
            return false;
        };
        let Some(direction) = resize_direction(pos, window, RESIZE_MARGIN) else {
            return false;
        };
        ui.ctx().set_cursor_icon(resize_cursor(direction));
        if ui.input(|i| i.pointer.primary_pressed()) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
        true
    }

    fn handle_frame_pointer(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        surface: &egui::Response,
        resizing: bool,
    ) {
        let panning = self.frame.zoom > 1.0 && self.frame.pan_tool;
        if !resizing && surface.drag_started_by(egui::PointerButton::Primary) && !panning {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if panning && surface.dragged_by(egui::PointerButton::Primary) {
            self.frame.pan += surface.drag_delta();
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if panning && surface.hovered() && !resizing {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }
        if surface.double_clicked() {
            self.reset_frame_view();
        }

        if surface.hovered() {
            let (zoom_factor, ctrl, raw_y) =
                ui.input(|i| (i.zoom_delta(), i.modifiers.ctrl, i.raw_scroll_delta.y));
            let factor = if zoom_factor != 1.0 {
                zoom_factor
            } else if ctrl && raw_y != 0.0 {
                if raw_y > 0.0 {
                    1.15
                } else {
                    1.0 / 1.15
                }
            } else {
                1.0
            };
            if factor != 1.0 {
                if let Some(cursor) = surface.hover_pos() {
                    let inner = ui.max_rect().shrink(FRAME_BORDER);
                    let aspect = self
                        .viewer
                        .texture
                        .as_ref()
                        .map_or(1.0, |t| t.size_vec2().x / t.size_vec2().y.max(1.0));
                    let base = fit_rect(inner, aspect);
                    let (zoom, pan) = zoom_about(
                        self.frame.zoom,
                        self.frame.pan,
                        base.center(),
                        cursor,
                        factor,
                    );
                    self.set_frame_zoom(zoom, pan);
                }
            } else if !ctrl && raw_y.abs() > 5.0 {
                self.viewer.page_delta = if raw_y < 0.0 { 1 } else { -1 };
            }
        }

        let inner = ui.max_rect().shrink(FRAME_BORDER);
        let aspect = self
            .viewer
            .texture
            .as_ref()
            .map_or(1.0, |t| t.size_vec2().x / t.size_vec2().y.max(1.0));
        self.frame.pan = clamp_pan(
            self.frame.pan,
            fit_rect(inner, aspect).size(),
            self.frame.zoom,
        );
    }

    /// Troca de página volta ao enquadramento inteiro.
    pub(crate) fn reset_frame_view(&mut self) {
        self.frame.zoom = 1.0;
        self.frame.pan = egui::Vec2::ZERO;
        self.frame.pan_tool = false;
        self.frame.last_size = egui::Vec2::ZERO;
    }

    /// Aplica zoom/pan; ao sair do enquadramento a mão liga sozinha.
    fn set_frame_zoom(&mut self, zoom: f32, pan: egui::Vec2) {
        if self.frame.zoom <= 1.0 && zoom > 1.0 {
            self.frame.pan_tool = true;
        }
        if zoom <= 1.0 {
            self.frame.pan_tool = false;
        }
        self.frame.zoom = zoom;
        self.frame.pan = pan;
    }

    /// Zoom pelos botões da barra, ancorado no centro da página.
    fn step_frame_zoom(&mut self, factor: f32) {
        let (zoom, pan) = zoom_about(
            self.frame.zoom,
            self.frame.pan,
            egui::Pos2::ZERO,
            egui::Pos2::ZERO,
            factor,
        );
        self.set_frame_zoom(zoom, pan);
    }

    /// Ajusta a resolução do bitmap ao tamanho exibido, sem renderizar durante
    /// o arraste e só quando a diferença é perceptível.
    fn keep_frame_resolution(&mut self, ctx: &egui::Context, base: egui::Rect) {
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let Some((_, page_h_pts)) = self.viewer.page_layout.page_size_pts(self.current_page) else {
            return;
        };
        let target = frame_render_zoom(base.height(), self.frame.zoom, page_h_pts);
        let current = self.viewer.zoom;
        if target > current * 1.15 || target < current * 0.5 {
            self.set_zoom(ctx, target);
        }
    }

    /// Depois que o tamanho estabiliza, ajusta a largura à proporção da página.
    fn keep_frame_aspect(&mut self, ctx: &egui::Context, aspect: f32, now: f64) {
        let Some(inner) = ctx.input(|i| i.viewport().inner_rect) else {
            return;
        };
        let size = inner.size();
        if (size - self.frame.last_size).length() > 0.5 {
            self.frame.last_size = size;
            self.frame.size_changed_at = now;
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(SNAP_SETTLE_SECS));
            return;
        }
        if now - self.frame.size_changed_at < SNAP_SETTLE_SECS
            || ctx.input(|i| i.pointer.any_down())
        {
            return;
        }
        let target = snapped_inner_size(size, aspect, FRAME_BORDER);
        if (target - size).length() > 1.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target));
        }
    }

    fn apply_frame_action(&mut self, ctx: &egui::Context, action: FrameAction) {
        match action {
            FrameAction::ToggleBar => self.frame.bar_open = !self.frame.bar_open,
            FrameAction::ZoomIn => self.step_frame_zoom(FRAME_ZOOM_STEP),
            FrameAction::ZoomOut => self.step_frame_zoom(1.0 / FRAME_ZOOM_STEP),
            FrameAction::TogglePan => self.frame.pan_tool = !self.frame.pan_tool,
            FrameAction::ToggleOnTop => {
                self.frame.always_on_top = !self.frame.always_on_top;
                let level = if self.frame.always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            }
            FrameAction::Next if self.current_page + 1 < self.page_count => {
                let page = self.current_page + 1;
                self.go_to_page(ctx, page);
            }
            FrameAction::Prev if self.current_page > 0 => {
                let page = self.current_page - 1;
                self.go_to_page(ctx, page);
            }
            FrameAction::Next | FrameAction::Prev => {}
            FrameAction::GoTo => {
                self.goto_open = true;
                self.goto_input.clear();
            }
            FrameAction::Open => self.open_file_dialog(),
            FrameAction::ResetView => self.reset_frame_view(),
            FrameAction::Exit => self.exit_frame_mode(ctx),
            FrameAction::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
}

fn frame_menu(ui: &mut egui::Ui, always_on_top: bool) -> Option<FrameAction> {
    let mut action = None;
    let mut item = |ui: &mut egui::Ui, label: String, value: FrameAction| {
        if ui.button(label).clicked() {
            action = Some(value);
            ui.close_menu();
        }
    };
    let on_top = format!(
        "{} {}",
        if always_on_top { "✓" } else { "   " },
        t!("frame-always-on-top")
    );
    item(ui, on_top, FrameAction::ToggleOnTop);
    ui.separator();
    item(ui, t!("frame-next"), FrameAction::Next);
    item(ui, t!("frame-prev"), FrameAction::Prev);
    item(ui, t!("frame-goto"), FrameAction::GoTo);
    item(ui, t!("frame-reset-view"), FrameAction::ResetView);
    ui.separator();
    item(ui, t!("frame-open"), FrameAction::Open);
    item(ui, t!("frame-exit"), FrameAction::Exit);
    item(ui, t!("frame-close"), FrameAction::Close);
    action
}

// ── Geometria pura (testada) ──────────────────────────────────────────────────

/// Maior retângulo com a proporção `aspect` (largura/altura) centrado em `container`.
fn fit_rect(container: egui::Rect, aspect: f32) -> egui::Rect {
    let aspect = if aspect > 0.0 { aspect } else { 1.0 };
    let mut size = container.size();
    if size.x / size.y.max(f32::EPSILON) > aspect {
        size.x = size.y * aspect;
    } else {
        size.y = size.x / aspect;
    }
    egui::Rect::from_center_size(container.center(), size)
}

/// Página ampliada em torno do centro do enquadramento e deslocada por `pan`.
fn zoomed_page_rect(base: egui::Rect, zoom: f32, pan: egui::Vec2) -> egui::Rect {
    egui::Rect::from_center_size(base.center() + pan, base.size() * zoom)
}

/// Novo zoom e pan mantendo fixo o ponto da página sob o cursor.
fn zoom_about(
    zoom: f32,
    pan: egui::Vec2,
    center: egui::Pos2,
    cursor: egui::Pos2,
    factor: f32,
) -> (f32, egui::Vec2) {
    let new_zoom = (zoom * factor).clamp(1.0, MAX_FRAME_ZOOM);
    let page_point = (cursor - center - pan) / zoom;
    let new_pan = cursor - center - page_point * new_zoom;
    (new_zoom, new_pan)
}

/// Impede que o pan revele área fora da página.
fn clamp_pan(pan: egui::Vec2, base_size: egui::Vec2, zoom: f32) -> egui::Vec2 {
    let limit = base_size * (zoom - 1.0).max(0.0) * 0.5;
    egui::vec2(
        pan.x.clamp(-limit.x, limit.x),
        pan.y.clamp(-limit.y, limit.y),
    )
}

fn resize_direction(
    pos: egui::Pos2,
    window: egui::Rect,
    margin: f32,
) -> Option<egui::ResizeDirection> {
    use egui::ResizeDirection as Dir;
    if !window.expand(1.0).contains(pos) {
        return None;
    }
    let left = pos.x - window.left() <= margin;
    let right = window.right() - pos.x <= margin;
    let top = pos.y - window.top() <= margin;
    let bottom = window.bottom() - pos.y <= margin;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(Dir::NorthWest),
        (_, true, true, _) => Some(Dir::NorthEast),
        (true, _, _, true) => Some(Dir::SouthWest),
        (_, true, _, true) => Some(Dir::SouthEast),
        (true, ..) => Some(Dir::West),
        (_, true, ..) => Some(Dir::East),
        (_, _, true, _) => Some(Dir::North),
        (_, _, _, true) => Some(Dir::South),
        _ => None,
    }
}

fn resize_cursor(direction: egui::ResizeDirection) -> egui::CursorIcon {
    use egui::ResizeDirection as Dir;
    match direction {
        Dir::North | Dir::South => egui::CursorIcon::ResizeVertical,
        Dir::East | Dir::West => egui::CursorIcon::ResizeHorizontal,
        Dir::NorthEast | Dir::SouthWest => egui::CursorIcon::ResizeNeSw,
        Dir::NorthWest | Dir::SouthEast => egui::CursorIcon::ResizeNwSe,
    }
}

/// Mantém a altura da janela e ajusta a largura para a proporção da página.
fn snapped_inner_size(size: egui::Vec2, aspect: f32, border: f32) -> egui::Vec2 {
    let height = size.y.max(MIN_FRAME_SIDE);
    let page_height = height - border * 2.0;
    let width = (page_height * aspect + border * 2.0).max(MIN_FRAME_SIDE);
    egui::vec2(width.round(), height.round())
}

/// Zoom de render que gera um bitmap nítido para a altura exibida.
fn frame_render_zoom(display_height: f32, frame_zoom: f32, page_height_pts: f32) -> f32 {
    (display_height * frame_zoom / page_height_pts.max(1.0)).clamp(MIN_ZOOM, MAX_ZOOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn fit_rect_letterboxes_wide_and_tall_containers() {
        let tall_page = fit_rect(rect(0.0, 0.0, 400.0, 200.0), 0.5);
        assert_eq!(tall_page.size(), egui::vec2(100.0, 200.0));
        assert_eq!(tall_page.center(), egui::pos2(200.0, 100.0));

        let wide_page = fit_rect(rect(0.0, 0.0, 200.0, 400.0), 2.0);
        assert_eq!(wide_page.size(), egui::vec2(200.0, 100.0));
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let center = egui::pos2(100.0, 100.0);
        let cursor = egui::pos2(150.0, 120.0);
        let (zoom, pan) = zoom_about(1.0, egui::Vec2::ZERO, center, cursor, 2.0);
        assert_eq!(zoom, 2.0);
        // O ponto (50, 20) do centro continua sob o cursor após dobrar o zoom.
        let page_point = egui::vec2(50.0, 20.0);
        assert_eq!(center + pan + page_point * zoom, cursor);
    }

    #[test]
    fn centered_zoom_scales_pan_proportionally() {
        let (zoom, pan) = zoom_about(
            2.0,
            egui::vec2(10.0, -4.0),
            egui::Pos2::ZERO,
            egui::Pos2::ZERO,
            1.5,
        );
        assert_eq!(zoom, 3.0);
        assert_eq!(pan, egui::vec2(15.0, -6.0));
    }

    #[test]
    fn zoom_never_goes_below_fit() {
        let (zoom, _) = zoom_about(
            1.0,
            egui::Vec2::ZERO,
            egui::Pos2::ZERO,
            egui::Pos2::ZERO,
            0.5,
        );
        assert_eq!(zoom, 1.0);
    }

    #[test]
    fn pan_is_limited_to_the_zoomed_overflow() {
        let base = egui::vec2(100.0, 200.0);
        assert_eq!(
            clamp_pan(egui::vec2(500.0, -500.0), base, 2.0),
            egui::vec2(50.0, -100.0)
        );
        assert_eq!(
            clamp_pan(egui::vec2(30.0, 30.0), base, 1.0),
            egui::Vec2::ZERO
        );
    }

    #[test]
    fn resize_direction_detects_edges_and_corners() {
        use egui::ResizeDirection as Dir;
        let window = rect(0.0, 0.0, 300.0, 400.0);
        assert_eq!(
            resize_direction(egui::pos2(2.0, 2.0), window, 6.0),
            Some(Dir::NorthWest)
        );
        assert_eq!(
            resize_direction(egui::pos2(298.0, 398.0), window, 6.0),
            Some(Dir::SouthEast)
        );
        assert_eq!(
            resize_direction(egui::pos2(150.0, 399.0), window, 6.0),
            Some(Dir::South)
        );
        assert_eq!(
            resize_direction(egui::pos2(1.0, 200.0), window, 6.0),
            Some(Dir::West)
        );
        assert_eq!(
            resize_direction(egui::pos2(150.0, 200.0), window, 6.0),
            None
        );
    }

    #[test]
    fn snapped_size_keeps_height_and_page_aspect() {
        // A4 retrato (~0,707) numa janela de 800pt de altura com borda de 1pt.
        let size = snapped_inner_size(egui::vec2(1000.0, 800.0), 595.0 / 842.0, 1.0);
        assert_eq!(size.y, 800.0);
        assert_eq!(size.x, ((798.0 * 595.0 / 842.0) + 2.0_f32).round());
        // Nunca abaixo do mínimo.
        assert_eq!(
            snapped_inner_size(egui::vec2(50.0, 50.0), 0.1, 1.0).y,
            MIN_FRAME_SIDE
        );
    }

    #[test]
    fn render_zoom_tracks_displayed_height() {
        assert_eq!(frame_render_zoom(842.0, 1.0, 842.0), 1.0);
        assert_eq!(frame_render_zoom(421.0, 2.0, 842.0), 1.0);
        assert_eq!(frame_render_zoom(10_000.0, 8.0, 842.0), MAX_ZOOM);
    }
}
