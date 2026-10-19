// pdf/text_search.rs — Busca full-text incremental pelo scheduler do documento.

use crate::pdf::document_worker::{
    DocumentJobOutput, DocumentJobTarget, DocumentWorkerHandle, JobCompletion, JobOutcome,
};
use crate::pdf::engine::{SearchChar, SearchPage};
use std::sync::mpsc::{self, Receiver, Sender};

/// Caracteres de contexto exibidos antes e depois da ocorrência.
const CONTEXT_CHARS: usize = 40;

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub page_index: usize,
    /// Trecho com a caixa original e espaços normalizados.
    pub context: String,
    /// Intervalo em bytes da ocorrência dentro de `context`.
    pub match_start: usize,
    pub match_end: usize,
    /// Retângulos `[left, bottom, right, top]` em pontos PDF, um por linha.
    pub rects: Vec<[f32; 4]>,
    /// Tamanho da página em pontos PDF, para mapear `rects` para a tela.
    pub page_size: (f32, f32),
}

pub struct TextSearch {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub current_hit: usize,
    pub is_running: bool,
    scheduler: Option<DocumentWorkerHandle>,
    result_tx: Sender<JobCompletion>,
    result_rx: Receiver<JobCompletion>,
    page_count: usize,
    next_page: usize,
    generation: u64,
    query_folded: Vec<char>,
    inflight: bool,
}

impl TextSearch {
    pub fn new() -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        Self {
            query: String::new(),
            hits: Vec::new(),
            current_hit: 0,
            is_running: false,
            scheduler: None,
            result_tx,
            result_rx,
            page_count: 0,
            next_page: 0,
            generation: 0,
            query_folded: Vec::new(),
            inflight: false,
        }
    }

    pub fn attach_document(&mut self, scheduler: DocumentWorkerHandle, page_count: usize) {
        self.detach_document();
        self.scheduler = Some(scheduler);
        self.page_count = page_count;
    }

    pub fn detach_document(&mut self) {
        self.clear();
        self.scheduler = None;
        self.page_count = 0;
    }

    /// Inicia uma busca nova. Uma página por vez entra na fila global com a
    /// menor prioridade, permitindo que viewport e interação sempre a ultrapassem.
    pub fn start(&mut self, query: &str) {
        self.query = query.trim().to_string();
        self.query_folded = normalize_query(&self.query);
        self.hits.clear();
        self.current_hit = 0;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.next_page = 0;
        self.inflight = false;
        self.is_running = !self.query.is_empty() && self.scheduler.is_some() && self.page_count > 0;
        self.schedule_next();
    }

    /// Consome progresso sem bloquear. Retorna true quando o primeiro lote de
    /// resultados da busca atual ficou disponível.
    pub fn poll(&mut self) -> bool {
        let mut first_hits_arrived = false;

        while let Ok(completion) = self.result_rx.try_recv() {
            let DocumentJobTarget::SearchText {
                page_index,
                search_generation,
            } = completion.target
            else {
                continue;
            };
            if search_generation != self.generation {
                continue;
            }
            self.inflight = false;

            match completion.outcome {
                JobOutcome::Ready(DocumentJobOutput::SearchText {
                    page_index: output_page,
                    search_generation: output_generation,
                    page,
                }) if output_page == page_index && output_generation == search_generation => {
                    let page_hits = find_hits_in_page(page_index, &page, &self.query_folded);
                    if self.hits.is_empty() && !page_hits.is_empty() {
                        first_hits_arrived = true;
                    }
                    self.hits.extend(page_hits);
                }
                JobOutcome::Failed(error) => {
                    tracing::warn!("text_search: pg {page_index}: {error}");
                }
                JobOutcome::Cancelled => {}
                JobOutcome::Ready(_) => {
                    tracing::warn!("text_search: resultado de tipo inesperado");
                }
            }
        }

        self.schedule_next();
        first_hits_arrived
    }

    fn schedule_next(&mut self) {
        if !self.is_running || self.inflight {
            return;
        }
        if self.next_page >= self.page_count {
            self.is_running = false;
            return;
        }

        let page_index = self.next_page;
        let accepted = self.scheduler.as_ref().is_some_and(|scheduler| {
            scheduler.request_search_text(page_index, self.generation, self.result_tx.clone())
        });
        if accepted {
            self.next_page += 1;
            self.inflight = true;
        }
    }

    pub fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.hits.clear();
        self.current_hit = 0;
        self.next_page = 0;
        self.inflight = false;
        self.is_running = false;
    }

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

