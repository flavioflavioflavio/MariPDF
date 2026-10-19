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
