// pdf/page_cache.rs — Cache LRU por bytes com render em worker.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
};

use crate::byte_cache::{ByteLruCache, CacheMetrics};
use crate::pdf::engine::{get_pdfium, PdfEngine, RenderedPage};

/// Bitmaps CPU podem usar até 64 MiB. Uma única página maior continua utilizável
/// e fica sozinha no cache até ser substituída.
pub const DEFAULT_CPU_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Pedido enviado pelo main thread para a thread de render.
pub struct RenderRequest {
    pub page_index: usize,
    pub scale: f32,
}

/// Resposta da thread de render para o main thread.
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
/// 1. `insert_sync` — insere direto (main thread, Single-Page mode).
/// 2. Canal async  — recebe bitmaps da thread de background (Continuous).
pub struct LruPageCache {
    entries: ByteLruCache<usize, RenderedPage>,

    // ── Canal para background render ──
    tx: Option<SyncSender<RenderRequest>>,
    rx: Receiver<RenderResult>,
    inflight: std::collections::HashSet<usize>,
    failures: VecDeque<(usize, String)>,
}

impl LruPageCache {
    /// Cria um cache com orçamento `budget_bytes`.
    /// Inicia a thread de background imediatamente, ligada ao arquivo `pdf_path`.
    pub fn new(budget_bytes: usize, pdf_path: PathBuf, _scale_hint: f32) -> Self {
        let (req_tx, req_rx) = mpsc::sync_channel::<RenderRequest>(4);
        let (res_tx, res_rx) = mpsc::sync_channel::<RenderResult>(4);

        // Thread de background: reabre o PDF e renderiza sob demanda
        let path = pdf_path.clone();
        std::thread::Builder::new()
            .name("maripdf-render".into())
            .spawn(move || {
                background_render_loop(path, req_rx, res_tx);
            })
            .expect("Falha ao criar thread de render");

        Self {
            entries: ByteLruCache::new(budget_bytes),
            tx: Some(req_tx),
            rx: res_rx,
            inflight: Default::default(),
            failures: VecDeque::new(),
        }
    }

    /// Cria cache sem thread de background (modo Single-Page sync).
    pub fn new_sync(budget_bytes: usize) -> Self {
        let (_tx, rx) = mpsc::sync_channel::<RenderResult>(0);
        Self {
            entries: ByteLruCache::new(budget_bytes),
            tx: None,
            rx,
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

    /// Solicita pré-renderização de `page_index` na thread de background.
    /// Não bloqueia; ignora se já estiver em cache ou em voo.
    pub fn prefetch(&mut self, page_index: usize, scale: f32) {
        if self.entries.contains(page_index) {
            return;
        }
        if self.inflight.contains(&page_index) {
            return;
        }
        if let Some(ref tx) = self.tx {
            if tx.try_send(RenderRequest { page_index, scale }).is_ok() {
                self.inflight.insert(page_index);
            }
        }
    }

    /// Drena resultados prontos da thread de background.
    /// Retorna `true` se ao menos um bitmap chegou (sinaliza repaint).
    pub fn drain_ready(&mut self) -> bool {
        let mut got_any = false;
        while let Ok(result) = self.rx.try_recv() {
            got_any |= self.accept_result(result);
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
        self.inflight.insert(result.page_index);
        self.accept_result(result)
    }

    #[cfg(test)]
    pub(super) fn is_inflight(&self, page_index: usize) -> bool {
        self.inflight.contains(&page_index)
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
        self.inflight.retain(|&idx| idx >= first && idx <= last);
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

fn background_render_loop(
    path: PathBuf,
    rx: std::sync::mpsc::Receiver<RenderRequest>,
    tx: SyncSender<RenderResult>,
) {
    // Tenta abrir o PDF; se falhar, a thread encerra silenciosamente
    let pdfium = match get_pdfium() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("bg_render: falha ao carregar pdfium: {e}");
            return;
        }
    };
    let engine = match PdfEngine::open(&path, pdfium) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("bg_render: falha ao abrir PDF: {e}");
            return;
        }
    };

    for req in rx {
        match engine.render_page(req.page_index, req.scale) {
            Ok(page) => {
                // O worker pode aguardar o consumidor: isso aplica backpressure
                // sem descartar resultados e sem deixar `inflight` preso.
                match tx.send(RenderResult {
                    page_index: req.page_index,
                    outcome: RenderOutcome::Ready(page),
                }) {
                    Ok(_) => {}
                    Err(_) => {
                        tracing::info!("bg_render: receiver desconectado, encerrando");
                        return;
                    }
                }
            }
            Err(e) => {
                if tx
                    .send(RenderResult {
                        page_index: req.page_index,
                        outcome: RenderOutcome::Failed(e.to_string()),
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    }
    tracing::info!("bg_render: thread encerrada");
}
