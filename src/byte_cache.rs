// byte_cache.rs — LRU genérico limitado por orçamento real em bytes.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub evictions: u64,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub budget_bytes: usize,
}

struct Entry<V> {
    value: V,
    bytes: usize,
    lru_stamp: u64,
}

pub(crate) struct ByteLruCache<K, V> {
    entries: HashMap<K, Entry<V>>,
    budget_bytes: usize,
    resident_bytes: usize,
    clock: u64,
    metrics: CacheMetrics,
}

impl<K, V> ByteLruCache<K, V>
where
    K: Copy + Eq + Hash,
{
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            budget_bytes,
            resident_bytes: 0,
            clock: 0,
            metrics: CacheMetrics {
                budget_bytes,
                ..Default::default()
            },
        }
    }

    pub fn get(&mut self, key: K) -> Option<&V> {
        if let Some(entry) = self.entries.get_mut(&key) {
            self.clock += 1;
            entry.lru_stamp = self.clock;
            self.metrics.hits += 1;
            Some(&entry.value)
        } else {
            self.metrics.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V, bytes: usize) {
        if let Some(previous) = self.entries.remove(&key) {
            self.resident_bytes = self.resident_bytes.saturating_sub(previous.bytes);
        }

        // Uma entrada individual maior que o orçamento ainda é aceita, mas
        // permanece sozinha. Isso mantém páginas gigantes utilizáveis.
        while !self.entries.is_empty()
            && self.resident_bytes.saturating_add(bytes) > self.budget_bytes
        {
            self.evict_oldest();
        }

        self.clock += 1;
        self.entries.insert(
            key,
            Entry {
                value,
                bytes,
                lru_stamp: self.clock,
            },
        );
        self.resident_bytes = self.resident_bytes.saturating_add(bytes);
        self.metrics.insertions += 1;
        self.update_byte_metrics();
    }

    pub fn retain(&mut self, mut keep: impl FnMut(K) -> bool) {
        let removed: Vec<K> = self
            .entries
            .keys()
            .copied()
            .filter(|key| !keep(*key))
            .collect();
        for key in removed {
            if let Some(entry) = self.entries.remove(&key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(entry.bytes);
                self.metrics.evictions += 1;
            }
        }
        self.update_byte_metrics();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.resident_bytes = 0;
        self.update_byte_metrics();
    }

    pub fn contains(&self, key: K) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn metrics(&self) -> CacheMetrics {
        self.metrics
    }

    fn evict_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.lru_stamp)
            .map(|(key, _)| *key);
        if let Some(key) = oldest {
            if let Some(entry) = self.entries.remove(&key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(entry.bytes);
                self.metrics.evictions += 1;
            }
        }
    }

    fn update_byte_metrics(&mut self) {
        self.metrics.resident_bytes = self.resident_bytes;
        self.metrics.peak_bytes = self.metrics.peak_bytes.max(self.resident_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_lru_until_new_value_fits_budget() {
        let mut cache = ByteLruCache::new(8);
        cache.insert(1, "one", 4);
        cache.insert(2, "two", 4);
        assert_eq!(cache.get(1), Some(&"one"));

        cache.insert(3, "three", 4);

        assert!(cache.contains(1));
        assert!(!cache.contains(2));
        assert!(cache.contains(3));
        assert_eq!(cache.metrics().resident_bytes, 8);
        assert_eq!(cache.metrics().evictions, 1);
    }

    #[test]
    fn keeps_single_oversized_value_usable() {
        let mut cache = ByteLruCache::new(8);
        cache.insert(1, "large", 16);
        assert_eq!(cache.get(1), Some(&"large"));
        assert_eq!(cache.metrics().resident_bytes, 16);

        cache.insert(2, "small", 4);
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
        assert_eq!(cache.metrics().resident_bytes, 4);
    }

    #[test]
    fn retain_and_lookups_update_metrics() {
        let mut cache = ByteLruCache::new(16);
        cache.insert(1, "one", 4);
        cache.insert(2, "two", 4);
        assert!(cache.get(1).is_some());
        assert!(cache.get(9).is_none());
        cache.retain(|key| key == 2);

        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.evictions, 1);
        assert_eq!(metrics.resident_bytes, 4);
    }
}
