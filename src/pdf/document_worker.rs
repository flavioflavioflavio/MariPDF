// pdf/document_worker.rs — Scheduler global de trabalho PDF do documento.

use crate::pdf::engine::{
    BookmarkEntry, PdfEngine, PdfLinkInfo, RenderedPage, SearchPage, TextCharInfo,
};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        mpsc::{self, Receiver, Sender, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};

const REQUEST_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Search,
    Background,
    Visible,
    Immediate,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DocumentJobTarget {
    Bookmarks,
    Links(usize),
    Chars(usize),
    Render {
        page_index: usize,
        scale_bits: u32,
    },
    Thumbnail(usize),
    SearchText {
        page_index: usize,
        search_generation: u64,
    },
}

#[derive(Debug)]
enum JobKind {
    Bookmarks,
    Links(usize),
    Chars(usize),
    Render {
        page_index: usize,
        scale: f32,
    },
    Thumbnail {
        page_index: usize,
        height_px: u32,
    },
    SearchText {
        page_index: usize,
        search_generation: u64,
    },
}

impl JobKind {
    fn key(&self) -> DocumentJobTarget {
        match self {
            Self::Bookmarks => DocumentJobTarget::Bookmarks,
            Self::Links(page) => DocumentJobTarget::Links(*page),
            Self::Chars(page) => DocumentJobTarget::Chars(*page),
            Self::Render { page_index, scale } => DocumentJobTarget::Render {
                page_index: *page_index,
                scale_bits: scale.to_bits(),
            },
            Self::Thumbnail { page_index, .. } => DocumentJobTarget::Thumbnail(*page_index),
            Self::SearchText {
                page_index,
                search_generation,
            } => DocumentJobTarget::SearchText {
                page_index: *page_index,
                search_generation: *search_generation,
            },
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
    result_tx: Sender<JobCompletion>,
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
    Render {
        page_index: usize,
        scale: f32,
        page: RenderedPage,
    },
    Thumbnail {
        page_index: usize,
        page: RenderedPage,
    },
    SearchText {
        page_index: usize,
        search_generation: u64,
        page: SearchPage,
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
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Default)]
struct SharedMetrics {
    queued: AtomicUsize,
    running: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
    cancelled: AtomicUsize,
}

/// Acorda o loop da UI quando um job termina; sem ele, um resultado que chega
/// com a janela ociosa só seria aplicado no próximo evento de entrada.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Worker único para render, miniaturas, busca e metadados do documento atual.
///
/// A fila é bounded, deduplica jobs equivalentes e entrega exatamente um estado
/// terminal para cada job aceito. Cancelamento é lógico por `generation`.
pub struct DocumentWorker {
    handle: DocumentWorkerHandle,
    result_tx: Sender<JobCompletion>,
    result_rx: Receiver<JobCompletion>,
    inflight: HashMap<DocumentJobTarget, u64>,
}

#[derive(Clone)]
pub struct DocumentWorkerHandle {
    request_tx: SyncSender<DocumentJob>,
    queue_capacity: usize,
    generation: Arc<AtomicU64>,
    metrics: Arc<SharedMetrics>,
    next_id: Arc<AtomicU64>,
    next_sequence: Arc<AtomicU64>,
}

impl DocumentWorker {
    #[cfg(test)]
    pub fn new(engine: Arc<PdfEngine>) -> Self {
        Self::with_waker(engine, None)
    }

    pub fn with_waker(engine: Arc<PdfEngine>, waker: Option<Waker>) -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel(REQUEST_CAPACITY);
        let (result_tx, result_rx) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(1));
        let metrics = Arc::new(SharedMetrics::default());
        let handle = DocumentWorkerHandle {
            request_tx,
            queue_capacity: REQUEST_CAPACITY,
            generation: Arc::clone(&generation),
            metrics: Arc::clone(&metrics),
            next_id: Arc::new(AtomicU64::new(1)),
            next_sequence: Arc::new(AtomicU64::new(1)),
        };

        std::thread::Builder::new()
            .name("maripdf-document".into())
            .spawn(move || {
                run_worker(engine, request_rx, generation, metrics, waker);
            })
            .expect("Falha ao criar thread maripdf-document");

        Self {
            handle,
            result_tx,
            result_rx,
            inflight: HashMap::new(),
        }
    }

    pub fn handle(&self) -> DocumentWorkerHandle {
        self.handle.clone()
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

    fn submit(&mut self, kind: JobKind, priority: JobPriority) -> bool {
        let key = kind.key();
        if self.inflight.contains_key(&key) {
            return false;
        }

        if let Some(id) = self.handle.submit(kind, priority, self.result_tx.clone()) {
            self.inflight.insert(key, id);
            true
        } else {
            false
        }
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
                running = snapshot.running,
                completed = snapshot.completed,
                failed = snapshot.failed,
                cancelled = snapshot.cancelled,
                "document_worker: metricas"
            );
        }
        results
    }

    pub fn cancel_pending(&mut self) {
        self.handle.cancel_all();
        self.inflight.clear();
    }

    pub fn metrics(&self) -> DocumentWorkerMetrics {
        DocumentWorkerMetrics {
            queued: self.handle.metrics.queued.load(AtomicOrdering::Acquire),
            running: self.handle.metrics.running.load(AtomicOrdering::Acquire),
            completed: self.handle.metrics.completed.load(AtomicOrdering::Acquire),
            failed: self.handle.metrics.failed.load(AtomicOrdering::Acquire),
            cancelled: self.handle.metrics.cancelled.load(AtomicOrdering::Acquire),
        }
    }
}

