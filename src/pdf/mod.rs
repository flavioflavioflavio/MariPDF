// pdf/mod.rs — Módulo central de integração PDF

pub mod document_worker;
pub mod engine;
pub mod extract;
pub mod page_cache;
#[cfg(test)]
mod page_cache_tests;
pub mod persistence;
#[cfg(test)]
mod robustness_tests;
pub mod text_search;
pub mod thumb_cache;
#[cfg(test)]
mod tile_spike_tests;
