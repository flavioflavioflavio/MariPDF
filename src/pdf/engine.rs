// pdf/engine.rs — Wrapper sobre pdfium-render
//
// Engine de renderização de PDFs usando o pdfium do Chromium.
// Responsabilidade: abrir documentos, contar páginas, renderizar RGBA.
// Extração de conteúdo (texto, links, bookmarks) delegada a engine_extract.
//
// ADR-002: pdfium na Fase 1 (binários prebuilt, zero build time);
//          migração para MuPDF nativo quando mupdf-sys suportar MSVC 2022.
//
// DESIGN NOTE (singleton Pdfium serializado):
//   O Pdfium e single-threaded. A feature `sync` de pdfium-render permite mover
//   seus wrappers entre threads, mas nao torna chamadas simultaneas seguras.
//   Por isso a DLL e carregada uma unica vez e TODA operacao nativa, inclusive
//   o fechamento de documentos, passa pelo mesmo Mutex. Os workers continuam
//   fora da UI, mas nunca entram no Pdfium concorrentemente.

use pdfium_render::prelude::*;
use std::{
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

use crate::pdf::extract as engine_extract;

/// Re-exporta tipos de extract.rs para manter compatibilidade com todos os sites de uso.
pub use crate::pdf::extract::{BookmarkEntry, PdfLinkInfo, TextCharInfo};

/// DLL do pdfium embutida no executável em compile-time.
/// Em runtime, é extraída para %TEMP%\maripdf\ e carregada de lá.
/// Isso torna o executável completamente portátil (single-file).
static PDFIUM_DLL_BYTES: &[u8] = include_bytes!("../../assets/pdfium.dll");

/// Singleton global do Pdfium — carregado uma vez, nunca descarregado.
/// Compartilhado entre main thread e threads de background via Arc.
static PDFIUM_INSTANCE: OnceLock<Arc<Mutex<Pdfium>>> = OnceLock::new();
static PDFIUM_INIT_LOCK: Mutex<()> = Mutex::new(());

/// Retorna (ou inicializa) o singleton global do Pdfium.
/// `PDFIUM_INIT_LOCK` garante uma unica inicializacao; o `OnceLock` publica o resultado.
pub fn get_pdfium() -> anyhow::Result<Arc<Mutex<Pdfium>>> {
    if let Some(arc) = PDFIUM_INSTANCE.get() {
        return Ok(Arc::clone(arc));
    }

    // `Pdfium::new()` segura um lock global interno durante toda a vida da
    // instancia. Portanto nao podemos deixar dois callers inicializarem em
    // paralelo e resolver a corrida apenas no `OnceLock::set`: o perdedor
    // ficaria bloqueado para sempre antes de chegar ao `set`.
    let _init_guard = PDFIUM_INIT_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Lock de inicializacao do Pdfium foi corrompido"))?;

    // Outro thread pode ter concluido enquanto aguardavamos o lock.
    if let Some(arc) = PDFIUM_INSTANCE.get() {
        return Ok(Arc::clone(arc));
    }

    // Primeira chamada efetiva: extrai e carrega a DLL.
    let pdfium = make_pdfium()?;
    let instance = Arc::new(Mutex::new(pdfium));
    PDFIUM_INSTANCE
        .set(Arc::clone(&instance))
        .map_err(|_| anyhow::anyhow!("Falha inesperada ao registrar singleton do Pdfium"))?;
    Ok(instance)
}

/// Cria e retorna uma instância do pdfium (carrega a DLL).
/// Prefira `get_pdfium()` para reutilizar o singleton.
pub fn make_pdfium() -> anyhow::Result<Pdfium> {
    let dll_path = extract_pdfium_dll()?;
    let dll_dir = dll_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Caminho da DLL inválido"))?;

    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
        dll_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Caminho com caracteres inválidos"))?,
    ))
    .map_err(|e| anyhow::anyhow!("Falha ao carregar pdfium.dll: {e}"))?;

    Ok(Pdfium::new(bindings))
}

/// Extrai a DLL embedded para %TEMP%\maripdf\pdfium.dll se necessário.
/// Reutiliza o arquivo existente se o tamanho bater (evita writes desnecessários).
fn extract_pdfium_dll() -> anyhow::Result<std::path::PathBuf> {
    let target_dir = std::env::temp_dir().join("maripdf");
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| anyhow::anyhow!("Não foi possível criar dir temp: {e}"))?;

    let dll_path = target_dir.join("pdfium.dll");

    // Só reescreve se o arquivo não existe ou tem tamanho diferente
    let should_write = match std::fs::metadata(&dll_path) {
        Ok(meta) => meta.len() != PDFIUM_DLL_BYTES.len() as u64,
        Err(_) => true,
    };

    if should_write {
        tracing::info!("Extraindo pdfium.dll para {}", dll_path.display());
        std::fs::write(&dll_path, PDFIUM_DLL_BYTES)
            .map_err(|e| anyhow::anyhow!("Falha ao extrair pdfium.dll: {e}"))?;
    }

    Ok(dll_path)
}

