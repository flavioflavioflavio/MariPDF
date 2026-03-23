// pdf/thumb_cache.rs — Cache LRU de thumbnails de páginas PDF
//
// Design:
//   • Thread de background dedicada ("maripdf-thumbs") — não bloqueia UI
//   • LRU com capacidade 40 thumbs (~8 MB total a 120px de altura)
//   • Lazy: só renderiza as páginas visíveis + 2 de buffer
//   • drain_ready() → main thread drena bitmaps prontos e repinta

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
};

use crate::pdf::engine::{get_pdfium, PdfEngine, RenderedPage};

/// Altura alvo dos thumbnails em pixels (largura proporcional à página)
pub const THUMB_HEIGHT: u32 = 140;

/// Capacidade máxima do cache LRU de thumbnails
pub const THUMB_CAPACITY: usize = 40;

// ── Mensagens inter-thread ────────────────────────────────────────────────────

struct ThumbRequest {
    page_index: usize,
}

struct ThumbResult {
    page_index: usize,
    page:       RenderedPage,
}

// ── ThumbCache ────────────────────────────────────────────────────────────────

/// Entrada do cache: bitmap + ordem de acesso LRU
struct CacheEntry {
    page:      RenderedPage,
    lru_stamp: u64,
}

/// Cache LRU de thumbnails com renderização async em background thread.
pub struct ThumbCache {
    entries:    HashMap<usize, CacheEntry>,
    lru_order:  VecDeque<usize>,  // fila de eviction auxiliar
    capacity:   usize,
    clock:      u64,

    tx:         SyncSender<ThumbRequest>,
    rx:         Receiver<ThumbResult>,
    inflight:   HashSet<usize>,

    pub page_count: usize,
}

impl ThumbCache {
    /// Cria o cache e inicia a thread de background.
    pub fn new(pdf_path: PathBuf, page_count: usize) -> Self {
        let (req_tx, req_rx) = mpsc::sync_channel::<ThumbRequest>(8);
        let (res_tx, res_rx) = mpsc::sync_channel::<ThumbResult>(8);

        let path = pdf_path.clone();
        std::thread::Builder::new()
            .name("maripdf-thumbs".into())
            .spawn(move || {
                thumb_render_loop(path, req_rx, res_tx);
            })
            .expect("Falha ao criar thread maripdf-thumbs");

        Self {
            entries:    HashMap::with_capacity(THUMB_CAPACITY),
            lru_order:  VecDeque::with_capacity(THUMB_CAPACITY),
            capacity:   THUMB_CAPACITY,
            clock:      0,
            tx:         req_tx,
            rx:         res_rx,
            inflight:   HashSet::new(),
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
        if self.entries.contains_key(&page) { return; }
        if self.inflight.contains(&page)    { return; }
        if self.tx.try_send(ThumbRequest { page_index: page }).is_ok() {
            self.inflight.insert(page);
        }
    }

    /// Drena bitmaps prontos da thread de background.
    /// Retorna `true` se ao menos um chegou (sinaliza repaint ao caller).
    pub fn drain_ready(&mut self) -> bool {
        let mut got_any = false;
        while let Ok(result) = self.rx.try_recv() {
            self.evict_if_full();
            self.clock += 1;
            self.entries.insert(result.page_index, CacheEntry {
                page:      result.page,
                lru_stamp: self.clock,
            });
            self.lru_order.push_back(result.page_index);
            self.inflight.remove(&result.page_index);
            got_any = true;
        }
        got_any
    }

    /// Verifica se uma página está cacheada.
    #[allow(dead_code)]
    pub fn contains(&self, page: usize) -> bool {
        self.entries.contains_key(&page)
    }

    /// Retorna (width, height) do thumb em cache sem atualizar o LRU.
    /// Usado para calcular a proporção real da página no layout.
    pub fn get_cached_dimensions(&self, page: usize) -> Option<(u32, u32)> {
        self.entries.get(&page).map(|e| (e.page.width, e.page.height))
    }

    /// Limpa tudo (usado ao fechar documento).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.inflight.clear();
        self.lru_order.clear();
    }

    // ── LRU eviction ─────────────────────────────────────────────────────────

    fn evict_if_full(&mut self) {
        if self.entries.len() < self.capacity { return; }
        // Expulsa o thumb com menor lru_stamp
        if let Some((&oldest, _)) = self.entries
            .iter()
            .min_by_key(|(_, e)| e.lru_stamp)
        {
            self.entries.remove(&oldest);
            self.lru_order.retain(|&x| x != oldest);
        }
    }
}

// ── Thread de background ──────────────────────────────────────────────────────

fn thumb_render_loop(
    path: PathBuf,
    rx:   Receiver<ThumbRequest>,
    tx:   SyncSender<ThumbResult>,
) {
    let pdfium = match get_pdfium() {
        Ok(p)  => p,
        Err(e) => {
            tracing::error!("thumbs: falha ao carregar pdfium: {e}");
            return;
        }
    };
    let engine = match PdfEngine::open(&path, pdfium) {
        Ok(e)  => e,
        Err(e) => {
            tracing::error!("thumbs: falha ao abrir PDF: {e}");
            return;
        }
    };

    for req in rx {
        // Calcula escala para atingir THUMB_HEIGHT
        let (_, height_pts) = engine.page_size_pts(req.page_index);
        let scale = if height_pts > 0.0 {
            THUMB_HEIGHT as f32 / height_pts
        } else {
            0.2
        };

        match engine.render_page(req.page_index, scale) {
            Ok(page) => {
                match tx.try_send(ThumbResult { page_index: req.page_index, page }) {
                    Ok(_) => {}
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        tracing::info!("thumbs: receiver desconectado, encerrando");
                        return;
                    }
                    Err(mpsc::TrySendError::Full(_)) => {
                        tracing::trace!("thumbs: canal cheio, thumb descartado");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("thumbs: página {}: {e}", req.page_index);
            }
        }
    }
    tracing::info!("thumbs: thread encerrada");
}
