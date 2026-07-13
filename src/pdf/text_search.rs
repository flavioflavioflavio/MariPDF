// pdf/text_search.rs — Busca full-text incremental em worker dedicado.

use crate::pdf::engine::{get_pdfium, PdfEngine};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};

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

struct SearchRequest {
    generation: u64,
    query: String,
}

struct SearchProgress {
    generation: u64,
    hits: Vec<SearchHit>,
    finished: bool,
}

pub struct TextSearch {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub current_hit: usize,
    pub is_running: bool,
    request_tx: Option<Sender<SearchRequest>>,
    progress_rx: Option<Receiver<SearchProgress>>,
    document_path: Option<PathBuf>,
    generation: Arc<AtomicU64>,
}

impl TextSearch {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            hits: Vec::new(),
            current_hit: 0,
            is_running: false,
            request_tx: None,
            progress_rx: None,
            document_path: None,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Vincula a busca a um documento sem abrir uma segunda instancia do PDF.
    /// O worker so nasce quando a primeira busca for iniciada.
    pub fn attach_document(&mut self, path: PathBuf) {
        self.detach_document();
        self.document_path = Some(path);
    }

    fn ensure_worker(&mut self) -> bool {
        if self.request_tx.is_some() {
            return true;
        }

        let Some(path) = self.document_path.clone() else {
            return false;
        };
        let (request_tx, request_rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        let generation = Arc::clone(&self.generation);

        std::thread::Builder::new()
            .name("maripdf-search".into())
            .spawn(move || search_worker(path, request_rx, progress_tx, generation))
            .expect("Falha ao criar thread maripdf-search");

        self.request_tx = Some(request_tx);
        self.progress_rx = Some(progress_rx);
        true
    }

    pub fn detach_document(&mut self) {
        self.clear();
        self.request_tx = None;
        self.progress_rx = None;
        self.document_path = None;
    }

    /// Inicia uma busca nova e invalida imediatamente resultados anteriores.
    pub fn start(&mut self, query: &str) {
        let query = query.trim().to_string();
        self.query = query.clone();
        self.hits.clear();
        self.current_hit = 0;

        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        if query.is_empty() {
            self.is_running = false;
            return;
        }

        if !self.ensure_worker() {
            self.is_running = false;
            return;
        }

        self.is_running = self
            .request_tx
            .as_ref()
            .is_some_and(|tx| tx.send(SearchRequest { generation, query }).is_ok());
    }

    /// Consome progresso sem bloquear. Retorna true quando o primeiro lote de
    /// resultados da busca atual ficou disponível.
    pub fn poll(&mut self) -> bool {
        let current_generation = self.generation.load(Ordering::Acquire);
        let mut first_hits_arrived = false;

        if let Some(rx) = &self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                if progress.generation != current_generation {
                    continue;
                }
                if self.hits.is_empty() && !progress.hits.is_empty() {
                    first_hits_arrived = true;
                }
                self.hits.extend(progress.hits);
                if progress.finished {
                    self.is_running = false;
                }
            }
        }

        first_hits_arrived
    }

    pub fn clear(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.hits.clear();
        self.current_hit = 0;
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

fn search_worker(
    path: PathBuf,
    request_rx: Receiver<SearchRequest>,
    progress_tx: Sender<SearchProgress>,
    generation: Arc<AtomicU64>,
) {
    let engine = get_pdfium()
        .and_then(|pdfium| PdfEngine::open(&path, pdfium))
        .map_err(|error| tracing::error!("text_search: falha ao abrir documento: {error}"));

    let Ok(engine) = engine else {
        return;
    };

    for request in request_rx {
        if generation.load(Ordering::Acquire) != request.generation {
            continue;
        }

        let query_lower = request.query.to_lowercase();
        for page_index in 0..engine.page_count() {
            if generation.load(Ordering::Acquire) != request.generation {
                break;
            }

            match engine.extract_text(page_index) {
                Ok(text) => {
                    let hits = find_hits_in_text(page_index, &text, &query_lower);
                    if !hits.is_empty()
                        && progress_tx
                            .send(SearchProgress {
                                generation: request.generation,
                                hits,
                                finished: false,
                            })
                            .is_err()
                    {
                        return;
                    }
                }
                Err(error) => tracing::warn!("text_search: pg {page_index}: {error}"),
            }
        }

        if generation.load(Ordering::Acquire) == request.generation
            && progress_tx
                .send(SearchProgress {
                    generation: request.generation,
                    hits: Vec::new(),
                    finished: true,
                })
                .is_err()
        {
            return;
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
    fn attaching_document_does_not_start_worker() {
        let mut search = TextSearch::new();
        search.attach_document(PathBuf::from("document.pdf"));

        assert!(search.document_path.is_some());
        assert!(search.request_tx.is_none());
        assert!(search.progress_rx.is_none());
    }
}