/// Minúscula 1:1 por caractere; mantém o índice estável entre texto e geometria.
fn fold_char(c: char) -> char {
    if c.is_whitespace() {
        return ' ';
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

fn normalize_query(query: &str) -> Vec<char> {
    let mut folded = Vec::with_capacity(query.len());
    for c in query.trim().chars().map(fold_char) {
        if c == ' ' && folded.last() == Some(&' ') {
            continue;
        }
        folded.push(c);
    }
    folded
}

/// Texto da página com espaços colapsados, mantendo o índice do char original.
struct NormalizedPage {
    folded: Vec<char>,
    display: Vec<char>,
    source: Vec<usize>,
}

fn normalize_page(chars: &[SearchChar]) -> NormalizedPage {
    let mut page = NormalizedPage {
        folded: Vec::with_capacity(chars.len()),
        display: Vec::with_capacity(chars.len()),
        source: Vec::with_capacity(chars.len()),
    };
    for (index, sc) in chars.iter().enumerate() {
        let is_space = sc.ch.is_whitespace() || sc.ch.is_control();
        if is_space && page.folded.last().is_none_or(|last| *last == ' ') {
            continue;
        }
        let (folded, display) = if is_space {
            (' ', ' ')
        } else {
            (fold_char(sc.ch), sc.ch)
        };
        page.folded.push(folded);
        page.display.push(display);
        page.source.push(index);
    }
    page
}

fn find_hits_in_page(page_index: usize, page: &SearchPage, query: &[char]) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let normalized = normalize_page(&page.chars);
    let text = &normalized.folded;
    let mut hits = Vec::new();
    let mut start = 0;

    while start + query.len() <= text.len() {
        if text[start..start + query.len()] != *query {
            start += 1;
            continue;
        }
        let end = start + query.len();
        let (context, match_start, match_end) = build_context(&normalized.display, start, end);
        let match_chars = normalized.source[start..end]
            .iter()
            .map(|&source| page.chars[source]);
        hits.push(SearchHit {
            page_index,
            context,
            match_start,
            match_end,
            rects: line_rects(match_chars),
            page_size: (page.width_pts, page.height_pts),
        });
        start = end;
    }

    hits
}

fn build_context(display: &[char], start: usize, end: usize) -> (String, usize, usize) {
    let context_start = start.saturating_sub(CONTEXT_CHARS);
    let context_end = (end + CONTEXT_CHARS).min(display.len());

    let mut context = String::new();
    if context_start > 0 {
        context.push('…');
    }
    let before: String = display[context_start..start].iter().collect();
    context.push_str(before.trim_start());
    let match_start = context.len();
    context.extend(&display[start..end]);
    let match_end = context.len();
    let after: String = display[end..context_end].iter().collect();
    context.push_str(after.trim_end());
    if context_end < display.len() {
        context.push('…');
    }
    (context, match_start, match_end)
}

/// Une as caixas dos chars de uma ocorrência em um retângulo por linha.
fn line_rects(chars: impl Iterator<Item = SearchChar>) -> Vec<[f32; 4]> {
    let mut rects: Vec<[f32; 4]> = Vec::new();
    for bounds in chars.filter_map(|sc| sc.bounds) {
        let [left, bottom, right, top] = bounds;
        if right <= left || top <= bottom {
            continue;
        }
        let center = (bottom + top) * 0.5;
        match rects.last_mut() {
            Some(line) if center >= line[1] && center <= line[3] => {
                line[0] = line[0].min(left);
                line[1] = line[1].min(bottom);
                line[2] = line[2].max(right);
                line[3] = line[3].max(top);
            }
            _ => rects.push(bounds),
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::{
        document_worker::DocumentWorker,
        engine::{get_pdfium, PdfEngine},
    };
    use std::{path::PathBuf, sync::Arc, time::Duration};

    /// Página sintética: cada char ocupa 10pt de largura numa linha de 12pt.
    fn page(text: &str) -> SearchPage {
        let mut line = 0.0;
        let mut x = 0.0;
        let chars = text
            .chars()
            .map(|ch| {
                if ch == '\n' {
                    line += 1.0;
                    x = 0.0;
                    return SearchChar { ch, bounds: None };
                }
                let bottom = 800.0 - line * 20.0;
                let bounds = (!ch.is_whitespace()).then_some([x, bottom, x + 10.0, bottom + 12.0]);
                x += 10.0;
                SearchChar { ch, bounds }
            })
            .collect();
        SearchPage {
            chars,
            width_pts: 595.0,
            height_pts: 842.0,
        }
    }

    fn hits(text: &str, query: &str) -> Vec<SearchHit> {
        find_hits_in_page(3, &page(text), &normalize_query(query))
    }

    #[test]
    fn finds_multiple_case_insensitive_hits() {
        let found = hits("MariPDF é rápido. maripdf é leve.", "MARIPDF");
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|hit| hit.page_index == 3));
        assert_eq!(found[0].page_size, (595.0, 842.0));
    }

    #[test]
    fn empty_query_has_no_hits() {
        assert!(hits("conteúdo", "   ").is_empty());
    }

    #[test]
    fn accented_text_matches_without_shifting_geometry() {
        let found = hits("İx AÇÃO final", "ação");
        assert_eq!(found.len(), 1);
        // "AÇÃO" começa no 4º char (x = 30pt) e tem 4 chars de largura.
        assert_eq!(found[0].rects, vec![[30.0, 800.0, 70.0, 812.0]]);
    }

    #[test]
    fn phrase_matches_across_line_break_with_one_rect_per_line() {
        let found = hits("leitor de\r\nPDF rápido", "de pdf");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rects.len(), 2);
        let [_, first_bottom, ..] = found[0].rects[0];
        let [_, second_bottom, ..] = found[0].rects[1];
        assert!(second_bottom < first_bottom);
    }

    #[test]
    fn context_keeps_original_case_and_marks_the_match() {
        let found = hits("Antes   do Termo depois", "termo");
        let hit = &found[0];
        assert_eq!(hit.context, "Antes do Termo depois");
        assert_eq!(&hit.context[hit.match_start..hit.match_end], "Termo");
    }

    #[test]
    fn long_context_is_truncated_with_ellipsis() {
        let text = format!("{} alvo {}", "a".repeat(100), "b".repeat(100));
        let hit = &hits(&text, "alvo")[0];
        assert!(hit.context.starts_with('…') && hit.context.ends_with('…'));
        assert_eq!(&hit.context[hit.match_start..hit.match_end], "alvo");
    }

    #[test]
    fn new_search_is_detached_and_idle() {
        let search = TextSearch::new();

        assert!(search.scheduler.is_none());
        assert_eq!(search.page_count, 0);
        assert!(!search.inflight);
        assert!(!search.is_running);
    }

    #[test]
    #[ignore = "requer MARIPDF_TEST_PDFS e MARIPDF_TEST_QUERY presente na 1ª página"]
    fn real_pdf_hits_carry_geometry_inside_the_page() {
        let raw = std::env::var("MARIPDF_TEST_PDFS").expect("defina MARIPDF_TEST_PDFS");
        let query = std::env::var("MARIPDF_TEST_QUERY").expect("defina MARIPDF_TEST_QUERY");
        let path = PathBuf::from(
            raw.lines()
                .find(|line| !line.trim().is_empty())
                .expect("inclua ao menos um PDF"),
        );
        let engine = PdfEngine::open(&path, get_pdfium().unwrap()).unwrap();
        let page = engine.extract_search_page(0).unwrap();
        let found = find_hits_in_page(0, &page, &normalize_query(&query));

        assert!(!found.is_empty(), "'{query}' não encontrado na 1ª página");
        for hit in &found {
            let (width, height) = hit.page_size;
            assert!(!hit.rects.is_empty());
            assert!(hit.rects.iter().all(|[l, b, r, t]| {
                *l >= 0.0 && *b >= 0.0 && r <= &width && t <= &height && l < r && b < t
            }));
            let matched = &hit.context[hit.match_start..hit.match_end];
            assert_eq!(matched.to_lowercase(), query.to_lowercase());
        }
    }

    #[test]
    #[ignore = "requer MARIPDF_TEST_PDFS com um caminho por linha"]
    fn real_pdf_search_uses_scheduler_and_finishes() {
        let raw = std::env::var("MARIPDF_TEST_PDFS").expect("defina MARIPDF_TEST_PDFS");
        let path = PathBuf::from(
            raw.lines()
                .find(|line| !line.trim().is_empty())
                .expect("inclua ao menos um PDF"),
        );
        let engine = Arc::new(PdfEngine::open(&path, get_pdfium().unwrap()).unwrap());
        let worker = DocumentWorker::new(engine);
        let mut search = TextSearch::new();
        search.attach_document(worker.handle(), 1);
        search.start("espiritismo");

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while search.is_running && std::time::Instant::now() < deadline {
            search.poll();
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(!search.is_running);
        assert_eq!(worker.metrics().completed, 1);
    }
}