/// Representa um documento PDF aberto e pronto para renderizar.
pub struct PdfEngine {
    /// Referência ao singleton do pdfium (mantida para garantir que a DLL
    /// não seja descarregada enquanto o doc ainda existe).
    #[allow(dead_code)]
    pdfium: Arc<Mutex<Pdfium>>,
    doc: Option<PdfDocument<'static>>,
    page_count: usize,
}

impl PdfEngine {
    /// Abre um arquivo PDF usando pdfium.
    pub fn open(path: &Path, pdfium: Arc<Mutex<Pdfium>>) -> anyhow::Result<Self> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Caminho com caracteres inválidos"))?;

        let (doc, page_count) = {
            let pdfium_guard = pdfium
                .lock()
                .map_err(|_| anyhow::anyhow!("Lock global do Pdfium foi corrompido"))?;

            // SAFETY: lifetime 'static aqui e um contorno — o doc vive junto com
            // o Arc<Mutex<Pdfium>> mantido na struct. O Drop abaixo tambem fecha
            // o documento enquanto segura esse mesmo lock.
            let doc = unsafe {
                std::mem::transmute::<PdfDocument<'_>, PdfDocument<'static>>(
                    pdfium_guard
                        .load_pdf_from_file(path_str, None)
                        .map_err(|e| anyhow::anyhow!("Falha ao abrir PDF: {e}"))?,
                )
            };
            let page_count = usize::from(doc.pages().len());
            (doc, page_count)
        };

        tracing::info!(
            "PDF aberto: {} ({} páginas)",
            path.file_name().unwrap_or_default().to_string_lossy(),
            page_count
        );

        Ok(Self {
            pdfium,
            doc: Some(doc),
            page_count,
        })
    }

    fn with_document<T>(
        &self,
        operation: impl FnOnce(&PdfDocument<'static>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let _pdfium_guard = self
            .pdfium
            .lock()
            .map_err(|_| anyhow::anyhow!("Lock global do Pdfium foi corrompido"))?;
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Documento PDF ja foi fechado"))?;
        operation(doc)
    }

    /// Número total de páginas do documento.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Dimensões da página em pontos PDF (largura, altura).
    /// Necessário para converter coordenadas de tela ↔ coordenadas PDF.
    pub fn page_size_pts(&self, page_index: usize) -> (f32, f32) {
        self.with_document(|doc| {
            Ok(doc
                .pages()
                .get(page_index as u16)
                .map(|p| (p.width().value, p.height().value))
                .unwrap_or((595.0, 842.0)))
        })
        .unwrap_or((595.0, 842.0)) // A4 como fallback
    }

    /// Renderiza uma página como bitmap RGBA.
    ///
    /// # Parâmetros
    /// - `page_index`: índice da página (0-based)
    /// - `scale`: fator de escala (1.0 = 72dpi, 2.0 = 144dpi/HD)
    pub fn render_page(&self, page_index: usize, scale: f32) -> anyhow::Result<RenderedPage> {
        self.with_document(|doc| {
            let page = doc
                .pages()
                .get(page_index as u16)
                .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

            let width_pts = page.width().value;
            let height_pts = page.height().value;

            // Converte pontos (72dpi) para pixels com o fator de escala
            let width_px = (width_pts * scale) as u32;
            let height_px = (height_pts * scale) as u32;

            let render_cfg = PdfRenderConfig::new()
                .set_target_width(width_px as i32)
                .set_target_height(height_px as i32)
                // PDFium usa BGRA nativamente. Este flag pede a inversao no
                // proprio render e permite entregar RGBA sem um segundo buffer.
                .set_reverse_byte_order(true);

            let bitmap = page
                .render_with_config(&render_cfg)
                .map_err(|e| anyhow::anyhow!("Falha ao renderizar página {page_index}: {e}"))?;

            // Com reverse_byte_order, `as_rgba_bytes()` devolve diretamente o
            // Vec copiado do bitmap, sem nova alocacao ou varredura por pixel.
            let rgba = bitmap.as_rgba_bytes();

            Ok(RenderedPage {
                width: width_px,
                height: height_px,
                rgba,
            })
        })
    }

    /// Extrai o texto de uma página como String.
    pub fn extract_text(&self, page_index: usize) -> anyhow::Result<String> {
        self.with_document(|doc| engine_extract::extract_text(doc, page_index))
    }

    /// Extrai todos os chars de uma página com seus bounding boxes em pontos PDF.
    pub fn extract_chars_with_bounds(
        &self,
        page_index: usize,
    ) -> anyhow::Result<Vec<TextCharInfo>> {
        self.with_document(|doc| engine_extract::extract_chars_with_bounds(doc, page_index))
    }

    /// Extrai todos os links URI de uma página.
    pub fn extract_page_links(&self, page_index: usize) -> Vec<PdfLinkInfo> {
        self.with_document(|doc| Ok(engine_extract::extract_page_links(doc, page_index)))
            .unwrap_or_default()
    }

    /// Extrai a árvore de bookmarks (Table-of-Contents) do documento.
    pub fn extract_bookmarks(&self) -> Vec<BookmarkEntry> {
        self.with_document(|doc| Ok(engine_extract::extract_bookmarks(doc)))
            .unwrap_or_default()
    }
}

