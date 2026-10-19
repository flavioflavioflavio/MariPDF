// pdf/mod.rs — Módulo central de integração PDF

pub mod document_worker;
pub mod engine;
pub mod extract;
pub mod page_cache;
#[cfg(test)]
mod page_cache_tests;
pub mod persistence;
pub mod text_search;
pub mod thumb_cache;
