// pdf/extract.rs — Extração de conteúdo de páginas PDF
// Submodule de engine.rs (declarado como `pub mod extract`).
// Responsabilidade: texto, chars com bounds, links URI, e bookmarks (TOC).
// Structs de tipos públicos definidas aqui — engine.rs as re-exporta.

use pdfium_render::prelude::*;

// ── Tipos públicos ────────────────────────────────────────────────────────────

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

/// Entrada do índice (Table-of-Contents)
#[derive(Clone, Debug)]
pub struct BookmarkEntry {
    pub title: String,
    /// Página de destino (0-based). `None` se o destino não é uma página.
    pub page:  Option<usize>,
    /// Profundidade na hierarquia (0 = root, 1 = sub, …)
    pub depth: usize,
}

// ── Texto bruto ───────────────────────────────────────────────────────────────

/// Extrai o texto de uma página como String.
/// Usado pela busca full-text (text_search.rs).
pub fn extract_text(doc: &PdfDocument<'static>, page_index: usize) -> anyhow::Result<String> {
    let page = doc
        .pages()
        .get(page_index as u16)
        .map_err(|e| anyhow::anyhow!("Falha ao carregar página {page_index}: {e}"))?;

    let text = page
        .text()
        .map_err(|e| anyhow::anyhow!("Falha ao extrair texto da página {page_index}: {e}"))?
        .all();

    Ok(text)
}

// ── Chars com bounding boxes ──────────────────────────────────────────────────

/// Extrai todos os chars de uma página com seus bounding boxes em pontos PDF.
/// Coordenadas: origem = canto inferior esquerdo da página (sistema pdfium).
pub fn extract_chars_with_bounds(
    doc:        &PdfDocument<'static>,
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
            None    => continue,
        };
        if c.is_control() && c != '\n' && c != '\r' { continue; }

        let bounds = match ch.tight_bounds() {
            Ok(r)  => r,
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

// ── Links URI ─────────────────────────────────────────────────────────────────

/// Extrai todos os links URI de uma página.
pub fn extract_page_links(doc: &PdfDocument<'static>, page_index: usize) -> Vec<PdfLinkInfo> {
    let page = match doc.pages().get(page_index as u16) {
        Ok(p)  => p,
        Err(_) => return Vec::new(),
    };

    let mut links = Vec::new();

    for link in page.links().iter() {
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

        let rect = match link.rect() {
            Ok(r)  => [r.left().value, r.bottom().value, r.right().value, r.top().value],
            Err(_) => continue,
        };

        links.push(PdfLinkInfo { uri, rect });
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

fn collect_bookmarks(
    node:  &PdfBookmark<'_>,
    depth: usize,
    out:   &mut Vec<BookmarkEntry>,
) {
    let title = node.title().unwrap_or_default();
    let page = node.destination()
        .and_then(|d| d.page_index().ok())
        .map(|idx| idx as usize);

    out.push(BookmarkEntry { title, page, depth });

    let mut child = node.first_child();
    while let Some(c) = child {
        collect_bookmarks(&c, depth + 1, out);
        child = c.next_sibling();
    }
}
