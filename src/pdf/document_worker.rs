// pdf/document_worker.rs — Scheduler de metadados do documento.

use crate::pdf::engine::{BookmarkEntry, PdfEngine, PdfLinkInfo, TextCharInfo};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};

const REQUEST_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Background,
    Visible,
    Immediate,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DocumentJobTarget {
    Bookmarks,
    Links(usize),
    Chars(usize),
    Text(usize),
}

#[derive(Debug)]
enum JobKind {
    Bookmarks,
    Links(usize),
    Chars(usize),
    Text(usize),
}

impl JobKind {
    fn key(&self) -> DocumentJobTarget {
        match self {
            Self::Bookmarks => DocumentJobTarget::Bookmarks,
            Self::Links(page) => DocumentJobTarget::Links(*page),
            Self::Chars(page) => DocumentJobTarget::Chars(*page),
            Self::Text(page) => DocumentJobTarget::Text(*page),
        }
    }
}

#[derive(Debug)]
struct DocumentJob {
    id: u64,
    generation: u64,
    sequence: u64,
    priority: JobPriority,
    kind: JobKind,
}

#[derive(Debug)]
struct QueuedJob(DocumentJob);

impl PartialEq for QueuedJob {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for QueuedJob {}

impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .priority
            .cmp(&other.0.priority)
            // BinaryHeap e max-first; menor sequence deve vencer no empate.
            .then_with(|| other.0.sequence.cmp(&self.0.sequence))
    }
}

pub enum DocumentJobOutput {
    Bookmarks(Vec<BookmarkEntry>),
    Links {
        page_index: usize,
        links: Vec<PdfLinkInfo>,
    },
    Chars {
        page_index: usize,
        chars: Vec<TextCharInfo>,
    },
    Text {
        page_index: usize,
        text: String,
    },
}

pub enum JobOutcome {
    Ready(DocumentJobOutput),
    Failed(String),
    Cancelled,
}

pub struct JobCompletion {
    pub id: u64,
    pub generation: u64,
    pub duration: Duration,
    pub target: DocumentJobTarget,
    pub outcome: JobOutcome,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocumentWorkerMetrics {
    pub queued: usize,
    pub inflight: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Default)]
struct SharedMetrics {
    queued: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
    cancelled: AtomicUsize,
}

/// Worker único para metadados e texto do documento atual.
///
/// A fila é bounded, deduplica jobs equivalentes e entrega exatamente um estado
/// terminal para cada job aceito. Cancelamento é lógico por `generation`.
pub struct DocumentWorker {
    request_tx: Option<SyncSender<DocumentJob>>,
    result_rx: Receiver<JobCompletion>,
    generation: Arc<AtomicU64>,
    metrics: Arc<SharedMetrics>,
    inflight: HashMap<DocumentJobTarget, u64>,
    next_id: u64,
    next_sequence: u64,
}

impl DocumentWorker {
    pub fn new(engine: Arc<PdfEngine>) -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel(REQUEST_CAPACITY);
        let (result_tx, result_rx) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(1));
        let metrics = Arc::new(SharedMetrics::default());

        let worker_generation = Arc::clone(&generation);
        let worker_metrics = Arc::clone(&metrics);
        std::thread::Builder::new()
            .name("maripdf-document".into())
            .spawn(move || {
                run_worker(
                    engine,
                    request_rx,
                    result_tx,
                    worker_generation,
                    worker_metrics,
                );
            })
            .expect("Falha ao criar thread maripdf-document");

        Self {
            request_tx: Some(request_tx),
            result_rx,
            generation,
            metrics,
            inflight: HashMap::new(),
            next_id: 1,
            next_sequence: 1,
        }
    }

    pub fn request_bookmarks(&mut self) -> bool {
        self.submit(JobKind::Bookmarks, JobPriority::Background)
    }

    pub fn request_links(&mut self, page_index: usize) -> bool {
        self.submit(JobKind::Links(page_index), JobPriority::Visible)
    }

    pub fn request_chars(&mut self, page_index: usize) -> bool {
        self.submit(JobKind::Chars(page_index), JobPriority::Visible)
    }

    pub fn request_text(&mut self, page_index: usize) -> bool {
        self.submit(JobKind::Text(page_index), JobPriority::Immediate)
    }

    fn submit(&mut self, kind: JobKind, priority: JobPriority) -> bool {
        let key = kind.key();
        if self.inflight.contains_key(&key) {
            return false;
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1).max(1);
        let job = DocumentJob {
            id,
            generation: self.generation.load(AtomicOrdering::Acquire),
            sequence,
            priority,
            kind,
        };

        let accepted = self
            .request_tx
            .as_ref()
            .is_some_and(|tx| tx.try_send(job).is_ok());
        if accepted {
            self.inflight.insert(key, id);
            self.metrics.queued.fetch_add(1, AtomicOrdering::AcqRel);
        }
        accepted
    }

    pub fn drain_results(&mut self) -> Vec<JobCompletion> {
        let mut results = Vec::new();
        while let Ok(completion) = self.result_rx.try_recv() {
            if self.inflight.get(&completion.target) == Some(&completion.id) {
                self.inflight.remove(&completion.target);
            }
            tracing::debug!(
                job_id = completion.id,
                generation = completion.generation,
                duration_ms = completion.duration.as_secs_f64() * 1000.0,
                "document_job: terminal"
            );
            results.push(completion);
        }
        if !results.is_empty() {
            let snapshot = self.metrics();
            tracing::debug!(
                queued = snapshot.queued,
                inflight = snapshot.inflight,
                completed = snapshot.completed,
                failed = snapshot.failed,
                cancelled = snapshot.cancelled,
                "document_worker: metricas"
            );
        }
        results
    }

    pub fn cancel_pending(&mut self) {
        self.generation.fetch_add(1, AtomicOrdering::AcqRel);
        self.inflight.clear();
    }

    pub fn metrics(&self) -> DocumentWorkerMetrics {
        DocumentWorkerMetrics {
            queued: self.metrics.queued.load(AtomicOrdering::Acquire),
            inflight: self.inflight.len(),
            completed: self.metrics.completed.load(AtomicOrdering::Acquire),
            failed: self.metrics.failed.load(AtomicOrdering::Acquire),
            cancelled: self.metrics.cancelled.load(AtomicOrdering::Acquire),
        }
    }
}

