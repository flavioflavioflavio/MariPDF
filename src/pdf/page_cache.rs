// pdf/page_cache.rs — Cache LRU de bitmaps de páginas PDF
//
// Mantém até `capacity` páginas renderizadas na RAM.
// Quando o limite é atingido, expulsa a página menos usada recentemente.
// Integra-se com um canal mpsc para receber bitmaps de uma thread de fundo.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
};

use crate::pdf::engine::{get_pdfium, PdfEngine, RenderedPage};

// ── Capacidade padrão do cache ────────────────────────────────────────────────

/// Número máximo de páginas renderizadas mantidas na RAM.
/// A4 a 1.5× escala ≈ 2.5 MB/página. 12 páginas ≈ 30 MB.
pub const DEFAULT_CAPACITY: usize = 12;

// ── Mensagens da thread de background ────────────────────────────────────────

/// Pedido enviado pelo main thread para a thread de render.
pub struct RenderRequest {
    pub page_index: usize,
    pub scale:      f32,
}

/// Resposta da thread de render para o main thread.
pub struct RenderResult {
    pub page_index: usize,
    pub page:       RenderedPage,
}

// ── LruPageCache ─────────────────────────────────────────────────────────────

/// Entrada do cache: bitmap + timestamp LRU.
struct CacheEntry {
    page:   RenderedPage,
    /// Contador monotônico de "último acesso".
    lru_stamp: u64,
}

/// Cache LRU de páginas renderizadas.
///
/// Possui dois modos de população:
/// 1. `insert_sync` — insere direto (main thread, Single-Page mode).
/// 2. Canal async  — recebe bitmaps da thread de background (Continuous).
pub struct LruPageCache {
    entries:  HashMap<usize, CacheEntry>,
    capacity: usize,
    clock:    u64,

    // ── Canal para background render ──
    tx:       Option<SyncSender<RenderRequest>>,
    rx:       Receiver<RenderResult>,
    inflight: std::collections::HashSet<usize>,
}

impl LruPageCache {
    /// Cria um cache com capacidade `capacity`.
    /// Inicia a thread de background imediatamente, ligada ao arquivo `pdf_path`.
    pub fn new(capacity: usize, pdf_path: PathBuf, scale_hint: f32) -> Self {
        let (req_tx, req_rx) = mpsc::sync_channel::<RenderRequest>(4);
        let (res_tx, res_rx) = mpsc::sync_channel::<RenderResult>(4);

        // Thread de background: reabre o PDF e renderiza sob demanda
        let path = pdf_path.clone();
        std::thread::Builder::new()
            .name("maripdf-render".into())
            .spawn(move || {
                background_render_loop(path, scale_hint, req_rx, res_tx);
            })
            .expect("Falha ao criar thread de render");

        Self {
            entries:  HashMap::with_capacity(capacity),
            capacity,
            clock:    0,
            tx:       Some(req_tx),
            rx:       res_rx,
            inflight: Default::default(),
        }
    }

    /// Cria cache sem thread de background (modo Single-Page sync).
    pub fn new_sync(capacity: usize) -> Self {
        let (_tx, rx) = mpsc::sync_channel::<RenderResult>(0);
        Self {
            entries:  HashMap::with_capacity(capacity),
            capacity,
            clock:    0,
            tx:       None,
            rx,
            inflight: Default::default(),
        }
    }

    // ── Acesso ───────────────────────────────────────────────────────────────

    /// Retorna uma referência ao bitmap se estiver no cache.
    pub fn get(&mut self, page_index: usize) -> Option<&RenderedPage> {
        if let Some(entry) = self.entries.get_mut(&page_index) {
            self.clock += 1;
            entry.lru_stamp = self.clock;
            Some(&entry.page)
        } else {
            None
        }
    }

    /// Insere um bitmap diretamente (uso sync / Single-Page).
    #[allow(dead_code)]
    pub fn insert_sync(&mut self, page_index: usize, page: RenderedPage) {
        self.evict_if_full();
        self.clock += 1;
        self.entries.insert(page_index, CacheEntry { page, lru_stamp: self.clock });
        self.inflight.remove(&page_index);
    }

    /// Solicita pré-renderização de `page_index` na thread de background.
    /// Não bloqueia; ignora se já estiver em cache ou em voo.
    pub fn prefetch(&mut self, page_index: usize, scale: f32) {
        if self.entries.contains_key(&page_index) { return; }
        if self.inflight.contains(&page_index)    { return; }
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
            self.evict_if_full();
            self.clock += 1;
            self.entries.insert(
                result.page_index,
                CacheEntry { page: result.page, lru_stamp: self.clock },
            );
            self.inflight.remove(&result.page_index);
            got_any = true;
        }
        got_any
    }

    /// Remove páginas fora do range `[first, last]` (janela de viewport).
    pub fn evict_range(&mut self, first: usize, last: usize) {
        self.entries.retain(|&idx, _| idx >= first && idx <= last);
        self.inflight.retain(|&idx| idx >= first && idx <= last);
    }

    /// Remove tudo.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.inflight.clear();
    }

    #[allow(dead_code)]
    pub fn contains(&self, page_index: usize) -> bool {
        self.entries.contains_key(&page_index)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize { self.entries.len() }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn evict_if_full(&mut self) {
        if self.entries.len() < self.capacity { return; }
        // Expulsa a página com menor lru_stamp (menos recentemente usada)
        if let Some((&oldest_key, _)) = self.entries
            .iter()
            .min_by_key(|(_, e)| e.lru_stamp)
        {
            self.entries.remove(&oldest_key);
        }
    }
}

// ── Thread de background ──────────────────────────────────────────────────────

fn background_render_loop(
    path:    PathBuf,
    scale:   f32,
    rx:      std::sync::mpsc::Receiver<RenderRequest>,
    tx:      SyncSender<RenderResult>,
) {
    // Tenta abrir o PDF; se falhar, a thread encerra silenciosamente
    let pdfium = match get_pdfium() {
        Ok(p)  => p,
        Err(e) => {
            tracing::error!("bg_render: falha ao carregar pdfium: {e}");
            return;
        }
    };
    let engine = match PdfEngine::open(&path, pdfium) {
        Ok(e)  => e,
        Err(e) => {
            tracing::error!("bg_render: falha ao abrir PDF: {e}");
            return;
        }
    };

    for req in rx {
        let use_scale = if scale > 0.05 { scale } else { req.scale };
        match engine.render_page(req.page_index, use_scale) {
            Ok(page) => {
                // try_send não bloqueia: se o receiver foi dropado (reset/Volta),
                // encerra a thread graciosamente em vez de travar o main thread.
                match tx.try_send(RenderResult { page_index: req.page_index, page }) {
                    Ok(_) => {}
                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                        tracing::info!("bg_render: receiver desconectado, encerrando");
                        return;
                    }
                    Err(std::sync::mpsc::TrySendError::Full(_)) => {
                        // Canal cheio mas ainda conectado — resultado descartado
                        tracing::trace!("bg_render: canal cheio, resultado descartado");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("bg_render: página {}: {e}", req.page_index);
            }
        }
    }
    tracing::info!("bg_render: thread encerrada");
}

// ── Send explícito ────────────────────────────────────────────────────────────
// pdfium-render com feature "sync" garante que PdfDocument: Send.
// LruPageCache não é publicamente compartilhado entre threads — reside somente
// no PdfViewer do main thread. O canal mpsc já é Send por natureza.
