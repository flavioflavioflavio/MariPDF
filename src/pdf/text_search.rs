// pdf/text_search.rs — Busca full-text em PDFs via pdfium
//
// Usa pdfium-render para extrair texto de cada página e buscar por substring.
// Retorna lista de resultados (página + trecho) para exibição na UI.
//
// Não implementa highlight de bounding box nesta versão (Fase 2);
// o highlight visual (fase 3) exige FFI para fz_search_page do MuPDF.

use crate::pdf::engine::PdfEngine;

// ── Tipos públicos ────────────────────────────────────────────────────────────

/// Um resultado de busca: página onde foi encontrado + contexto ao redor.
#[derive(Clone, Debug)]
pub struct SearchHit {
    /// Índice da página (0-based).
    pub page_index: usize,
    /// Trecho de contexto em torno do match (até CONTEXT_CHARS caracteres).
    #[allow(dead_code)]
    pub context: String,
    /// Posição do match dentro de `context`.
    #[allow(dead_code)]
    pub match_start: usize,
    #[allow(dead_code)]
    pub match_end: usize,
}

/// Estado da busca: termo atual + resultados + posição ativa.
pub struct TextSearch {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub current_hit: usize,
    pub is_running: bool,
}

impl TextSearch {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            hits: Vec::new(),
            current_hit: 0,
            is_running: false,
        }
    }

    /// Limpa resultados e reseta estado.
    pub fn clear(&mut self) {
        self.hits.clear();
        self.current_hit = 0;
        self.is_running = false;
    }

    /// Resultado ativo (se houver).
    pub fn current(&self) -> Option<&SearchHit> {
        self.hits.get(self.current_hit)
    }

    pub fn go_next(&mut self) {
        if !self.hits.is_empty() {
            self.current_hit = (self.current_hit + 1) % self.hits.len();
        }
    }

    pub fn go_prev(&mut self) {
        if !self.hits.is_empty() {
            self.current_hit = self
                .current_hit
                .checked_sub(1)
                .unwrap_or(self.hits.len() - 1);
        }
    }
}

// ── Buscador síncrono ─────────────────────────────────────────────────────────

/// Quantos caracteres de contexto exibir ao redor de cada match.
const CONTEXT_CHARS: usize = 60;

/// Executa a busca em todas as páginas do documento.
///
/// Extrai o texto de cada página via pdfium e busca `query` (case-insensitive).
/// Indexação inteira é feita no main thread — para documentos grandes (>200 pág)
/// considerar mover para thread de background na Fase 3.
pub fn search_all_pages(engine: &PdfEngine, query: &str) -> Vec<SearchHit> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut hits = Vec::new();

    for idx in 0..engine.page_count() {
        let text = match engine.extract_text(idx) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("text_search: pg {idx}: {e}");
                continue;
            }
        };

        let text_lower = text.to_lowercase();
        let mut search_from = 0;

        while let Some(pos) = text_lower[search_from..].find(&query_lower) {
            let abs_pos = search_from + pos;

            // Calcula janela de contexto (em bytes, arredondada para char boundary)
            let ctx_start = text.floor_char_boundary(abs_pos.saturating_sub(CONTEXT_CHARS));
            let ctx_end =
                text.ceil_char_boundary((abs_pos + query.len() + CONTEXT_CHARS).min(text.len()));

            let context = text[ctx_start..ctx_end].to_string();
            let match_start = abs_pos - ctx_start;
            let match_end = match_start + query.len();

            hits.push(SearchHit {
                page_index: idx,
                context,
                match_start,
                match_end,
            });

            search_from = abs_pos + query_lower.len().max(1);
        }
    }

    hits
}
