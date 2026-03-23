// i18n/mod.rs — Internacionalização via Mozilla Fluent
// Fase 4: implementação completa (hot-swap, on-demand download)
// Fase 0: apenas stub com mensagens hardcoded para desbloquear compilação

pub mod loader;

/// Retorna uma string traduzida pela chave.
/// Fase 0: stub que retorna a chave como fallback.
/// Fase 4: substituído por FluentBundle real com hot-swap.
pub fn t(key: &str) -> String {
    // TODO(Fase 4): implementar FluentBundle com hot-swap
    key.to_string()
}
