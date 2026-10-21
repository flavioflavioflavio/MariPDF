use super::page_layout::{LayoutAnchor, LayoutParams, PageLayout};

fn params() -> LayoutParams {
    LayoutParams {
        available_width: 1_000.0,
        zoom: 1.0,
        margin: 20.0,
    }
}

#[test]
fn builds_prefix_offsets_from_estimates() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(3);
    assert!(layout.rebuild(params()));

    assert_eq!(layout.page_top(0), 20.0);
    assert!(layout.page_top(1) > layout.page_top(0));
    assert!(layout.total_height() > layout.page_top(2));
}

#[test]
fn observed_bitmap_replaces_estimated_height() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(2);
    layout.rebuild(params());
    let estimated = layout.page_height(0);

    assert!(layout.observe_bitmap(0, 500, 1_000, 1.0));
    assert!(layout.rebuild(params()));
    assert_ne!(layout.page_height(0), estimated);
}

#[test]
fn anchor_keeps_same_page_after_layout_change() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(4);
    layout.rebuild(params());
    let anchor = LayoutAnchor {
        page: 2,
        page_fraction: 0.4,
    };

    layout.observe_bitmap(0, 500, 1_200, 1.0);
    layout.rebuild(params());
    let offset = layout.scroll_offset_for_anchor(anchor, 700.0);

    assert_eq!(layout.anchor_at(offset + 350.0).page, 2);
}

#[test]
fn page_navigation_returns_its_top_offset() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(10);
    layout.rebuild(params());

    let offset = layout.scroll_offset_for_page(7);
    assert_eq!(layout.page_at(offset + 20.0), 7);
}

#[test]
fn dominant_page_has_largest_visible_area() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(3);
    layout.rebuild(params());
    let split = layout.page_top(1);

    assert_eq!(layout.dominant_page(split - 100.0, split + 600.0), 1);
}

#[test]
fn unseen_pages_are_still_capped_by_an_estimated_width() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(3);
    let density = 1.25;
    let unseen = crate::ui::viewer::page_raster_scale(&layout, 2, 4.0, 1200.0, density);
    let uncapped = crate::ui::page_scale::raster_scale(4.0, density);
    assert!(
        unseen < uncapped * 0.5,
        "página nunca vista não pode pedir o zoom cheio"
    );
}

#[test]
fn quarter_turn_swaps_learned_sizes_and_the_estimate() {
    let mut layout = PageLayout::new();
    layout.ensure_page_count(3);
    layout.observe_bitmap(0, 595, 842, 1.0);
    layout.rebuild(params());
    let portrait_total = layout.total_height();

    layout.set_axes_swapped(true);
    assert_eq!(layout.page_size_pts(0), Some((842.0, 595.0)));
    assert_eq!(layout.default_size(), (842.0, 595.0));
    assert!(layout.rebuild(params()));
    // Páginas deitadas ficam mais baixas: nada de estimativa em retrato.
    assert!(layout.total_height() < portrait_total * 0.8);

    layout.set_axes_swapped(false);
    assert_eq!(layout.page_size_pts(0), Some((595.0, 842.0)));
}
