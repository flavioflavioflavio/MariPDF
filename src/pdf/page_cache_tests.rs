use super::document_worker::JobPriority;
use super::engine::RenderedPage;
use super::page_cache::{LruPageCache, RenderOutcome, RenderResult};

fn rendered(marker: u8) -> RenderedPage {
    RenderedPage {
        scale: 1.0,
        width: 1,
        height: 1,
        rgba: vec![marker, 0, 0, 255],
    }
}

#[test]
fn lru_evicts_the_least_recently_used_page() {
    let mut cache = LruPageCache::new_sync(8);
    cache.insert_sync(1, rendered(1));
    cache.insert_sync(2, rendered(2));

    assert!(cache.get(1).is_some());
    cache.insert_sync(3, rendered(3));

    assert!(cache.contains(1));
    assert!(!cache.contains(2));
    assert!(cache.contains(3));
    assert_eq!(cache.len(), 2);
}

#[test]
fn clear_removes_cached_pages() {
    let mut cache = LruPageCache::new_sync(8);
    cache.insert_sync(1, rendered(1));
    cache.clear();

    assert_eq!(cache.len(), 0);
    assert!(!cache.contains(1));
}

#[test]
fn failed_result_clears_inflight_and_can_be_consumed() {
    let mut cache = LruPageCache::new_sync(8);
    let ready = cache.inject_result(RenderResult {
        page_index: 4,
        outcome: RenderOutcome::Failed("render failed".into()),
    });

    assert!(!ready);
    assert!(!cache.is_inflight(4));
    assert_eq!(cache.take_failure(4).as_deref(), Some("render failed"));
    assert!(cache.take_failure(4).is_none());
}

#[test]
fn low_resolution_entry_is_dropped_when_a_much_sharper_render_is_requested() {
    let mut cache = LruPageCache::new_sync(1024);
    cache.insert_sync(1, rendered(1));
    // Mesma escala (ou próxima): continua no cache.
    cache.prefetch(1, 1.1, JobPriority::Visible);
    assert!(cache.contains(1));
    // Pedido bem mais nítido: o bitmap pequeno sai para ser re-renderizado.
    cache.prefetch(1, 2.0, JobPriority::Visible);
    assert!(!cache.contains(1));
}

#[test]
fn page_at_the_texture_cap_is_not_rerendered_forever() {
    use super::page_cache::needs_sharper_render;
    use crate::ui::page_scale::capped_raster_scale;
    // Página muito alta (595×3000 pt): a escala pedida passa do teto.
    let requested = 8.0;
    let capped = capped_raster_scale(595.0, 3000.0, requested);
    assert!(capped < requested);
    let at_cap = RenderedPage {
        scale: capped,
        width: (595.0 * capped) as u32,
        height: (3000.0 * capped) as u32,
        rgba: Vec::new(),
    };
    assert!(
        !needs_sharper_render(&at_cap, requested),
        "não pode pedir de novo o que já é o máximo"
    );
    // Abaixo do atingível (janela ampliada), ainda pede um mais nítido.
    let blurry = RenderedPage {
        scale: capped / 2.0,
        width: (595.0 * capped / 2.0) as u32,
        height: (3000.0 * capped / 2.0) as u32,
        rgba: Vec::new(),
    };
    assert!(needs_sharper_render(&blurry, requested));
}
