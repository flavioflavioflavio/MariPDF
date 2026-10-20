// ui/page_layout.rs — Índice virtual de offsets e alturas das páginas.

pub(crate) const DEFAULT_PAGE_WIDTH: f32 = 595.0;
const DEFAULT_PAGE_HEIGHT: f32 = 842.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct LayoutParams {
    pub available_width: f32,
    pub zoom: f32,
    pub margin: f32,
}

impl LayoutParams {
    fn approximately_equals(self, other: Self) -> bool {
        (self.available_width - other.available_width).abs() < 0.5
            && (self.zoom - other.zoom).abs() < 0.001
            && (self.margin - other.margin).abs() < 0.1
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayoutAnchor {
    pub page: usize,
    pub page_fraction: f32,
}

pub(crate) struct PageLayout {
    page_sizes_pts: Vec<Option<(f32, f32)>>,
    page_tops: Vec<f32>,
    page_heights: Vec<f32>,
    total_height: f32,
    params: Option<LayoutParams>,
    dirty: bool,
}

impl PageLayout {
    pub fn new() -> Self {
        Self {
            page_sizes_pts: Vec::new(),
            page_tops: Vec::new(),
            page_heights: Vec::new(),
            total_height: 0.0,
            params: None,
            dirty: true,
        }
    }

    pub fn clear(&mut self) {
        self.page_sizes_pts.clear();
        self.page_tops.clear();
        self.page_heights.clear();
        self.total_height = 0.0;
        self.params = None;
        self.dirty = true;
    }

    /// Tamanho em pontos PDF aprendido pelo bitmap já renderizado da página.
    pub fn page_size_pts(&self, page: usize) -> Option<(f32, f32)> {
        self.page_sizes_pts.get(page).copied().flatten()
    }

    pub fn ensure_page_count(&mut self, page_count: usize) {
        if self.page_sizes_pts.len() == page_count {
            return;
        }
        self.page_sizes_pts.resize(page_count, None);
        self.page_tops.resize(page_count, 0.0);
        self.page_heights.resize(page_count, 0.0);
        self.dirty = true;
    }

    /// Aprende as dimensões PDF a partir do bitmap já renderizado. A divisão
    /// pela escala recupera aproximadamente os pontos originais da página.
    pub fn observe_bitmap(
        &mut self,
        page: usize,
        bitmap_width: u32,
        bitmap_height: u32,
        render_scale: f32,
    ) -> bool {
        if page >= self.page_sizes_pts.len()
            || bitmap_width == 0
            || bitmap_height == 0
            || render_scale <= 0.0
        {
            return false;
        }

        let size = (
            bitmap_width as f32 / render_scale,
            bitmap_height as f32 / render_scale,
        );
        let changed = self.page_sizes_pts[page].is_none_or(|previous| {
            (previous.0 - size.0).abs() > 0.5 || (previous.1 - size.1).abs() > 0.5
        });
        if changed {
            self.page_sizes_pts[page] = Some(size);
            self.dirty = true;
        }
        changed
    }

    /// Reconstrói os prefixos somente quando dimensões ou parâmetros mudaram.
    /// Retorna true se offsets/alturas foram recalculados.
    pub fn rebuild(&mut self, params: LayoutParams) -> bool {
        let params_changed = self
            .params
            .is_none_or(|previous| !previous.approximately_equals(params));
        if !self.dirty && !params_changed {
            return false;
        }

        let usable_width = (params.available_width - params.margin * 2.0).max(100.0);
        let mut top = params.margin;
        for page in 0..self.page_sizes_pts.len() {
            let (page_width, page_height) =
                self.page_sizes_pts[page].unwrap_or((DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT));
            let natural_width = page_width * crate::ui::page_scale::display_scale(params.zoom);
            let display_width = natural_width.min(usable_width);
            let display_height = page_height * display_width / page_width.max(1.0);

            self.page_tops[page] = top;
            self.page_heights[page] = display_height.max(1.0);
            top += display_height + params.margin;
        }

        self.total_height = top;
        self.params = Some(params);
        self.dirty = false;
        true
    }

    pub fn total_height(&self) -> f32 {
        self.total_height
    }

    pub fn page_top(&self, page: usize) -> f32 {
        self.page_tops.get(page).copied().unwrap_or(0.0)
    }

    pub fn page_height(&self, page: usize) -> f32 {
        self.page_heights.get(page).copied().unwrap_or(1.0)
    }

    pub fn page_at(&self, content_y: f32) -> usize {
        if self.page_tops.is_empty() {
            return 0;
        }
        self.page_tops
            .partition_point(|top| *top <= content_y)
            .saturating_sub(1)
            .min(self.page_tops.len() - 1)
    }

    pub fn visible_range(&self, viewport_min: f32, viewport_max: f32) -> (usize, usize) {
        let first = self.page_at(viewport_min.max(0.0));
        let last = self.page_at(viewport_max.max(viewport_min));
        (first, last.max(first))
    }

    pub fn dominant_page(&self, viewport_min: f32, viewport_max: f32) -> usize {
        let (first, last) = self.visible_range(viewport_min, viewport_max);
        (first..=last)
            .max_by(|left, right| {
                let visible_height = |page: usize| {
                    let top = self.page_top(page);
                    let bottom = top + self.page_height(page);
                    (bottom.min(viewport_max) - top.max(viewport_min)).max(0.0)
                };
                visible_height(*left).total_cmp(&visible_height(*right))
            })
            .unwrap_or(first)
    }

    pub fn anchor_at(&self, content_y: f32) -> LayoutAnchor {
        let page = self.page_at(content_y);
        let fraction = ((content_y - self.page_top(page)) / self.page_height(page)).clamp(0.0, 1.0);
        LayoutAnchor {
            page,
            page_fraction: fraction,
        }
    }

    pub fn scroll_offset_for_anchor(&self, anchor: LayoutAnchor, viewport_height: f32) -> f32 {
        let content_y =
            self.page_top(anchor.page) + self.page_height(anchor.page) * anchor.page_fraction;
        (content_y - viewport_height * 0.5)
            .clamp(0.0, (self.total_height - viewport_height).max(0.0))
    }

    pub fn scroll_offset_for_page(&self, page: usize) -> f32 {
        let margin = self.params.map_or(0.0, |params| params.margin);
        (self.page_top(page) - margin).max(0.0)
    }
}
