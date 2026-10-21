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
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use crate::ui::page_rotation::Rotation;

/// O PDF pede senha (ou a senha informada está errada).
#[derive(Debug)]
pub struct PasswordRequired;

impl std::fmt::Display for PasswordRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PDF protegido por senha (password required)")
    }
}

impl std::error::Error for PasswordRequired {}

/// Metadados exibidos em Propriedades do documento.
#[derive(Clone, Debug, Default)]
pub struct DocumentInfo {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
    pub created: String,
    pub modified: String,
    pub version: String,
}

use crate::pdf::extract as engine_extract;

/// Re-exporta tipos de extract.rs para manter compatibilidade com todos os sites de uso.
pub use crate::pdf::extract::{BookmarkEntry, PdfLinkInfo, SearchChar, SearchPage, TextCharInfo};

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
        .unwrap_or_else(|poisoned| poisoned.into_inner());

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

    // Compara o conteúdo inteiro (não só o tamanho): uma DLL truncada ou
    // trocada por outra do mesmo tamanho nunca deve ser carregada.
    let up_to_date = std::fs::read(&dll_path)
        .map(|existing| existing == PDFIUM_DLL_BYTES)
        .unwrap_or(false);

    if !up_to_date {
        tracing::info!("Extraindo pdfium.dll para {}", dll_path.display());
        crate::storage::write_atomic(&dll_path, PDFIUM_DLL_BYTES)
            .map_err(|e| anyhow::anyhow!("Falha ao extrair pdfium.dll: {e}"))?;
    }

    Ok(dll_path)
}

/// Limite de sanidade para uma página (PDF permite até 14.400 pt por lado;
/// UserUnit pode ampliar). Fora disso o render é recusado com erro claro.
const MAX_PAGE_SIDE_PTS: f32 = 200_000.0;

pub(crate) fn valid_page_size(width_pts: f32, height_pts: f32) -> bool {
    let ok = |v: f32| v.is_finite() && v > 0.0 && v <= MAX_PAGE_SIDE_PTS;
    ok(width_pts) && ok(height_pts)
}

/// Representa um documento PDF aberto e pronto para renderizar.
pub struct PdfEngine {
    /// Referência ao singleton do pdfium (mantida para garantir que a DLL
    /// não seja descarregada enquanto o doc ainda existe; também serializa o drop).
    pdfium: Arc<Mutex<Pdfium>>,
    doc: Option<PdfDocument<'static>>,
    page_count: usize,
    /// Rotação de exibição em quartos de volta (compartilhada com o worker).
    rotation: AtomicU8,
}

impl PdfEngine {
    /// Abre um arquivo PDF sem senha (testes).
    #[cfg(test)]
    pub fn open(path: &Path, pdfium: Arc<Mutex<Pdfium>>) -> anyhow::Result<Self> {
        Self::open_with_password(path, pdfium, None)
    }

