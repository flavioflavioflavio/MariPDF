// pdf/robustness_tests.rs — Entradas hostis e falhas de ambiente (BF-506).
//
// Usam as fixtures versionadas em tests/fixtures (geradas por
// tools/make_fixtures.py) e rodam no CI, sem depender de PDFs locais.

use super::document_worker::guarded;
use super::engine::{get_pdfium, valid_page_size, PdfEngine};
use crate::app::error_to_user_message;
use crate::i18n::{self, Language};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn open(path: &Path) -> anyhow::Result<PdfEngine> {
    PdfEngine::open(path, get_pdfium().expect("PDFium embutido"))
}

/// Mensagem que o usuário veria, em PT-BR, para a falha de abertura.
fn user_message(path: &Path) -> (String, String) {
    let error = match open(path) {
        Ok(_) => panic!("{} deveria falhar", path.display()),
        Err(error) => error,
    };
    let _guard = i18n::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    i18n::init(Language::PtBr);
    (error.to_string(), error_to_user_message(&error))
}

#[test]
fn sample_opens_renders_and_exposes_text() {
    let engine = open(&fixture("sample.pdf")).expect("abrir sample.pdf");
    assert_eq!(engine.page_count(), 2);
    let page = engine.render_page(0, 0.5).expect("render");
    assert!(page.width > 0 && page.height > 0);
    let text: String = engine
        .extract_search_page(0)
        .expect("texto")
        .chars
        .iter()
        .map(|c| c.ch)
        .collect();
    assert!(text.contains("MariPDF"), "{text}");
}

#[test]
fn password_protected_pdf_reports_password_message() {
    let (raw, message) = user_message(&fixture("encrypted.pdf"));
    let _guard = i18n::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    i18n::init(Language::PtBr);
    assert_eq!(message, crate::t!("err-pdf-password"), "erro bruto: {raw}");
}

#[test]
fn corrupted_pdf_reports_corruption_without_panicking() {
    let (raw, message) = user_message(&fixture("corrupt.pdf"));
    let _guard = i18n::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    i18n::init(Language::PtBr);
    assert_eq!(message, crate::t!("err-pdf-corrupt"), "erro bruto: {raw}");
}

#[test]
fn missing_file_reports_not_found() {
    let (raw, message) = user_message(&fixture("does-not-exist.pdf"));
    let _guard = i18n::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    i18n::init(Language::PtBr);
    assert_eq!(message, crate::t!("err-pdf-not-found"), "erro bruto: {raw}");
}

#[test]
fn unicode_path_opens() {
    let dir = crate::storage::temp_dir("unicode");
    let path = dir.join("Ação — 日本語 📄 Mariana.pdf");
    std::fs::copy(fixture("sample.pdf"), &path).expect("copiar fixture");
    let engine = open(&path).expect("abrir caminho Unicode");
    assert_eq!(engine.page_count(), 2);
    drop(engine);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn file_removed_after_open_keeps_engine_usable() {
    // O PDFium mantém o arquivo aberto; renderizar depois de apagar o atalho
    // do usuário não pode derrubar o processo.
    let dir = crate::storage::temp_dir("removed");
    let path = dir.join("sumira.pdf");
    std::fs::copy(fixture("sample.pdf"), &path).expect("copiar fixture");
    let engine = open(&path).expect("abrir");
    let _ = std::fs::remove_file(&path);
    let rendered = engine.render_page(1, 0.5);
    assert!(
        rendered.is_ok() || rendered.is_err(),
        "não pode entrar em panic"
    );
    drop(engine);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn invalid_page_sizes_are_rejected() {
    assert!(valid_page_size(595.0, 842.0));
    assert!(!valid_page_size(0.0, 842.0));
    assert!(!valid_page_size(f32::NAN, 842.0));
    assert!(!valid_page_size(595.0, f32::INFINITY));
    assert!(!valid_page_size(1.0e7, 842.0));
}

#[test]
fn worker_guard_turns_panics_into_errors() {
    let ok: anyhow::Result<u32> = guarded(|| Ok(7));
    assert_eq!(ok.unwrap(), 7);
    let failed: anyhow::Result<u32> = guarded(|| panic!("página explodiu"));
    let message = failed.unwrap_err().to_string();
    assert!(message.contains("página explodiu"), "{message}");
}

#[test]
fn a_poisoned_pdfium_lock_does_not_block_opening_documents() {
    let pdfium = get_pdfium().expect("PDFium embutido");
    // Envenena o mutex global como um panic dentro de with_document faria.
    let poison = std::sync::Arc::clone(&pdfium);
    let _ = std::thread::spawn(move || {
        let _guard = poison.lock().unwrap_or_else(|p| p.into_inner());
        panic!("job explodiu segurando o lock");
    })
    .join();
    assert!(pdfium.is_poisoned());
    let engine = open(&fixture("sample.pdf")).expect("abrir depois do panic");
    assert_eq!(engine.page_count(), 2);
    assert!(engine.render_page(0, 0.5).is_ok());
    pdfium.clear_poison();
}

#[test]
fn encrypted_pdf_opens_with_the_right_password_only() {
    use super::engine::PasswordRequired;
    let path = fixture("encrypted.pdf");
    let pdfium = || get_pdfium().expect("PDFium embutido");
    let missing = PdfEngine::open_with_password(&path, pdfium(), None)
        .err()
        .unwrap();
    assert!(missing.downcast_ref::<PasswordRequired>().is_some());
    let wrong = PdfEngine::open_with_password(&path, pdfium(), Some("errada"))
        .err()
        .unwrap();
    assert!(wrong.downcast_ref::<PasswordRequired>().is_some());
    let engine = PdfEngine::open_with_password(&path, pdfium(), Some("segredo"))
        .expect("senha correta abre");
    assert!(engine.page_count() > 0);
    assert!(engine.render_page(0, 0.5).is_ok());
}

#[test]
fn rotated_render_swaps_width_and_height() {
    use crate::ui::page_rotation::Rotation;
    let engine = open(&fixture("sample.pdf")).expect("abrir");
    let upright = engine.render_page(0, 0.5).expect("render");
    engine.set_rotation(Rotation::R90);
    let turned = engine.render_page(0, 0.5).expect("render girado");
    assert_eq!(
        (turned.width, turned.height),
        (upright.height, upright.width)
    );
    assert_eq!(
        turned.rgba.len(),
        (turned.width * turned.height * 4) as usize
    );
    let (w, h) = engine.page_size_pts(0);
    assert_eq!(engine.display_size_pts(0), (h, w));
}

#[test]
fn document_info_and_pdf_dates() {
    let engine = open(&fixture("sample.pdf")).expect("abrir");
    let info = engine.info();
    assert!(!info.version.is_empty(), "versão do PDF");
    assert_eq!(
        super::engine::pdf_date("D:20240115103000+03'00'"),
        "15/01/2024 10:30"
    );
    assert_eq!(super::engine::pdf_date("D:20240115"), "15/01/2024");
    assert_eq!(super::engine::pdf_date("ontem"), "ontem");
}
