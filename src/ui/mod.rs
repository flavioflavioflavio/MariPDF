// ui/mod.rs — Módulo central da camada de apresentação
// Sub-módulos ativados conforme cada Fase do roadmap é implementada

// Fase 0: apenas declaração dos módulos para estrutura compilar
pub mod donation;
pub mod icons;
pub mod recent_files;
pub mod search;
pub mod settings_panel;
pub mod side_panel;
pub mod toolbar;
pub mod viewer;

// Módulos extraídos (refactoring Fase 2)
pub(crate) mod settings_tabs;
pub(crate) mod settings_widgets;
pub(crate) mod side_panel_thumbs;
pub(crate) mod side_panel_toc;
pub(crate) mod viewer_continuous;
pub(crate) mod viewer_helpers;
