// ui/mod.rs — Módulo central da camada de apresentação

pub mod icons;
pub mod settings_panel;
pub mod side_panel;
pub mod viewer;

pub(crate) mod page_layout;
#[cfg(test)]
mod page_layout_tests;
pub(crate) mod prefetch;
pub(crate) mod settings_tabs;
pub(crate) mod settings_widgets;
pub(crate) mod side_panel_thumbs;
pub(crate) mod side_panel_toc;
pub(crate) mod texture_cache;
pub(crate) mod viewer_continuous;
#[cfg(test)]
mod viewer_continuous_tests;
pub(crate) mod viewer_helpers;