impl Drop for PdfEngine {
    fn drop(&mut self) {
        match self.pdfium.lock() {
            Ok(_guard) => drop(self.doc.take()),
            Err(poisoned) => {
                tracing::warn!("Lock global do Pdfium estava corrompido ao fechar documento");
                let _guard = poisoned.into_inner();
                drop(self.doc.take());
            }
        }
    }
}

/// Página renderizada: bitmap RGBA pronto para egui::ColorImage
#[derive(Clone)]
pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{get_pdfium, PdfEngine};

    fn legacy_bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
        bgra.chunks_exact(4)
            .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
            .collect()
    }

    #[test]
    fn converts_bgra_pixels_to_rgba() {
        let source = [10, 20, 30, 40, 50, 60, 70, 80];
        assert_eq!(
            legacy_bgra_to_rgba(&source),
            vec![30, 20, 10, 40, 70, 60, 50, 80]
        );
    }

    #[test]
    fn ignores_incomplete_trailing_pixel() {
        assert_eq!(legacy_bgra_to_rgba(&[1, 2, 3]), Vec::<u8>::new());
    }

    #[test]
    #[ignore = "microbenchmark manual; execute em release com --nocapture"]
    fn rgba_handoff_benchmark() {
        const WIDTH: usize = 2560;
        const HEIGHT: usize = 1440;
        const ITERATIONS: u32 = 16;
        let source = vec![127_u8; WIDTH * HEIGHT * 4];

        let legacy_started = std::time::Instant::now();
        for _ in 0..ITERATIONS {
            // Simula a copia FFI + a segunda alocacao/varredura antiga.
            let raw = std::hint::black_box(source.clone());
            std::hint::black_box(legacy_bgra_to_rgba(&raw));
        }
        let legacy = legacy_started.elapsed();

        let direct_started = std::time::Instant::now();
        for _ in 0..ITERATIONS {
            // O novo caminho termina na unica copia feita por as_raw/as_rgba.
            std::hint::black_box(source.clone());
        }
        let direct = direct_started.elapsed();

        println!(
            "rgba_handoff: {ITERATIONS}x {WIDTH}x{HEIGHT}: antigo={legacy:?}, direto={direct:?}"
        );
        assert!(
            direct < legacy,
            "o handoff direto deve remover trabalho mensuravel"
        );
    }

    /// Stress manual contra um corpus real. Exemplo no Windows:
    /// `$env:MARIPDF_TEST_PDFS = "a.pdf`nb.pdf"; cargo test pdf_corpus -- --ignored`
    #[test]
    #[ignore = "requer MARIPDF_TEST_PDFS com ao menos dois arquivos"]
    fn pdf_corpus_opens_renders_and_closes_concurrently() {
        let raw = std::env::var_os("MARIPDF_TEST_PDFS")
            .expect("defina MARIPDF_TEST_PDFS com um caminho por linha");
        // Um caminho real pode conter `;`, que tambem e o separador de PATH no
        // Windows. Uma entrada por linha evita essa ambiguidade.
        let paths: Vec<std::path::PathBuf> = raw
            .to_string_lossy()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(std::path::PathBuf::from)
            .collect();
        assert!(paths.len() >= 2, "o stress requer ao menos dois PDFs");

        let workers: Vec<_> = paths
            .into_iter()
            .map(|path| {
                std::thread::spawn(move || {
                    for _ in 0..3 {
                        let engine = std::sync::Arc::new(
                            PdfEngine::open(&path, get_pdfium().unwrap())
                                .unwrap_or_else(|error| panic!("{}: {error}", path.display())),
                        );
                        assert!(
                            engine.page_count() > 0,
                            "{} nao tem paginas",
                            path.display()
                        );

                        let render_engine = std::sync::Arc::clone(&engine);
                        let render_path = path.clone();
                        let renderer = std::thread::spawn(move || {
                            render_engine.render_page(0, 0.15).unwrap_or_else(|error| {
                                panic!("{}: {error}", render_path.display())
                            });
                        });

                        let text_engine = std::sync::Arc::clone(&engine);
                        let text_path = path.clone();
                        let extractor = std::thread::spawn(move || {
                            text_engine
                                .extract_text(0)
                                .unwrap_or_else(|error| panic!("{}: {error}", text_path.display()));
                        });

                        let (width, height) = engine.page_size_pts(0);
                        assert!(width > 0.0 && height > 0.0);
                        renderer
                            .join()
                            .expect("render compartilhado entrou em panico");
                        extractor
                            .join()
                            .expect("extracao compartilhada entrou em panico");
                    }
                })
            })
            .collect();

        for worker in workers {
            worker.join().expect("worker do corpus entrou em panico");
        }
    }
}
