// pdf/text_search.rs — Busca full-text incremental pelo scheduler do documento.

use crate::pdf::document_worker::{
    DocumentJobOutput, DocumentJobTarget, DocumentWorkerHandle, JobCompletion, JobOutcome,
};
use std::sync::mpsc::{self, Receiver, Sender};

const CONTEXT_CHARS: usize = 60;

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub page_index: usize,
    #[allow(dead_code)]
    pub context: String,
    #[allow(dead_code)]
    pub match_start: usize,
    #[allow(dead_code)]
    pub match_end: usize,
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
    query_lower: String,
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
            query_lower: String::new(),
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
        self.query_lower = self.query.to_lowercase();
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
                    text,
                }) if output_page == page_index && output_generation == search_generation => {
                    let page_hits = find_hits_in_text(page_index, &text, &self.query_lower);
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

fn find_hits_in_text(page_index: usize, text: &str, query_lower: &str) -> Vec<SearchHit> {
    if query_lower.is_empty() {
        return Vec::new();
    }

    let normalized = text.to_lowercase();
    let mut hits = Vec::new();
    let mut search_from = 0;

    while let Some(relative_pos) = normalized[search_from..].find(query_lower) {
        let absolute_pos = search_from + relative_pos;
        let context_start =
            normalized.floor_char_boundary(absolute_pos.saturating_sub(CONTEXT_CHARS));
        let context_end = normalized.ceil_char_boundary(
            (absolute_pos + query_lower.len() + CONTEXT_CHARS).min(normalized.len()),
        );

        hits.push(SearchHit {
            page_index,
            context: normalized[context_start..context_end].to_string(),
            match_start: absolute_pos - context_start,
            match_end: absolute_pos - context_start + query_lower.len(),
        });

        search_from = absolute_pos + query_lower.len();
    }

    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::{
        document_worker::DocumentWorker,
        engine::{get_pdfium, PdfEngine},
    };
    use std::{path::PathBuf, sync::Arc, time::Duration};

    #[test]
    fn finds_multiple_case_insensitive_hits() {
        let hits = find_hits_in_text(7, "MariPDF é rápido. maripdf é leve.", "maripdf");
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|hit| hit.page_index == 7));
    }

    #[test]
    fn empty_query_has_no_hits() {
        assert!(find_hits_in_text(0, "conteúdo", "").is_empty());
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
