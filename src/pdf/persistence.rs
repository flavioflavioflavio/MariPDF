// pdf/persistence.rs — Persistência de sessão por arquivo
//
// Salva: última página, zoom, modo de visualização
// Local: %APPDATA%\maripdf\session.toml

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const APP_DIR: &str = "maripdf";
const SESSION_FILE: &str = "session.toml";

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
        let mut list: Vec<(PathBuf, FileSession)> = self.store.files
            .iter()
            .filter_map(|(path_str, session)| {
                let path = PathBuf::from(path_str);
                if path.exists() { Some((path, session.clone())) } else { None }
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
}

fn session_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_DIR).join(SESSION_FILE)
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
