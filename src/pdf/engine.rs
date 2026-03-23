// pdf/engine.rs — Wrapper sobre pdfium-render
//
// Engine de renderização de PDFs usando o pdfium do Chromium.
// Responsabilidade: abrir documentos, contar páginas, renderizar RGBA.
//
// ADR-002: pdfium na Fase 1 (binários prebuilt, zero build time);
//          migração para MuPDF nativo quando mupdf-sys suportar MSVC 2022.
//
// DESIGN NOTE (singleton Pdfium):
//   A DLL do pdfium é carregada uma única vez via OnceLock<Arc<Pdfium>>.
//   Isso evita o deadlock que ocorria quando o PdfEngine do main thread
//   era dropado (descarregando a DLL) enquanto a thread de background
//   ainda usava sua própria instância da mesma DLL.
//   A DLL permanece carregada durante toda a vida do processo — igual ao
//   comportamento do Chrome/Firefox com suas próprias instâncias do pdfium.

use std::{
    path::Path,
    sync::{Arc, OnceLock},
};
use pdfium_render::prelude::*;

/// Um caractere com seu bounding box em pontos PDF (origem = canto inferior esquerdo).
#[derive(Clone, Debug)]
pub struct TextCharInfo {
    pub ch:     char,
    pub left:   f32,
    pub bottom: f32,
    pub right:  f32,
    pub top:    f32,
}

/// Informação sobre um link URI encontrado numa página PDF.
/// Coordenadas em pontos PDF (sistema pdfium: origem = canto inferior esquerdo).
#[derive(Debug, Clone)]
pub struct PdfLinkInfo {
    pub uri:  String,
    #[allow(dead_code)]
    /// Rect em pontos PDF: [left, bottom, right, top] (origem canto inf-esq)
    pub rect: [f32; 4],
}

/// DLL do pdfium embutida no executável em compile-time.
/// Em runtime, é extraída para %TEMP%\maripdf\ e carregada de lá.
/// Isso torna o executável completamente portátil (single-file).
static PDFIUM_DLL_BYTES: &[u8] = include_bytes!("../../assets/pdfium.dll");

/// Singleton global do Pdfium — carregado uma vez, nunca descarregado.
/// Compartilhado entre main thread e threads de background via Arc.
static PDFIUM_INSTANCE: OnceLock<Arc<Pdfium>> = OnceLock::new();

/// Retorna (ou inicializa) o singleton global do Pdfium.
/// Thread-safe via OnceLock — apenas um thread inicializa, os demais esperam.
pub fn get_pdfium() -> anyhow::Result<Arc<Pdfium>> {
    if let Some(arc) = PDFIUM_INSTANCE.get() {
        return Ok(Arc::clone(arc));
    }

    // Primeira chamada: extrai e carrega a DLL
    let pdfium = make_pdfium()?;
    let arc = Arc::new(pdfium);
    // Se outra thread já inicializou enquanto carregávamos, usa a dela
    match PDFIUM_INSTANCE.set(Arc::clone(&arc)) {
        Ok(_)  => Ok(arc),
        Err(_) => Ok(Arc::clone(PDFIUM_INSTANCE.get().unwrap())),
    }
}

/// Cria e retorna uma instância do pdfium (carrega a DLL).
/// Prefira `get_pdfium()` para reutilizar o singleton.
pub fn make_pdfium() -> anyhow::Result<Pdfium> {
    let dll_path = extract_pdfium_dll()?;
    let dll_dir  = dll_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Caminho da DLL inválido"))?;

    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path(
            dll_dir.to_str().ok_or_else(|| anyhow::anyhow!("Caminho com caracteres inválidos"))?
        )
    )
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
        Err(_)   => true,
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
    pdfium:     Arc<Pdfium>,
    doc:        PdfDocument<'static>,
    page_count: usize,
}

impl PdfEngine {
    /// Abre um arquivo PDF usando pdfium.
    pub fn open(path: &Path, pdfium: Arc<Pdfium>) -> anyhow::Result<Self> {
        let path_str = path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Caminho com caracteres inválidos"))?;

        // SAFETY: lifetime 'static aqui é um contorno — o doc vive junto com pdfium
        // Ambos são mantidos juntos na struct PdfEngine
        let doc = unsafe {
            std::mem::transmute::<PdfDocument<'_>, PdfDocument<'static>>(
                pdfium
                    .load_pdf_from_file(path_str, None)
                    .map_err(|e| anyhow::anyhow!("Falha ao abrir PDF: {e}"))?
            )
        };

        let page_count = doc.pages().len();

        tracing::info!(
            "PDF aberto: {} ({} páginas)",
            path.file_name().unwrap_or_default().to_string_lossy(),
            page_count
        );

        Ok(Self { pdfium, doc, page_count: page_count.into() })
    }

    /// Número total de páginas do documento.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Dimensões da página em pontos PDF (largura, altura).
    /// Necessário para converter coordenadas de tela ↔ coordenadas PDF.
    pub fn page_size_pts(&self, page_index: usize) -> (f32, f32) {
        self.doc
            .pages()
            .get(page_index as u16)
            .map(|p| (p.width().value, p.height().value))
            .unwrap_or((595.0, 842.0)) // A4 como fallback
    }