impl Drop for DocumentWorker {
    fn drop(&mut self) {
        self.cancel_pending();
    }
}

impl DocumentWorkerHandle {
    /// Reserva uma vaga na fila limitada; falha se a fila estiver cheia.
    fn reserve_queue_slot(&self) -> bool {
        let queued = &self.metrics.queued;
        let mut current = queued.load(AtomicOrdering::Acquire);
        while current < self.queue_capacity {
            match queued.compare_exchange_weak(
                current,
                current + 1,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
        false
    }

    fn submit(
        &self,
        kind: JobKind,
        priority: JobPriority,
        result_tx: Sender<JobCompletion>,
    ) -> Option<u64> {
        if !self.reserve_queue_slot() {
            return None;
        }

        let id = self.next_id.fetch_add(1, AtomicOrdering::AcqRel);
        let sequence = self.next_sequence.fetch_add(1, AtomicOrdering::AcqRel);
        let job = DocumentJob {
            id,
            generation: self.generation.load(AtomicOrdering::Acquire),
            sequence,
            priority,
            kind,
            result_tx,
        };
        if self.request_tx.try_send(job).is_ok() {
            Some(id)
        } else {
            self.metrics.queued.fetch_sub(1, AtomicOrdering::AcqRel);
            None
        }
    }

    pub fn request_render(
        &self,
        page_index: usize,
        scale: f32,
        priority: JobPriority,
        result_tx: Sender<JobCompletion>,
    ) -> bool {
        self.submit(JobKind::Render { page_index, scale }, priority, result_tx)
            .is_some()
    }

    pub fn request_thumbnail(
        &self,
        page_index: usize,
        height_px: u32,
        result_tx: Sender<JobCompletion>,
    ) -> bool {
        self.submit(
            JobKind::Thumbnail {
                page_index,
                height_px,
            },
            JobPriority::Background,
            result_tx,
        )
        .is_some()
    }

    pub fn request_search_text(
        &self,
        page_index: usize,
        search_generation: u64,
        result_tx: Sender<JobCompletion>,
    ) -> bool {
        self.submit(
            JobKind::SearchText {
                page_index,
                search_generation,
            },
            JobPriority::Search,
            result_tx,
        )
        .is_some()
    }

    fn cancel_all(&self) {
        self.generation.fetch_add(1, AtomicOrdering::AcqRel);
    }
}

fn run_worker(
    engine: Arc<PdfEngine>,
    request_rx: Receiver<DocumentJob>,
    generation: Arc<AtomicU64>,
    metrics: Arc<SharedMetrics>,
    waker: Option<Waker>,
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
        metrics.running.fetch_add(1, AtomicOrdering::AcqRel);

        let started = Instant::now();
        let key = job.kind.key();
        let outcome = if generation.load(AtomicOrdering::Acquire) != job.generation {
            metrics.cancelled.fetch_add(1, AtomicOrdering::AcqRel);
            JobOutcome::Cancelled
        } else {
            // Um panic dentro do job vira falha daquele job; o worker segue
            // atendendo a fila em vez de deixar a feature morta em silêncio.
            guarded(|| execute_job(&engine, job.kind)).unwrap_or_else(|error| {
                metrics.failed.fetch_add(1, AtomicOrdering::AcqRel);
                JobOutcome::Failed(error.to_string())
            })
        };
        if matches!(outcome, JobOutcome::Ready(_)) {
            metrics.completed.fetch_add(1, AtomicOrdering::AcqRel);
        }
        metrics.running.fetch_sub(1, AtomicOrdering::AcqRel);

        let completion = JobCompletion {
            id: job.id,
            generation: job.generation,
            duration: started.elapsed(),
            target: key,
            outcome,
        };
        let _ = job.result_tx.send(completion);
        if let Some(wake) = &waker {
            wake();
        }
    }
}

/// Executa um job convertendo panic em erro comum.
pub(crate) fn guarded<T>(job: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
        .unwrap_or_else(|panic| Err(anyhow::anyhow!("panic no job: {}", panic_message(&panic))))
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "sem mensagem".into())
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
        JobKind::Render { page_index, scale } => DocumentJobOutput::Render {
            page_index,
            scale,
            page: engine.render_page(page_index, scale)?,
        },
        JobKind::Thumbnail {
            page_index,
            height_px,
        } => {
            // Altura como exibida: com 90°/270° é a largura original.
            let (_, height_pts) = engine.display_size_pts(page_index);
            let scale = if height_pts > 0.0 {
                height_px as f32 / height_pts
            } else {
                0.2
            };
            DocumentJobOutput::Thumbnail {
                page_index,
                page: engine.render_page(page_index, scale)?,
            }
        }
        JobKind::SearchText {
            page_index,
            search_generation,
        } => DocumentJobOutput::SearchText {
            page_index,
            search_generation,
            page: engine.extract_search_page(page_index)?,
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
            result_tx: mpsc::channel().0,
        })
    }

    #[test]
    fn priority_queue_prefers_urgent_jobs_and_preserves_fifo() {
        let mut queue = BinaryHeap::new();
        queue.push(queued(1, 1, JobPriority::Background));
        queue.push(queued(2, 2, JobPriority::Visible));
        queue.push(queued(3, 3, JobPriority::Immediate));
        queue.push(queued(4, 4, JobPriority::Visible));
        queue.push(queued(5, 5, JobPriority::Search));

        let order: Vec<_> = std::iter::from_fn(|| queue.pop().map(|job| job.0.id)).collect();
        assert_eq!(order, vec![3, 2, 4, 1, 5]);
    }

    #[test]
    fn job_keys_deduplicate_by_kind_and_page() {
        assert_eq!(JobKind::Links(4).key(), JobKind::Links(4).key());
        assert_ne!(JobKind::Links(4).key(), JobKind::Links(5).key());
        assert_ne!(JobKind::Links(4).key(), JobKind::Chars(4).key());
    }

    #[test]
    fn bounded_queue_reports_backpressure_without_counting_rejected_job() {
        let (request_tx, _request_rx) = mpsc::sync_channel(1);
        let metrics = Arc::new(SharedMetrics::default());
        let handle = DocumentWorkerHandle {
            request_tx,
            queue_capacity: 1,
            generation: Arc::new(AtomicU64::new(1)),
            metrics: Arc::clone(&metrics),
            next_id: Arc::new(AtomicU64::new(1)),
            next_sequence: Arc::new(AtomicU64::new(1)),
        };
        let (result_tx, _result_rx) = mpsc::channel();

        assert!(handle.request_render(1, 1.0, JobPriority::Immediate, result_tx.clone(),));
        assert!(!handle.request_render(2, 1.0, JobPriority::Visible, result_tx));
        assert_eq!(metrics.queued.load(AtomicOrdering::Acquire), 1);
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
        let wakes = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&wakes);
        let mut worker = DocumentWorker::with_waker(
            engine,
            Some(Arc::new(move || {
                counter.fetch_add(1, AtomicOrdering::AcqRel);
            })),
        );

        assert!(worker.request_bookmarks());
        assert!(worker.request_links(0));
        assert!(worker.request_chars(0));
        assert!(!worker.request_links(0), "job duplicado deve ser recusado");
        assert!(worker.handle.request_render(
            0,
            0.2,
            JobPriority::Immediate,
            worker.result_tx.clone(),
        ));
        assert!(worker
            .handle
            .request_thumbnail(0, 140, worker.result_tx.clone(),));
        assert!(worker
            .handle
            .request_search_text(0, 99, worker.result_tx.clone(),));

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut terminal = Vec::new();
        while terminal.len() < 6 && Instant::now() < deadline {
            terminal.extend(worker.drain_results());
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(terminal.len(), 6);
        assert!(terminal
            .iter()
            .all(|result| matches!(result.outcome, JobOutcome::Ready(_))));
        let snapshot = worker.metrics();
        assert_eq!(snapshot.running, 0);
        assert_eq!(snapshot.completed, 6);
        assert_eq!(snapshot.failed, 0);
        assert_eq!(
            wakes.load(AtomicOrdering::Acquire),
            6,
            "um wake por job terminal"
        );
    }

    #[test]
    #[ignore = "requer MARIPDF_TEST_PDFS com um caminho por linha"]
    fn generation_change_cancels_queued_real_pdf_jobs() {
        let raw = std::env::var("MARIPDF_TEST_PDFS").expect("defina MARIPDF_TEST_PDFS");
        let path = std::path::PathBuf::from(
            raw.lines()
                .find(|line| !line.trim().is_empty())
                .expect("inclua ao menos um PDF"),
        );
        let engine = Arc::new(PdfEngine::open(&path, get_pdfium().unwrap()).unwrap());
        let mut worker = DocumentWorker::new(engine);

        let mut accepted = 0;
        for page_index in 0..16 {
            accepted += usize::from(worker.handle.request_search_text(
                page_index,
                77,
                worker.result_tx.clone(),
            ));
        }
        assert!(accepted > 1);
        worker.handle.cancel_all();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut terminal = Vec::new();
        while terminal.len() < accepted && Instant::now() < deadline {
            terminal.extend(worker.drain_results());
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(terminal.len(), accepted);
        assert!(
            terminal
                .iter()
                .any(|result| matches!(result.outcome, JobOutcome::Cancelled)),
            "ao menos um job enfileirado deve ser cancelado"
        );
        assert!(worker.metrics().cancelled > 0);
    }
}
