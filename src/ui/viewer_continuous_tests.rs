use super::scroll_motion::JUMP_SECS;
use super::viewer::{PdfViewer, ViewMode};
use eframe::egui;

fn run_frame_at(ctx: &egui::Context, viewer: &mut PdfViewer, page_count: usize, time: f64) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_000.0, 700.0),
        )),
        time: Some(time),
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            viewer.show(ui, 0, page_count, egui::Color32::WHITE, egui::Color32::GRAY);
        });
    });
}

#[test]
fn programmatic_navigation_and_zoom_preserve_continuous_anchor() {
    let ctx = egui::Context::default();
    let mut viewer = PdfViewer::new();
    viewer.mode = ViewMode::Continuous;
    // Sem Scroll Suave a navegação programática é instantânea.
    viewer.smooth_scroll = false;
    viewer.request_continuous_page(7);

    run_frame_at(&ctx, &mut viewer, 20, 0.0);
    assert_eq!(viewer.dominant_continuous_page(), Some(7));

    viewer.zoom = 1.5;
    run_frame_at(&ctx, &mut viewer, 20, 0.1);
    assert_eq!(viewer.dominant_continuous_page(), Some(7));

    viewer.request_continuous_page(2);
    run_frame_at(&ctx, &mut viewer, 20, 0.2);
    assert_eq!(viewer.dominant_continuous_page(), Some(2));
}

#[test]
fn smooth_scroll_animates_jumps_and_lands_on_the_target() {
    let ctx = egui::Context::default();
    let mut viewer = PdfViewer::new();
    viewer.mode = ViewMode::Continuous;
    viewer.smooth_scroll = true;

    // Primeiro frame estabelece o offset atual no topo.
    run_frame_at(&ctx, &mut viewer, 20, 0.0);
    assert_eq!(viewer.dominant_continuous_page(), Some(0));

    viewer.request_continuous_page(10);
    run_frame_at(&ctx, &mut viewer, 20, 0.0);
    assert!(viewer.scroll_tween.is_some(), "salto deve animar");
    run_frame_at(&ctx, &mut viewer, 20, JUMP_SECS * 0.5);
    let midway = viewer.dominant_continuous_page().unwrap();
    assert!(midway > 0 && midway < 10, "meio do caminho: {midway}");

    run_frame_at(&ctx, &mut viewer, 20, JUMP_SECS + 0.05);
    run_frame_at(&ctx, &mut viewer, 20, JUMP_SECS + 0.1);
    assert!(viewer.scroll_tween.is_none());
    assert_eq!(viewer.dominant_continuous_page(), Some(10));
}