    /// Renderiza uma página como bitmap RGBA.
    ///
    /// # Parâmetros
    /// - `page_index`: índice da página (0-based)
    /// - `scale`: fator de escala (1.0 = 72dpi, 2.0 = 144dpi/HD)
    pub fn render_page(&self, page_index: usize, scale: f32) -> anyhow::Result<RenderedPage> {
        let page = self.doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

        let width_pts  = page.width().value;
        let height_pts = page.height().value;

        // Converte pontos (72dpi) para pixels com o fator de escala
        let width_px  = (width_pts  * scale) as u32;
        let height_px = (height_pts * scale) as u32;

        let render_cfg = PdfRenderConfig::new()
            .set_target_width(width_px as i32)
            .set_target_height(height_px as i32);

        let bitmap = page
            .render_with_config(&render_cfg)
            .map_err(|e| anyhow::anyhow!("Falha ao renderizar página {page_index}: {e}"))?;

        // pdfium devolve BGRA — converte para RGBA para o egui
        let rgba = bgra_to_rgba(&bitmap.as_raw_bytes());

        Ok(RenderedPage {
            width:  width_px,
            height: height_px,
            rgba,
        })
    }

    /// Extrai o texto de uma página como String.
    /// Usado pela busca full-text (text_search.rs).
    pub fn extract_text(&self, page_index: usize) -> anyhow::Result<String> {
        let page = self.doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

        let text = page
            .text()
            .map_err(|e| anyhow::anyhow!("Falha ao extrair texto da página {page_index}: {e}"))?
            .all();

        Ok(text)
    }

    /// Extrai todos os chars de uma página com seus bounding boxes em pontos PDF.
    /// Coordenadas: origem = canto inferior esquerdo da página (sistema pdfium).
    /// Retorno: Vec de (char, [left, bottom, right, top]) em pontos.
    pub fn extract_chars_with_bounds(&self, page_index: usize) -> anyhow::Result<Vec<TextCharInfo>> {
        let page = self.doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

        let text_page = page
            .text()
            .map_err(|e| anyhow::anyhow!("Falha ao obter texto da página {page_index}: {e}"))?;

        let chars = text_page.chars();
        let mut result = Vec::with_capacity(chars.len());

        for ch in chars.iter() {
            let c = match ch.unicode_char() {
                Some(c) => c,
                None    => continue,
            };
            // Ignora control chars (exceto espaços normais)
            if c.is_control() && c != '\n' && c != '\r' { continue; }

            let bounds = match ch.tight_bounds() {
                Ok(r) => r,
                Err(_) => continue,
            };

            result.push(TextCharInfo {
                ch: c,
                left:   bounds.left().value,
                bottom: bounds.bottom().value,
                right:  bounds.right().value,
                top:    bounds.top().value,
            });
        }

        Ok(result)
    }

    /// Extrai todos os links URI de uma página.
    /// Retorna rect em pontos PDF (origem = canto inferior esquerdo da página).
    pub fn extract_page_links(&self, page_index: usize) -> Vec<PdfLinkInfo> {
        let page = match self.doc.pages().get(page_index as u16) {
            Ok(p)  => p,
            Err(_) => return Vec::new(),
        };

        let mut links = Vec::new();

        for link in page.links().iter() {
            // Só links com URI válida
            let uri = match link.action() {
                Some(action) => match action.as_uri_action() {
                    Some(ua) => match ua.uri() {
                        Ok(u)  => u,
                        Err(_) => continue,
                    },
                    None => continue,
                },
                None => continue,
            };

            // rect() em pontos PDF: left, bottom, right, top (origem inf-esq)
            let rect = match link.rect() {
                Ok(r)  => [r.left().value, r.bottom().value, r.right().value, r.top().value],
                Err(_) => continue,
            };

            links.push(PdfLinkInfo { uri, rect });
        }

        links
    }

    /// Extrai a árvore de bookmarks (Table-of-Contents) do documento.
    /// Retorna lista plana ordenada por profundidade, pronta para renderizar.
    pub fn extract_bookmarks(&self) -> Vec<BookmarkEntry> {
        let mut result = Vec::new();
        let bookmarks = self.doc.bookmarks();
        // root() retorna Option<PdfBookmark>
        if let Some(root) = bookmarks.root() {
            collect_bookmarks(&root, 0, &mut result);
        }
        result
    }
}

/// Entrada do índice (Table-of-Contents)
#[derive(Clone, Debug)]
pub struct BookmarkEntry {
    pub title: String,
    /// Página de destino (0-based). `None` se o destino não é uma página.
    pub page:  Option<usize>,
    /// Profundidade na hierarquia (0 = root, 1 = sub, …)
    pub depth: usize,
}

/// Percorre a árvore de bookmarks recursivamente e preenche `out`.
fn collect_bookmarks(
    node:  &pdfium_render::prelude::PdfBookmark<'_>,
    depth: usize,
    out:   &mut Vec<BookmarkEntry>,
) {
    let title = node.title().unwrap_or_default();

    // destination() já está no PdfBookmark diretamente; page_index() retorna Result<u16>
    let page = node.destination()
        .and_then(|d| d.page_index().ok())
        .map(|idx| idx as usize);

    out.push(BookmarkEntry { title, page, depth });

    // Percorre filhos recursivamente
    let mut child = node.first_child();
    while let Some(c) = child {
        collect_bookmarks(&c, depth + 1, out);
        child = c.next_sibling();
    }
}

/// Página renderizada: bitmap RGBA pronto para egui::ColorImage
#[derive(Clone)]
pub struct RenderedPage {
    pub width:  u32,
    pub height: u32,
    pub rgba:   Vec<u8>,
}

/// Converte buffer BGRA (pdfium) para RGBA (egui)
fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for pixel in bgra.chunks_exact(4) {
        rgba.push(pixel[2]); // R (B<->R swap)
        rgba.push(pixel[1]); // G
        rgba.push(pixel[0]); // B
        rgba.push(pixel[3]); // A
    }
    rgba
}
