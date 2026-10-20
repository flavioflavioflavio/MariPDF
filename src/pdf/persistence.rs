// pdf/persistence.rs — Persistência de sessão por arquivo e da janela
//
// Salva: última página, zoom e último acesso por PDF; posição da janela.
// Local e I/O resiliente: storage.rs (%LOCALAPPDATA%\maripdf\*.toml).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::storage::{self, StoreKind};

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

#[derive(Debug, Serialize, Deserialize)]
struct SessionStore {
    #[serde(default = "storage::schema_version")]
    schema_version: u32,
    /// Chave: caminho absoluto do arquivo PDF
    #[serde(default)]
    files: HashMap<String, FileSession>,
}

/// Posição, tamanho e maximização da janela entre sessões. Arquivos do formato
/// antigo (só largura/altura aproximadas) são ignorados e caem no padrão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSession {
    #[serde(default = "storage::schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub placement: Option<crate::platform::window_placement::Placement>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            schema_version: storage::SCHEMA_VERSION,
            files: HashMap::new(),
        }
    }
}

impl Default for WindowSession {
    fn default() -> Self {
        Self {
            schema_version: storage::SCHEMA_VERSION,
            placement: None,
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
        let path = storage::session_path();
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
        list.sort_by_key(|(_, session)| std::cmp::Reverse(session.last_opened));
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
    pub fn clear_all(&mut self) -> anyhow::Result<()> {
        self.store.files.clear();
        try_save(&self.path, &self.store).inspect_err(|e| {
            tracing::warn!("Falha ao limpar histórico: {e}");
        })
    }

    // ── Janela ────────────────────────────────────────────────────────────────

    /// Carrega o estado da janela da sessão anterior.
    pub fn load_window() -> WindowSession {
        storage::load_toml(&storage::window_path(), StoreKind::Window)
    }

    /// Persiste o estado da janela (escrita atômica em storage.rs).
    pub fn save_window(
        placement: crate::platform::window_placement::Placement,
    ) -> anyhow::Result<()> {
        let session = WindowSession {
            placement: Some(placement),
            ..WindowSession::default()
        };
        storage::save_toml(&storage::window_path(), &session, StoreKind::Window)
    }
}

fn try_load(path: &Path) -> Option<SessionStore> {
    Some(storage::load_toml(path, StoreKind::Session))
}

fn try_save(path: &Path, store: &SessionStore) -> anyhow::Result<()> {
    storage::save_toml(path, store, StoreKind::Session)
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

    #[test]
    fn clear_all_persists_an_empty_history() {
        let path = std::env::temp_dir().join(format!(
            "maripdf-clear-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after Unix epoch")
                .as_nanos()
        ));
        let mut manager = PersistenceManager {
            path: path.clone(),
            store: SessionStore::default(),
        };
        manager.save(
            Path::new(r"C:\docs.pdf"),
            FileSession {
                last_page: 1,
                zoom: 1.0,
                last_opened: 1,
            },
        );
        assert_eq!(manager.store.files.len(), 1);

        manager.clear_all().expect("clear history");

        assert!(manager.store.files.is_empty());
        let reloaded = try_load(&path).expect("reload cleared history");
        assert!(reloaded.files.is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn new_stores_carry_the_current_schema() {
        assert_eq!(
            SessionStore::default().schema_version,
            storage::SCHEMA_VERSION
        );
        assert_eq!(
            WindowSession::default().schema_version,
            storage::SCHEMA_VERSION
        );
        let text = toml::to_string(&SessionStore::default()).unwrap();
        assert!(text.contains(&format!("schema_version = {}", storage::SCHEMA_VERSION)));
    }
}
