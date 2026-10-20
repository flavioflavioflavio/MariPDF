// pdf/tile_spike_tests.rs — Spike da BF-301: render por região no PDFium.
//
// 1. Correção: um tile renderizado isoladamente precisa bater, pixel a pixel,
//    com o recorte da página inteira (roda no CI com a fixture).
// 2. Custo: com MARIPDF_CORPUS apontando para bench/corpus, compara render da
//    página inteira em zoom alto × só os tiles visíveis × bitmap limitado ao
//    tamanho exibido (`--ignored --nocapture`).

use super::engine::{get_pdfium, PdfEngine, RenderedPage};
use std::path::Path;
use std::time::Instant;

fn open(path: &Path) -> PdfEngine {
    PdfEngine::open(path, get_pdfium().expect("PDFium")).expect("abrir PDF")
}

fn crop(full: &RenderedPage, (x, y, w, h): (u32, u32, u32, u32)) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let start = ((row * full.width + x) * 4) as usize;
        out.extend_from_slice(&full.rgba[start..start + (w * 4) as usize]);
    }
    out
}

#[test]
fn tile_matches_the_same_region_of_the_full_page() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    let engine = open(&fixture);
    let scale = 2.0;
    let full = engine.render_page(0, scale).expect("página inteira");
    // Região com texto (cabeçalho do sample) e fora da origem.
    let region = (96, 120, 320, 96);
    let tile = engine.render_region(0, scale, region).expect("tile");
    let expected = crop(&full, region);
    assert_eq!(tile.rgba.len(), expected.len());
    let differing = tile
        .rgba
        .chunks(4)
        .zip(expected.chunks(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(p, q)| p.abs_diff(*q) > 8))
        .count();
    let inked = expected.chunks(4).filter(|px| px[0] < 128).count();
    assert!(inked > 100, "a região de teste precisa conter texto");
    // Antialiasing pode variar em poucos pixels de borda.
    assert!(
        differing * 100 <= inked,
        "{differing} pixels diferentes de {inked} com tinta"
    );
}

#[test]
#[ignore = "medição: MARIPDF_CORPUS=bench/corpus cargo test --release -- --ignored --nocapture tile_spike"]
fn tile_spike_costs() {
    let corpus = std::env::var("MARIPDF_CORPUS").expect("defina MARIPDF_CORPUS");
    // Viewport típico 1600×1000 pontos a 125% de escala, qualidade 1×.
    let (view_w, view_h, density) = (1600.0_f32, 1000.0_f32, 1.25_f32);
    println!("| PDF | Caso | Escala | Bitmap (MPx) | Tempo (ms) |");
    println!("|---|---|---|---|---|");
    for name in ["text-100.pdf", "scan-10.pdf", "complex-fonts.pdf"] {
        let engine = open(&Path::new(&corpus).join(name));
        let page = 1.min(engine.page_count() - 1);
        let (pw, _) = engine.page_size_pts(page);
        // Zoom 400% pedido; na tela a largura é limitada à janela.
        let zoomed = 4.0 * (96.0 / 72.0) * density;
        let displayed = (view_w * density / pw).min(zoomed);
        let measure = |label: &str, scale: f32, render: &dyn Fn() -> RenderedPage| {
            let _ = render();
            let started = Instant::now();
            let runs = 3;
            let mut px = 0;
            for _ in 0..runs {
                let page = render();
                px = page.width as usize * page.height as usize;
            }
            let ms = started.elapsed().as_secs_f64() * 1000.0 / runs as f64;
            println!(
                "| {name} | {label} | {scale:.2} | {:.1} | {ms:.1} |",
                px as f64 / 1e6
            );
        };
        measure("página inteira no zoom pedido (hoje)", zoomed, &|| {
            engine.render_page(page, zoomed).unwrap()
        });
        measure("página inteira limitada ao exibido", displayed, &|| {
            engine.render_page(page, displayed).unwrap()
        });
        let tile = (0, 0, (view_w * density) as u32, (view_h * density) as u32);
        measure("só a viewport (tiles) no zoom exibido", displayed, &|| {
            engine.render_region(page, displayed, tile).unwrap()
        });
    }
}