    /// Abre um PDF, opcionalmente protegido. Falta de senha (ou senha errada)
    /// vira `PasswordRequired`, para a UI pedir a senha em vez de desistir.
    pub fn open_with_password(
        path: &Path,
        pdfium: Arc<Mutex<Pdfium>>,
        password: Option<&str>,
    ) -> anyhow::Result<Self> {
        if !path.is_file() {
            anyhow::bail!("Arquivo não encontrado (not found): {}", path.display());
        }
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Caminho com caracteres inválidos"))?;

        let (doc, page_count) = {
            // Um panic tratado em outro job envenena o mutex, mas o PDFium
            // segue válido; recusar aqui bloquearia abrir outro PDF para sempre.
            let pdfium_guard = pdfium
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            // SAFETY: lifetime 'static aqui e um contorno — o doc vive junto com
            // o Arc<Mutex<Pdfium>> mantido na struct. O Drop abaixo tambem fecha
            // o documento enquanto segura esse mesmo lock.
            let doc = unsafe {
                std::mem::transmute::<PdfDocument<'_>, PdfDocument<'static>>(
                    pdfium_guard
                        .load_pdf_from_file(path_str, password)
                        .map_err(|e| match e {
                            PdfiumError::PdfiumLibraryInternalError(
                                PdfiumInternalError::PasswordError,
                            ) => anyhow::Error::new(PasswordRequired),
                            other => anyhow::anyhow!("Falha ao abrir PDF: {other}"),
                        })?,
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
            rotation: AtomicU8::new(0),
        })
    }

    /// Rotação de exibição aplicada aos próximos renders.
    pub fn set_rotation(&self, rotation: Rotation) {
        self.rotation
            .store(rotation.quarter_turns(), Ordering::Release);
    }

    pub fn rotation(&self) -> Rotation {
        Rotation::from_quarter_turns(self.rotation.load(Ordering::Acquire))
    }

    /// Tamanho exibido da página (com a rotação aplicada), para ajustes de zoom.
    pub fn display_size_pts(&self, page_index: usize) -> (f32, f32) {
        self.rotation().display_size(self.page_size_pts(page_index))
    }

    /// Metadados do documento (campos ausentes ficam vazios).
    pub fn info(&self) -> DocumentInfo {
        self.with_document(|doc| {
            let tag = |kind| {
                doc.metadata()
                    .get(kind)
                    .map(|t| t.value().trim().to_string())
                    .unwrap_or_default()
            };
            Ok(DocumentInfo {
                title: tag(PdfDocumentMetadataTagType::Title),
                author: tag(PdfDocumentMetadataTagType::Author),
                subject: tag(PdfDocumentMetadataTagType::Subject),
                keywords: tag(PdfDocumentMetadataTagType::Keywords),
                creator: tag(PdfDocumentMetadataTagType::Creator),
                producer: tag(PdfDocumentMetadataTagType::Producer),
                created: pdf_date(&tag(PdfDocumentMetadataTagType::CreationDate)),
                modified: pdf_date(&tag(PdfDocumentMetadataTagType::ModificationDate)),
                version: pdf_version(doc.version()),
            })
        })
        .unwrap_or_default()
    }

    fn with_document<T>(
        &self,
        operation: impl FnOnce(&PdfDocument<'static>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        // Um panic em outro job envenena o mutex, mas o estado do PDFium segue
        // válido: recupera o lock em vez de inutilizar o documento inteiro.
        let _pdfium_guard = self
            .pdfium
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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

    /// Renderiza só a região `(x, y, w, h)` (em pixels da página inteira na
    /// `scale`) num bitmap do tamanho do tile. Spike da BF-301: a matriz do
    /// usuário é composta antes da escala, então a translação vai em pontos.
    #[cfg(test)]
    pub(crate) fn render_region(
        &self,
        page_index: usize,
        scale: f32,
        (x, y, w, h): (u32, u32, u32, u32),
    ) -> anyhow::Result<RenderedPage> {
        self.with_document(|doc| {
            let page = doc
                .pages()
                .get(page_index as u16)
                .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;
            let full_w = (page.width().value * scale) as i32;
            let full_h = (page.height().value * scale) as i32;
            let config = PdfRenderConfig::new()
                .set_target_width(full_w)
                .set_target_height(full_h)
                .set_reverse_byte_order(true)
                .translate(
                    PdfPoints::new(-(x as f32) / scale),
                    PdfPoints::new(-(y as f32) / scale),
                )
                .map_err(|e| anyhow::anyhow!("matriz do tile: {e}"))?;
            let mut bitmap =
                PdfBitmap::empty(w as i32, h as i32, PdfBitmapFormat::BGRA, page.bindings())
                    .map_err(|e| anyhow::anyhow!("bitmap do tile: {e}"))?;
            page.render_into_bitmap_with_config(&mut bitmap, &config)
                .map_err(|e| anyhow::anyhow!("render do tile: {e}"))?;
            Ok(RenderedPage {
                scale,
                width: w,
                height: h,
                rgba: bitmap.as_rgba_bytes(),
            })
        })
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
            if !valid_page_size(width_pts, height_pts) {
                anyhow::bail!(
                    "Página {} com dimensões inválidas ({width_pts}×{height_pts} pt)",
                    page_index + 1
                );
            }
            // Zoom alto × qualidade pode passar do limite de textura da GPU;
            // a página mantém o tamanho na tela e só perde nitidez.
            let scale = crate::ui::page_scale::capped_raster_scale(width_pts, height_pts, scale);

            // Converte pontos (72dpi) para pixels com o fator de escala
            let width_px = (width_pts * scale) as u32;
            let height_px = (height_pts * scale) as u32;

            let rotation = self.rotation();
            let mut render_cfg = PdfRenderConfig::new()
                .set_target_width(width_px as i32)
                .set_target_height(height_px as i32)
                // PDFium usa BGRA nativamente. Este flag pede a inversao no
                // proprio render e permite entregar RGBA sem um segundo buffer.
                .set_reverse_byte_order(true);
            // Só gira quando preciso: a matriz de rotação tira o PDFium do
            // caminho de render mais rápido.
            if rotation != Rotation::R0 {
                // Com `true`, largura e altura do alvo trocam junto.
                render_cfg = render_cfg.rotate(pdfium_rotation(rotation), true);
            }

            let bitmap = page
                .render_with_config(&render_cfg)
                .map_err(|e| anyhow::anyhow!("Falha ao renderizar página {page_index}: {e}"))?;

            // Com reverse_byte_order, `as_rgba_bytes()` devolve diretamente o
            // Vec copiado do bitmap, sem nova alocacao ou varredura por pixel.
            let rgba = bitmap.as_rgba_bytes();
            // Dimensões reais do bitmap: com rotação o PDFium recalcula o alvo
            // e o arredondamento pode diferir em 1 px do tamanho pedido. Um
            // RGBA que não bate com largura × altura derruba o upload.
            let (width, height) = (bitmap.width().max(0) as u32, bitmap.height().max(0) as u32);
            if rgba.len() != width as usize * height as usize * 4 {
                anyhow::bail!(
                    "Página {}: bitmap {width}×{height} com {} bytes",
                    page_index + 1,
                    rgba.len()
                );
            }

            Ok(RenderedPage {
                scale,
                width,
                height,
                rgba,
            })
        })
    }

    /// Extrai todos os chars de uma página com seus bounding boxes em pontos PDF.
    pub fn extract_chars_with_bounds(
        &self,
        page_index: usize,
    ) -> anyhow::Result<Vec<TextCharInfo>> {
        self.with_document(|doc| engine_extract::extract_chars_with_bounds(doc, page_index))
    }

    /// Extrai a camada de texto com geometria usada pela busca.
    pub fn extract_search_page(&self, page_index: usize) -> anyhow::Result<SearchPage> {
        self.with_document(|doc| engine_extract::extract_search_page(doc, page_index))
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

fn pdfium_rotation(rotation: Rotation) -> PdfPageRenderRotation {
    match rotation {
        Rotation::R0 => PdfPageRenderRotation::None,
        Rotation::R90 => PdfPageRenderRotation::Degrees90,
        Rotation::R180 => PdfPageRenderRotation::Degrees180,
        Rotation::R270 => PdfPageRenderRotation::Degrees270,
    }
}

fn pdf_version(version: PdfDocumentVersion) -> String {
    match version {
        PdfDocumentVersion::Unset => String::new(),
        PdfDocumentVersion::Pdf1_0 => "1.0".into(),
        PdfDocumentVersion::Pdf1_1 => "1.1".into(),
        PdfDocumentVersion::Pdf1_2 => "1.2".into(),
        PdfDocumentVersion::Pdf1_3 => "1.3".into(),
        PdfDocumentVersion::Pdf1_4 => "1.4".into(),
        PdfDocumentVersion::Pdf1_5 => "1.5".into(),
        PdfDocumentVersion::Pdf1_6 => "1.6".into(),
        PdfDocumentVersion::Pdf1_7 => "1.7".into(),
        PdfDocumentVersion::Pdf2_0 => "2.0".into(),
        PdfDocumentVersion::Other(n) => format!("{}.{}", n / 10, n % 10),
    }
}

/// `D:20240115103000+03'00'` → `15/01/2024 10:30`. Formatos estranhos voltam
/// como vieram.
pub(crate) fn pdf_date(raw: &str) -> String {
    let digits: String = raw
        .trim_start_matches("D:")
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.len() < 8 {
        return raw.to_string();
    }
    let part = |range: std::ops::Range<usize>| digits.get(range).unwrap_or("00");
    let date = format!("{}/{}/{}", part(6..8), part(4..6), part(0..4));
    if digits.len() >= 12 {
        format!("{date} {}:{}", part(8..10), part(10..12))
    } else {
        date
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
    /// Pixels por ponto PDF efetivamente usados (após o limite de textura).
    pub scale: f32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{get_pdfium, PdfEngine};

    fn legacy_bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
        bgra.as_chunks::<4>()
            .0
            .iter()
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
                                .extract_search_page(0)
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
