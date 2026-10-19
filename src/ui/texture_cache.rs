// ui/texture_cache.rs — Cache GPU de páginas limitado por bytes RGBA.

use crate::byte_cache::{ByteLruCache, CacheMetrics};
use eframe::egui;

pub(crate) const DEFAULT_GPU_CACHE_BYTES: usize = 128 * 1024 * 1024;

pub(crate) struct GpuTextureCache {
    entries: ByteLruCache<usize, egui::TextureHandle>,
}

impl GpuTextureCache {
    pub fn new() -> Self {
        Self::with_budget(DEFAULT_GPU_CACHE_BYTES)
    }

    fn with_budget(budget_bytes: usize) -> Self {
        Self {
            entries: ByteLruCache::new(budget_bytes),
        }
    }

    pub fn get(&mut self, page: usize) -> Option<&egui::TextureHandle> {
        self.entries.get(page)
    }

    #[cfg(test)]
    pub fn contains(&self, page: usize) -> bool {
        self.entries.contains(page)
    }

    pub fn insert(&mut self, page: usize, texture: egui::TextureHandle, width: u32, height: u32) {
        let bytes = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        self.entries.insert(page, texture, bytes);
    }

    pub fn retain_range(&mut self, first: usize, last: usize) {
        self.entries.retain(|page| page >= first && page <= last);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn metrics(&self) -> CacheMetrics {
        self.entries.metrics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(ctx: &egui::Context, name: &str) -> egui::TextureHandle {
        ctx.load_texture(
            name,
            egui::ColorImage::new([2, 2], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        )
    }

    #[test]
    fn gpu_cache_accounts_rgba_bytes_and_evicts_lru() {
        let ctx = egui::Context::default();
        let mut cache = GpuTextureCache::with_budget(16);
        cache.insert(1, texture(&ctx, "one"), 2, 2);
        cache.insert(2, texture(&ctx, "two"), 2, 2);

        assert!(!cache.contains(1));
        assert!(cache.contains(2));
        assert_eq!(cache.metrics().resident_bytes, 16);
        assert_eq!(cache.metrics().evictions, 1);
    }
}
