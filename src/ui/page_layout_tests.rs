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
