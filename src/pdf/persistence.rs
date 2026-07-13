// pdf/persistence.rs — Persistência de sessão por arquivo
//
// Salva: última página, zoom, modo de visualização
// Local: %APPDATA%\maripdf\session.toml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const APP_DIR: &str = "maripdf";
const SESSION_FILE: &str = "session.toml";
const WINDOW_FILE: &str = "window.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileSession {
    /// Última página visitada (0-indexed)
    pub last_page: usize,
    /// Último zoom (1.0 = 100%)
    pub zoom: f32,
    /// Timestamp Unix do último acesso (para ordenar recentes)
    #[serde(default)]
    pub last_opened: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SessionStore {
    /// Chave: caminho absoluto do arquivo PDF
    files: HashMap<String, FileSession>,
}

/// Tamanho e posição da janela persistidos entre sessões.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSession {
    pub width: f32,
    pub height: f32,
}

impl Default for WindowSession {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
        }
    }
}

pub struct PersistenceManager {
    path: PathBuf,
    store: SessionStore,
}

impl PersistenceManager {
    /// Carrega (ou cria) o arquivo de sessão
    pub fn load() -> Self {
        let path = session_path();
        let store = try_load(&path).unwrap_or_default();
        Self { path, store }
    }

    /// Recupera a sessão para um arquivo PDF específico
    pub fn get(&self, pdf_path: &Path) -> Option<&FileSession> {
        let key = pdf_path.to_string_lossy().to_string();
        self.store.files.get(&key)
    }

    /// Salva a sessão para um arquivo PDF
    pub fn save(&mut self, pdf_path: &Path, session: FileSession) {
        let key = pdf_path.to_string_lossy().to_string();
        self.store.files.insert(key, session);
        if let Err(e) = try_save(&self.path, &self.store) {
            tracing::warn!("Falha ao salvar sessão: {e}");
        }
    }

    /// Retorna até `max` arquivos recentes ordenados por último acesso (mais recente primeiro).
    /// Filtra arquivos que não existem mais no disco.
    pub fn recent_files(&self, max: usize) -> Vec<(PathBuf, FileSession)> {
        let mut list: Vec<(PathBuf, FileSession)> = self
            .store
            .files
            .iter()
            .filter_map(|(path_str, session)| {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    Some((path, session.clone()))
                } else {
                    None
                }
            })
            .collect();
        list.sort_by(|a, b| b.1.last_opened.cmp(&a.1.last_opened));
        list.truncate(max);
        list
    }

    /// Remove um arquivo da lista de recentes e persiste
    pub fn remove_recent(&mut self, pdf_path: &Path) {
        let key = pdf_path.to_string_lossy().to_string();
        self.store.files.remove(&key);
        if let Err(e) = try_save(&self.path, &self.store) {
            tracing::warn!("Falha ao salvar sessão após remoção: {e}");
        }
    }

    /// Remove todo o histórico da memória e persiste o estado vazio.
    pub fn clear_all(&mut self) {
        self.store.files.clear();
        if let Err(e) = try_save(&self.path, &self.store) {
            tracing::warn!("Falha ao limpar histórico: {e}");
        }
    }

    // ── Janela ────────────────────────────────────────────────────────────────

    /// Carrega as dimensões da janela da sessão anterior.
    pub fn load_window() -> WindowSession {
        let path = window_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persiste as dimensões atuais da janela.
    pub fn save_window(w: f32, h: f32) {
        let path = window_path();
        let session = WindowSession {
            width: w.max(640.0),
            height: h.max(480.0),
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(&session) {
            let _ = std::fs::write(&path, text);
        }
    }
}

fn session_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_DIR).join(SESSION_FILE)
}

fn window_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_DIR).join(WINDOW_FILE)
}

fn try_load(path: &Path) -> Option<SessionStore> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

fn try_save(path: &Path, store: &SessionStore) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(store)?;
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_store_round_trips_through_toml() {
        let unique = format!(
            "maripdf-persistence-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after Unix epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let pdf = PathBuf::from(r"C:\docs\sample.pdf");
        let mut store = SessionStore::default();
        store.files.insert(
            pdf.to_string_lossy().to_string(),
            FileSession {
                last_page: 42,
                zoom: 1.75,
                last_opened: 123,
            },
        );

        try_save(&path, &store).expect("save test session");
        let loaded = try_load(&path).expect("load test session");
        let session = loaded
            .files
            .get(&pdf.to_string_lossy().to_string())
            .expect("stored PDF session");

        assert_eq!(session.last_page, 42);
        assert_eq!(session.zoom, 1.75);
        assert_eq!(session.last_opened, 123);

        let _ = std::fs::remove_file(path);
    }
}
