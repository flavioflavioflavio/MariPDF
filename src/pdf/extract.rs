// pdf/extract.rs — Extração de conteúdo de páginas PDF
// Submodule de engine.rs (declarado como `pub mod extract`).
// Responsabilidade: texto, chars com bounds, links URI, e bookmarks (TOC).
// Structs de tipos públicos definidas aqui — engine.rs as re-exporta.

use pdfium_render::prelude::*;

// ── Tipos públicos ────────────────────────────────────────────────────────────

/// Um caractere com seu bounding box em pontos PDF (origem = canto inferior esquerdo).
#[derive(Clone, Debug)]
pub struct TextCharInfo {
    pub ch: char,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

/// Caractere da camada de texto usado pela busca. Mantém também os chars sem
/// bounding box (espaços e quebras gerados), para o texto ficar contínuo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchChar {
    pub ch: char,
    /// `[left, bottom, right, top]` em pontos PDF.
    pub bounds: Option<[f32; 4]>,
}

/// Camada de texto de uma página pronta para busca.
#[derive(Clone, Debug, Default)]
pub struct SearchPage {
    pub chars: Vec<SearchChar>,
    pub width_pts: f32,
    pub height_pts: f32,
}

/// Link encontrado numa página PDF: externo (URI) ou interno (página).
/// Coordenadas em pontos PDF (sistema pdfium: origem = canto inferior esquerdo).
#[derive(Debug, Clone)]
pub struct PdfLinkInfo {
    /// URI externa; vazia em links internos.
    pub uri: String,
    /// Página de destino de um link interno (GoTo).
    pub page: Option<usize>,
    /// Rect em pontos PDF: [left, bottom, right, top] (origem canto inf-esq)
    pub rect: [f32; 4],
}

/// Entrada do índice (Table-of-Contents)
#[derive(Clone, Debug)]
pub struct BookmarkEntry {
    pub title: String,
    /// Página de destino (0-based). `None` se o destino não é uma página.
    pub page: Option<usize>,
    /// Profundidade na hierarquia (0 = root, 1 = sub, …)
    pub depth: usize,
}

// ── Chars com bounding boxes ──────────────────────────────────────────────────

/// Extrai todos os chars de uma página com seus bounding boxes em pontos PDF.
/// Coordenadas: origem = canto inferior esquerdo da página (sistema pdfium).
pub fn extract_chars_with_bounds(
    doc: &PdfDocument<'static>,
    page_index: usize,
) -> anyhow::Result<Vec<TextCharInfo>> {
    let page = doc
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
            None => continue,
        };
        if c.is_control() && c != '\n' && c != '\r' {
            continue;
        }

        let bounds = match ch.tight_bounds() {
            Ok(r) => r,
            Err(_) => continue,
        };

        result.push(TextCharInfo {
            ch: c,
            left: bounds.left().value,
            bottom: bounds.bottom().value,
            right: bounds.right().value,
            top: bounds.top().value,
        });
    }

    Ok(result)
}

// ── Camada de texto para busca ────────────────────────────────────────────────

/// Extrai texto e geometria da página em uma única passada pelo PDFium.
pub fn extract_search_page(
    doc: &PdfDocument<'static>,
    page_index: usize,
) -> anyhow::Result<SearchPage> {
    let page = doc
        .pages()
        .get(page_index as u16)
        .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

    let text_page = page
        .text()
        .map_err(|e| anyhow::anyhow!("Falha ao obter texto da página {page_index}: {e}"))?;

    let chars = text_page
        .chars()
        .iter()
        .filter_map(|ch| {
            let c = ch.unicode_char()?;
            let bounds = ch.tight_bounds().ok().map(|r| {
                [
                    r.left().value,
                    r.bottom().value,
                    r.right().value,
                    r.top().value,
                ]
            });
            Some(SearchChar { ch: c, bounds })
        })
        .collect();

    Ok(SearchPage {
        chars,
        width_pts: page.width().value,
        height_pts: page.height().value,
    })
}

// ── Links ─────────────────────────────────────────────────────────────────────

/// Extrai os links (externos e internos) de uma página.
pub fn extract_page_links(doc: &PdfDocument<'static>, page_index: usize) -> Vec<PdfLinkInfo> {
    let page = match doc.pages().get(page_index as u16) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut links = Vec::new();

    for link in page.links().iter() {
        // Destino direto, ação GoTo local ou URI externa; o resto é ignorado.
        let direct = link.destination().and_then(|d| d.page_index().ok());
        let (uri, page) = match (direct, link.action()) {
            (Some(index), _) => (String::new(), Some(index as usize)),
            (None, Some(action)) => {
                if let Some(local) = action.as_local_destination_action() {
                    match local.destination().and_then(|d| d.page_index()) {
                        Ok(index) => (String::new(), Some(index as usize)),
                        Err(_) => continue,
                    }
                } else if let Some(Ok(uri)) = action.as_uri_action().map(|ua| ua.uri()) {
                    (uri, None)
                } else {
                    continue;
                }
            }
            (None, None) => continue,
        };

        let rect = match link.rect() {
            Ok(r) => [
                r.left().value,
                r.bottom().value,
                r.right().value,
                r.top().value,
            ],
            Err(_) => continue,
        };

        links.push(PdfLinkInfo { uri, page, rect });
    }

    links
}

// ── Bookmarks / TOC ───────────────────────────────────────────────────────────

/// Extrai a árvore de bookmarks (Table-of-Contents) do documento.
pub fn extract_bookmarks(doc: &PdfDocument<'static>) -> Vec<BookmarkEntry> {
    let mut result = Vec::new();
    let bookmarks = doc.bookmarks();
    if let Some(root) = bookmarks.root() {
        collect_bookmarks(&root, 0, &mut result);
    }
    result
}

fn collect_bookmarks(node: &PdfBookmark<'_>, depth: usize, out: &mut Vec<BookmarkEntry>) {
    let title = node.title().unwrap_or_default();
    let page = node
        .destination()
        .and_then(|d| d.page_index().ok())
        .map(|idx| idx as usize);

    out.push(BookmarkEntry { title, page, depth });

    let mut child = node.first_child();
    while let Some(c) = child {
        collect_bookmarks(&c, depth + 1, out);
        child = c.next_sibling();
    }
}
