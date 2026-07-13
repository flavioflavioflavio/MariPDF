use super::viewer::{PdfViewer, ViewMode};
use eframe::egui;

fn run_frame(ctx: &egui::Context, viewer: &mut PdfViewer, page_count: usize) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_000.0, 700.0),
        )),
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
    viewer.request_continuous_page(7);

    run_frame(&ctx, &mut viewer, 20);
    assert_eq!(viewer.dominant_continuous_page(), Some(7));

    viewer.zoom = 1.5;
    run_frame(&ctx, &mut viewer, 20);
    assert_eq!(viewer.dominant_continuous_page(), Some(7));

    viewer.request_continuous_page(2);
    run_frame(&ctx, &mut viewer, 20);
    assert_eq!(viewer.dominant_continuous_page(), Some(2));
}
