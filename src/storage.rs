// storage.rs — Diretórios, versão de schema e I/O resiliente (BF-501..504).
//
// Todo arquivo de estado passa por aqui:
//   • escrita atômica: temporário no mesmo diretório → sync → rename; uma
//     queda no meio da gravação mantém a última versão válida;
//   • leitura tolerante: TOML corrompido vira `*.corrupt-<unix>.toml`, o
//     motivo vai para o log, o app segue com o padrão e o usuário é avisado;
//   • falhas de gravação viram aviso na UI em vez de sumirem em `.ok()`.

use crate::notices;
use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Versão atual do formato dos arquivos de estado.
pub const SCHEMA_VERSION: u32 = 1;

pub fn schema_version() -> u32 {
    SCHEMA_VERSION
}

/// O que cada arquivo guarda, para logs e avisos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreKind {
    Settings,
    Session,
    Window,
}

impl StoreKind {
    fn label_key(self) -> &'static str {
        match self {
            Self::Settings => "storage-what-settings",
            Self::Session => "storage-what-session",
            Self::Window => "storage-what-window",
        }
    }
}

/// `MARIPDF_DATA_DIR` redireciona todo o estado (bench, smoke test, modo
/// portátil isolado) sem tocar nas preferências reais do usuário.
fn override_dir() -> Option<PathBuf> {
    std::env::var_os("MARIPDF_DATA_DIR").map(PathBuf::from)
}

/// Preferências (roaming, acompanham o perfil do Windows).
pub fn settings_path() -> PathBuf {
    if let Some(dir) = override_dir() {
        return dir.join("settings.toml");
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MariPDF")
        .join("settings.toml")
}

/// Estado local da máquina: sessões, janela e logs.
pub fn local_dir() -> PathBuf {
    if let Some(dir) = override_dir() {
        return dir;
    }
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("maripdf")
}

pub fn session_path() -> PathBuf {
    local_dir().join("session.toml")
}

pub fn window_path() -> PathBuf {
    local_dir().join("window.toml")
}

/// Grava `bytes` em `path` sem nunca deixar o arquivo final pela metade.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "state".into());
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));

    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Arquivos que existiam mas não puderam ser lidos nesta sessão.
static UNREADABLE: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

fn protect(path: &Path) {
    let mut paths = UNREADABLE.lock().unwrap_or_else(|p| p.into_inner());
    if !paths.iter().any(|p| p == path) {
        paths.push(path.to_path_buf());
    }
}

fn is_protected(path: &Path) -> bool {
    UNREADABLE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .any(|p| p == path)
}

/// Serializa e grava; em falha registra no log e avisa o usuário.
pub fn save_toml<T: Serialize>(path: &Path, value: &T, kind: StoreKind) -> anyhow::Result<()> {
    if is_protected(path) {
        // O arquivo é preservado, mas quem chamou não pode achar que salvou
        // (ex.: "Limpar histórico" mostraria sucesso e o histórico voltaria).
        tracing::warn!(?kind, path = %path.display(), "estado não lido nesta sessão; gravação suspensa");
        anyhow::bail!(
            "{} não pôde ser lido nesta sessão; gravação suspensa para não sobrescrevê-lo",
            path.display()
        );
    }
    let result = toml::to_string_pretty(value)
        .map_err(anyhow::Error::from)
        .and_then(|text| write_atomic(path, text.as_bytes()).map_err(anyhow::Error::from));
    if let Err(error) = &result {
        tracing::warn!(?kind, path = %path.display(), "falha ao salvar estado: {error}");
        notices::error(crate::t!(
            "storage-save-failed",
            what => crate::t!(kind.label_key()),
            error => error
        ));
    }
    result
}

/// Lê um TOML. Ausente → padrão. Corrompido → backup + padrão + aviso.
pub fn load_toml<T: DeserializeOwned + Default>(path: &Path, kind: StoreKind) -> T {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return T::default(),
        Err(error) => {
            // Existe mas não pôde ser lido (ACL, bloqueio): avisa e protege o
            // arquivo para que os padrões desta sessão não o sobrescrevam.
            tracing::warn!(?kind, path = %path.display(), "falha ao ler estado: {error}");
            protect(path);
            notices::error(crate::t!(
                "storage-read-failed",
                what => crate::t!(kind.label_key()),
                error => error
            ));
            return T::default();
        }
    };
    match toml::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            let backup = quarantine(path);
            tracing::warn!(
                ?kind,
                path = %path.display(),
                backup = ?backup,
                "estado corrompido, usando padrão: {error}"
            );
            notices::info(crate::t!(
                "storage-recovered",
                what => crate::t!(kind.label_key())
            ));
            T::default()
        }
    }
}

/// Move o arquivo ilegível para um backup datado ao lado do original.
fn quarantine(path: &Path) -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let backup = path.with_file_name(format!("{stem}.corrupt-{stamp}.toml"));
    std::fs::rename(path, &backup).ok().map(|_| backup)
}

#[cfg(test)]
pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    let unique = format!(
        "maripdf-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("relógio depois de 1970")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&dir).expect("criar diretório temporário");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        #[serde(default = "schema_version")]
        schema_version: u32,
        value: u32,
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp_files() {
        let dir = temp_dir("atomic");
        let path = dir.join("state.toml");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1, "só o arquivo final deve existir");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn failed_write_keeps_previous_file_and_reports() {
        let _guard = notices::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        notices::drain();
        let dir = temp_dir("atomic-fail");
        // Um diretório com o nome do arquivo final impede o rename.
        let path = dir.join("state.toml");
        std::fs::create_dir_all(&path).unwrap();
        let result = save_toml(
            &path,
            &Sample {
                schema_version: 1,
                value: 7,
            },
            StoreKind::Settings,
        );
        assert!(result.is_err());
        assert!(path.is_dir(), "o alvo original não pode ser destruído");
        let notices = notices::drain();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, notices::NoticeKind::Error);
        let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "temporário deve ser removido");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupted_toml_is_quarantined_and_defaults_are_used() {
        let _guard = notices::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        notices::drain();
        let dir = temp_dir("corrupt");
        let path = dir.join("settings.toml");
        std::fs::write(&path, "value = [unterminated").unwrap();

        let loaded: Sample = load_toml(&path, StoreKind::Settings);
        assert_eq!(loaded, Sample::default());
        assert!(!path.exists(), "o arquivo ruim sai do caminho");
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(backups.len(), 1);
        assert!(backups[0].starts_with("settings.corrupt-"), "{backups:?}");
        let notices = notices::drain();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, notices::NoticeKind::Info);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_is_silent_default() {
        let _guard = notices::TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        notices::drain();
        let dir = temp_dir("missing");
        let loaded: Sample = load_toml(&dir.join("nope.toml"), StoreKind::Session);
        assert_eq!(loaded, Sample::default());
        assert!(notices::drain().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn round_trip_keeps_schema_version() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("state.toml");
        save_toml(
            &path,
            &Sample {
                schema_version: SCHEMA_VERSION,
                value: 42,
            },
            StoreKind::Window,
        )
        .unwrap();
        let loaded: Sample = load_toml(&path, StoreKind::Window);
        assert_eq!(loaded.value, 42);
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
        let _ = std::fs::remove_dir_all(dir);
    }
}
