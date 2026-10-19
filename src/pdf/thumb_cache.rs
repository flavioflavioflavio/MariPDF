// pdf/thumb_cache.rs — Cache LRU de thumbnails de páginas PDF
//
// Design:
//   • Scheduler global priorizado — não bloqueia a UI nem duplica o documento
//   • LRU com capacidade 40 thumbs (~8 MB total a 120px de altura)
//   • Lazy: só renderiza as páginas visíveis + 2 de buffer
//   • drain_ready() → main thread drena bitmaps prontos e repinta

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::pdf::document_worker::{
    DocumentJobOutput, DocumentJobTarget, DocumentWorkerHandle, JobCompletion, JobOutcome,
};
use crate::pdf::engine::RenderedPage;

/// Altura alvo dos thumbnails em pixels (largura proporcional à página)
pub const THUMB_HEIGHT: u32 = 140;

/// Capacidade máxima do cache LRU de thumbnails
pub const THUMB_CAPACITY: usize = 40;

// ── ThumbCache ────────────────────────────────────────────────────────────────

/// Entrada do cache: bitmap + ordem de acesso LRU
struct CacheEntry {
    page: RenderedPage,
    lru_stamp: u64,
}

/// Cache LRU de thumbnails alimentado pelo scheduler global do documento.
pub struct ThumbCache {
    entries: HashMap<usize, CacheEntry>,
    lru_order: VecDeque<usize>, // fila de eviction auxiliar
    capacity: usize,
    clock: u64,

    scheduler: DocumentWorkerHandle,
    result_tx: Sender<JobCompletion>,
    result_rx: Receiver<JobCompletion>,
    inflight: HashSet<usize>,

    pub page_count: usize,
}

impl ThumbCache {
    /// Cria o cache ligado ao scheduler global do documento.
    pub fn new(page_count: usize, scheduler: DocumentWorkerHandle) -> Self {
        let (result_tx, result_rx) = mpsc::channel();

        Self {
            entries: HashMap::with_capacity(THUMB_CAPACITY),
            lru_order: VecDeque::with_capacity(THUMB_CAPACITY),
            capacity: THUMB_CAPACITY,
            clock: 0,
            scheduler,
            result_tx,
            result_rx,
            inflight: HashSet::new(),
            page_count,
        }
    }

    // ── API pública ───────────────────────────────────────────────────────────

    /// Retorna o bitmap se já estiver em cache (atualiza LRU).
    pub fn get(&mut self, page: usize) -> Option<&RenderedPage> {
        if let Some(entry) = self.entries.get_mut(&page) {
            self.clock += 1;
            entry.lru_stamp = self.clock;
            Some(&entry.page)
        } else {
            None
        }
    }

    /// Solicita renderização de uma página (não bloqueia).
    /// Ignora se já está em cache ou em andamento.
    pub fn request(&mut self, page: usize) {
        if self.entries.contains_key(&page) {
            return;
        }
        if self.inflight.contains(&page) {
            return;
        }
        if self
            .scheduler
            .request_thumbnail(page, THUMB_HEIGHT, self.result_tx.clone())
        {
            self.inflight.insert(page);
        }
    }

    /// Drena bitmaps prontos do scheduler global.
    /// Retorna `true` se ao menos um chegou (sinaliza repaint ao caller).
    pub fn drain_ready(&mut self) -> bool {
        let mut got_any = false;
        while let Ok(completion) = self.result_rx.try_recv() {
            let DocumentJobTarget::Thumbnail(page_index) = completion.target else {
                continue;
            };
            self.inflight.remove(&page_index);
            match completion.outcome {
                JobOutcome::Ready(DocumentJobOutput::Thumbnail {
                    page_index: output_page,
                    page,
                }) if output_page == page_index => {
                    self.evict_if_full();
                    self.clock += 1;
                    self.entries.insert(
                        page_index,
                        CacheEntry {
                            page,
                            lru_stamp: self.clock,
                        },
                    );
                    self.lru_order.push_back(page_index);
                    got_any = true;
                }
                JobOutcome::Failed(error) => {
                    tracing::warn!("thumbs: página {page_index}: {error}");
                }
                JobOutcome::Cancelled => {}
                JobOutcome::Ready(_) => {
                    tracing::warn!("thumbs: resultado de tipo inesperado");
                }
            }
        }
        got_any
    }

    /// Retorna (width, height) do thumb em cache sem atualizar o LRU.
    /// Usado para calcular a proporção real da página no layout.
    pub fn get_cached_dimensions(&self, page: usize) -> Option<(u32, u32)> {
        self.entries
            .get(&page)
            .map(|e| (e.page.width, e.page.height))
    }

    // ── LRU eviction ─────────────────────────────────────────────────────────

    fn evict_if_full(&mut self) {
        if self.entries.len() < self.capacity {
            return;
        }
        // Expulsa o thumb com menor lru_stamp
        if let Some((&oldest, _)) = self.entries.iter().min_by_key(|(_, e)| e.lru_stamp) {
            self.entries.remove(&oldest);
            self.lru_order.retain(|&x| x != oldest);
        }
    }
}