impl Drop for DocumentWorker {
    fn drop(&mut self) {
        self.cancel_pending();
        self.request_tx.take();
    }
}

fn run_worker(
    engine: Arc<PdfEngine>,
    request_rx: Receiver<DocumentJob>,
    result_tx: mpsc::Sender<JobCompletion>,
    generation: Arc<AtomicU64>,
    metrics: Arc<SharedMetrics>,
) {
    let mut queue = BinaryHeap::new();
    let mut disconnected = false;

    loop {
        if queue.is_empty() && !disconnected {
            match request_rx.recv() {
                Ok(job) => queue.push(QueuedJob(job)),
                Err(_) => disconnected = true,
            }
        }
        while let Ok(job) = request_rx.try_recv() {
            queue.push(QueuedJob(job));
        }

        let Some(QueuedJob(job)) = queue.pop() else {
            if disconnected {
                break;
            }
            continue;
        };
        metrics.queued.fetch_sub(1, AtomicOrdering::AcqRel);

        let started = Instant::now();
        let key = job.kind.key();
        let outcome = if generation.load(AtomicOrdering::Acquire) != job.generation {
            metrics.cancelled.fetch_add(1, AtomicOrdering::AcqRel);
            JobOutcome::Cancelled
        } else {
            execute_job(&engine, job.kind).unwrap_or_else(|error| {
                metrics.failed.fetch_add(1, AtomicOrdering::AcqRel);
                JobOutcome::Failed(error.to_string())
            })
        };
        if matches!(outcome, JobOutcome::Ready(_)) {
            metrics.completed.fetch_add(1, AtomicOrdering::AcqRel);
        }

        let completion = JobCompletion {
            id: job.id,
            generation: job.generation,
            duration: started.elapsed(),
            target: key,
            outcome,
        };
        if result_tx.send(completion).is_err() {
            break;
        }
    }
}

fn execute_job(engine: &PdfEngine, kind: JobKind) -> anyhow::Result<JobOutcome> {
    let output = match kind {
        JobKind::Bookmarks => DocumentJobOutput::Bookmarks(engine.extract_bookmarks()),
        JobKind::Links(page_index) => DocumentJobOutput::Links {
            page_index,
            links: engine.extract_page_links(page_index),
        },
        JobKind::Chars(page_index) => DocumentJobOutput::Chars {
            page_index,
            chars: engine.extract_chars_with_bounds(page_index)?,
        },
        JobKind::Text(page_index) => DocumentJobOutput::Text {
            page_index,
            text: engine.extract_text(page_index)?,
        },
    };
    Ok(JobOutcome::Ready(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::engine::{get_pdfium, PdfEngine};

    fn queued(id: u64, sequence: u64, priority: JobPriority) -> QueuedJob {
        QueuedJob(DocumentJob {
            id,
            generation: 1,
            sequence,
            priority,
            kind: JobKind::Links(id as usize),
        })
    }

    #[test]
    fn priority_queue_prefers_urgent_jobs_and_preserves_fifo() {
        let mut queue = BinaryHeap::new();
        queue.push(queued(1, 1, JobPriority::Background));
        queue.push(queued(2, 2, JobPriority::Visible));
        queue.push(queued(3, 3, JobPriority::Immediate));
        queue.push(queued(4, 4, JobPriority::Visible));

        let order: Vec<_> = std::iter::from_fn(|| queue.pop().map(|job| job.0.id)).collect();
        assert_eq!(order, vec![3, 2, 4, 1]);
    }

    #[test]
    fn job_keys_deduplicate_by_kind_and_page() {
        assert_eq!(JobKind::Links(4).key(), JobKind::Links(4).key());
        assert_ne!(JobKind::Links(4).key(), JobKind::Links(5).key());
        assert_ne!(JobKind::Links(4).key(), JobKind::Chars(4).key());
    }

    #[test]
    #[ignore = "requer MARIPDF_TEST_PDFS com um caminho por linha"]
    fn real_pdf_jobs_reach_terminal_state() {
        let raw = std::env::var("MARIPDF_TEST_PDFS").expect("defina MARIPDF_TEST_PDFS");
        let path = std::path::PathBuf::from(
            raw.lines()
                .find(|line| !line.trim().is_empty())
                .expect("inclua ao menos um PDF"),
        );
        let engine = Arc::new(PdfEngine::open(&path, get_pdfium().unwrap()).unwrap());
        let mut worker = DocumentWorker::new(engine);

        assert!(worker.request_bookmarks());
        assert!(worker.request_links(0));
        assert!(worker.request_chars(0));
        assert!(worker.request_text(0));
        assert!(!worker.request_links(0), "job duplicado deve ser recusado");

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut terminal = Vec::new();
        while terminal.len() < 4 && Instant::now() < deadline {
            terminal.extend(worker.drain_results());
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(terminal.len(), 4);
        assert!(terminal
            .iter()
            .all(|result| matches!(result.outcome, JobOutcome::Ready(_))));
        let snapshot = worker.metrics();
        assert_eq!(snapshot.inflight, 0);
        assert_eq!(snapshot.completed, 4);
        assert_eq!(snapshot.failed, 0);
    }
}
