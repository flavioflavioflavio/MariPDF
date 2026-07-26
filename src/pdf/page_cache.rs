// pdf/page_cache.rs — Cache LRU por bytes com render em worker.

use std::{
    collections::{HashMap, VecDeque},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::byte_cache::{ByteLruCache, CacheMetrics};
use crate::pdf::document_worker::{
    DocumentJobOutput, DocumentJobTarget, DocumentWorkerHandle, JobCompletion, JobOutcome,
    JobPriority,
};
use crate::pdf::engine::RenderedPage;

/// Bitmaps CPU podem usar até 64 MiB. Uma única página maior continua utilizável
/// e fica sozinha no cache até ser substituída.
pub const DEFAULT_CPU_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Resposta do scheduler de render para a main thread.
pub struct RenderResult {
    pub page_index: usize,
    pub outcome: RenderOutcome,
}

pub enum RenderOutcome {
    Ready(RenderedPage),
    Failed(String),
}

/// Cache LRU de páginas renderizadas.
///
/// Possui dois modos de população:
/// 1. `insert_sync` — insere direto em testes e caminhos auxiliares.
/// 2. Canal async — recebe bitmaps do scheduler global do documento.
pub struct LruPageCache {
    entries: ByteLruCache<usize, RenderedPage>,

    scheduler: Option<DocumentWorkerHandle>,
    result_tx: Sender<JobCompletion>,
    result_rx: Receiver<JobCompletion>,
    inflight: HashMap<usize, u32>,
    failures: VecDeque<(usize, String)>,
}

impl LruPageCache {
    /// Cria um cache com orçamento `budget_bytes`.
    /// Render e enviado ao scheduler global do documento.
    pub fn new(budget_bytes: usize, scheduler: DocumentWorkerHandle) -> Self {
        let (result_tx, result_rx) = mpsc::channel();

        Self {
            entries: ByteLruCache::new(budget_bytes),
            scheduler: Some(scheduler),
            result_tx,
            result_rx,
            inflight: Default::default(),
            failures: VecDeque::new(),
        }
    }

    /// Cria cache sem scheduler (usado por testes unitários).
    pub fn new_sync(budget_bytes: usize) -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        Self {
            entries: ByteLruCache::new(budget_bytes),
            scheduler: None,
            result_tx,
            result_rx,
            inflight: Default::default(),
            failures: VecDeque::new(),
        }
    }

    // ── Acesso ───────────────────────────────────────────────────────────────

    /// Retorna uma referência ao bitmap se estiver no cache.
    pub fn get(&mut self, page_index: usize) -> Option<&RenderedPage> {
        self.entries.get(page_index)
    }

    /// Insere um bitmap diretamente (uso sync / Single-Page).
    #[allow(dead_code)]
    pub fn insert_sync(&mut self, page_index: usize, page: RenderedPage) {
        let bytes = page.rgba.len();
        self.entries.insert(page_index, page, bytes);
        self.inflight.remove(&page_index);
    }

    /// Solicita renderização assíncrona de `page_index` ao scheduler global.
    /// Não bloqueia; ignora se já estiver em cache ou em voo.
    pub fn prefetch(&mut self, page_index: usize, scale: f32, priority: JobPriority) {
        if self.entries.contains(page_index) {
            return;
        }
        if self.inflight.contains_key(&page_index) {
            return;
        }
        if let Some(scheduler) = &self.scheduler {
            if scheduler.request_render(page_index, scale, priority, self.result_tx.clone()) {
                self.inflight.insert(page_index, scale.to_bits());
            }
        }
    }

    /// Drena resultados prontos do scheduler global.
    /// Retorna `true` se ao menos um bitmap chegou (sinaliza repaint).
    pub fn drain_ready(&mut self) -> bool {
        let mut got_any = false;
        while let Ok(completion) = self.result_rx.try_recv() {
            let DocumentJobTarget::Render {
                page_index,
                scale_bits,
            } = completion.target
            else {
                continue;
            };
            if self.inflight.get(&page_index) != Some(&scale_bits) {
                continue;
            }
            self.inflight.remove(&page_index);
            match completion.outcome {
                JobOutcome::Ready(DocumentJobOutput::Render {
                    page_index: output_page,
                    scale,
                    page,
                }) if output_page == page_index && scale.to_bits() == scale_bits => {
                    got_any |= self.accept_result(RenderResult {
                        page_index,
                        outcome: RenderOutcome::Ready(page),
                    });
                }
                JobOutcome::Failed(error) => {
                    got_any |= self.accept_result(RenderResult {
                        page_index,
                        outcome: RenderOutcome::Failed(error),
                    });
                }
                JobOutcome::Cancelled => {}
                JobOutcome::Ready(_) => {
                    tracing::warn!("render: resultado de tipo inesperado");
                }
            }
        }
        got_any
    }

    fn accept_result(&mut self, result: RenderResult) -> bool {
        self.inflight.remove(&result.page_index);
        match result.outcome {
            RenderOutcome::Ready(page) => {
                let bytes = page.rgba.len();
                self.entries.insert(result.page_index, page, bytes);
                true
            }
            RenderOutcome::Failed(error) => {
                tracing::warn!("bg_render: página {}: {error}", result.page_index);
                self.failures.push_back((result.page_index, error));
                false
            }
        }
    }

    #[cfg(test)]
    pub(super) fn inject_result(&mut self, result: RenderResult) -> bool {
        self.inflight.insert(result.page_index, 0);
        self.accept_result(result)
    }

    #[cfg(test)]
    pub(super) fn is_inflight(&self, page_index: usize) -> bool {
        self.inflight.contains_key(&page_index)
    }

    /// Retorna e remove a falha mais recente da página solicitada.
    pub fn take_failure(&mut self, page_index: usize) -> Option<String> {
        let position = self
            .failures
            .iter()
            .position(|(failed_page, _)| *failed_page == page_index)?;
        self.failures.remove(position).map(|(_, error)| error)
    }

    /// Remove páginas fora do range `[first, last]` (janela de viewport).
    pub fn evict_range(&mut self, first: usize, last: usize) {
        self.entries.retain(|idx| idx >= first && idx <= last);
        self.inflight.retain(|&idx, _| idx >= first && idx <= last);
    }

    /// Remove tudo nos testes unitários do cache.
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.inflight.clear();
        self.failures.clear();
    }

    #[allow(dead_code)]
    pub fn contains(&self, page_index: usize) -> bool {
        self.entries.contains(page_index)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn metrics(&self) -> CacheMetrics {
        self.entries.metrics()
    }
}
