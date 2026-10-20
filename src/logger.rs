// logger.rs — Logging em arquivo para MariPDF
//
// Salva logs em: %APPDATA%\maripdf\maripdf.log (rolling diário, max 5 arquivos)
// Também mantém stdout para desenvolvimento (filtrado por RUST_LOG).
//
// Uso: chamar `init()` uma vez em `main()` antes de qualquer `tracing::*`.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn configured_filter() -> EnvFilter {
    let mut filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // Alguns drivers Vulkan repetem estes avisos a cada present. Mantê-los no
    // log gera I/O contínuo sem ação possível para o usuário do MariPDF.
    for directive in ["wgpu_hal=error", "wgpu_core=error", "epaint=error"] {
        filter = filter.add_directive(
            directive
                .parse()
                .expect("diretiva estática de logging válida"),
        );
    }
    filter
}

/// Inicializa o sistema de logging.
/// Retorna o `WorkerGuard` do appender — manter vivo até o fim do processo.
pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    // Mesmo diretório do estado local (respeita MARIPDF_DATA_DIR).
    let log_dir = crate::storage::local_dir();

    // Cria o diretório se não existir
    if std::fs::create_dir_all(&log_dir).is_err() {
        // Sem diretório — só stdout
        return init_stdout_only();
    }

    // Rolling diário — mantém no máximo 5 arquivos
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("maripdf")
        .filename_suffix("log")
        .max_log_files(5)
        .build(log_dir)
        .ok()?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = configured_filter();

    tracing_subscriber::registry()
        .with(filter)
        // Camada de arquivo (JSON-like legível)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        // Camada de stdout
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        .init();

    Some(guard)
}

/// Fallback: só stdout (quando não consegue criar o diretório de log)
fn init_stdout_only() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = configured_filter();
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        .init();
    None
}
