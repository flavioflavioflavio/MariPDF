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
