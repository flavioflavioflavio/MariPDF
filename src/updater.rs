// updater.rs — Atualização manual e verificada (BF-606).
//
// Princípios (MANIFESTO: nada de telemetria):
//   • só consulta a rede quando o usuário clica em "Verificar atualizações";
//   • o instalador baixado só roda se SHA256SUMS.txt tiver assinatura
//     Ed25519 válida pela chave pública embutida E o hash do instalador
//     bater com a lista assinada (independe de certificado da Microsoft);
//   • rollout manual: a versão é oferecida, o usuário decide; o instalador
//     desfaz a instalação se falhar e bloqueia downgrade.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

const RELEASES_URL: &str =
    "https://api.github.com/repos/flavioflavioflavio/MariPDF/releases?per_page=30";
/// A lista é paginada: a estável mais recente é consultada à parte para não
/// sumir atrás de uma sequência de pré-releases.
const LATEST_STABLE_URL: &str =
    "https://api.github.com/repos/flavioflavioflavio/MariPDF/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/flavioflavioflavio/MariPDF/releases";
const SUMS_NAME: &str = "SHA256SUMS.txt";
const SIG_NAME: &str = "SHA256SUMS.txt.sig";

/// Chave pública de assinatura das releases. A privada fica fora do
/// repositório (ver doc/10_distribuicao.md).
const RELEASE_PUBLIC_KEY: [u8; 32] = [
    0xbb, 0x02, 0x53, 0x00, 0xb1, 0x57, 0xd0, 0x3b, 0x37, 0xf2, 0x97, 0xc5, 0x9d, 0xfc, 0x9c, 0x78,
    0x3d, 0x2d, 0x12, 0x95, 0xcc, 0xe4, 0xbf, 0x8f, 0x09, 0x9c, 0xdc, 0x39, 0x89, 0x0a, 0xdf, 0x5e,
];

// ── Versões ──────────────────────────────────────────────────────────────────

/// Versão semântica mínima: núcleo numérico + pré-release opcional.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    pre: Option<String>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre.to_string())),
            None => (text, None),
        };
        let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
        let core = [
            parts.next()??,
            parts.next().unwrap_or(Some(0))?,
            parts.next().unwrap_or(Some(0))?,
        ];
        Some(Self { core, pre })
    }

    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        self.core
            .cmp(&other.core)
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => compare_prerelease(a, b),
            })
    }
}

/// Identificadores separados por ponto; numéricos comparam como número.
fn compare_prerelease(a: &str, b: &str) -> std::cmp::Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                // SemVer §11: numérico compara como número e sempre
                // precede um identificador alfanumérico.
                let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("versão do Cargo.toml válida")
}

// ── Releases ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn version(&self) -> Option<Version> {
        Version::parse(&self.tag_name)
    }

    fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }

    pub fn setup_asset(&self) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|a| a.name.starts_with("MariPDF-") && a.name.ends_with("-setup.exe"))
    }
}

/// Melhor release mais nova que `current`. Pré-releases só entram para quem
/// já está numa pré-release (canal de testes); rascunhos nunca.
pub fn select_update(current: &Version, releases: &[Release]) -> Option<Release> {
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter(|r| current.is_prerelease() || !r.prerelease)
        .filter_map(|r| r.version().map(|v| (v, r)))
        .filter(|(v, _)| v > current)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, r)| r.clone())
}

// ── Verificação ──────────────────────────────────────────────────────────────

/// Confere a assinatura Ed25519 (base64) de `message`.
pub fn verify_signature(public_key: &[u8; 32], message: &[u8], signature_b64: &str) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(signature_b64.trim()) else {
        return false;
    };
    let Ok(bytes) = <[u8; 64]>::try_from(raw.as_slice()) else {
        return false;
    };
    key.verify(message, &Signature::from_bytes(&bytes)).is_ok()
}

/// Hash esperado de `file_name` dentro de um SHA256SUMS ("<hex>  <nome>").
pub fn expected_hash(sums: &str, file_name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        (name.trim().trim_start_matches('*') == file_name).then(|| hash.trim().to_lowercase())
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Verifica a cadeia completa: assinatura da lista e hash do instalador.
pub fn verify_download(
    public_key: &[u8; 32],
    sums: &str,
    signature_b64: &str,
    file_name: &str,
    file: &[u8],
) -> Result<(), UpdateError> {
    if !verify_signature(public_key, sums.as_bytes(), signature_b64) {
        return Err(UpdateError::BadSignature);
    }
    let expected = expected_hash(sums, file_name).ok_or(UpdateError::NotListed)?;
    if sha256_hex(file) != expected {
        return Err(UpdateError::HashMismatch);
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateError {
    Network(String),
    NoInstaller,
    BadSignature,
    NotListed,
    HashMismatch,
    Io(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = match self {
            Self::Network(_) => "update-err-network",
            Self::NoInstaller => "update-err-no-installer",
            Self::BadSignature => "update-err-signature",
            Self::NotListed | Self::HashMismatch => "update-err-hash",
            Self::Io(_) => "update-err-io",
        };
        write!(f, "{}", crate::t!(key))
    }
}

// ── Rede (só por ação do usuário) ────────────────────────────────────────────

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(concat!("MariPDF/", env!("CARGO_PKG_VERSION")))
        .build()
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>, UpdateError> {
    let response = agent()
        .get(url)
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut response.into_reader().take(300 * 1024 * 1024),
        &mut bytes,
    )
    .map_err(|e| UpdateError::Network(e.to_string()))?;
    Ok(bytes)
}

use std::io::Read as _;

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, UpdateError> {
    agent()
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?
        .into_json()
        .map_err(|e| UpdateError::Network(e.to_string()))
}

fn check() -> Result<Option<Release>, UpdateError> {
    let mut releases: Vec<Release> = fetch_json(RELEASES_URL)?;
    match fetch_json::<Release>(LATEST_STABLE_URL) {
        Ok(latest) => releases.push(latest),
        // 404 = ainda não há release estável publicada.
        Err(UpdateError::Network(error)) if error.contains("404") => {}
        Err(error) => return Err(error),
    }
    Ok(select_update(&current_version(), &releases))
}

/// Baixa, verifica e devolve o caminho do instalador pronto para rodar.
fn download(release: &Release) -> Result<PathBuf, UpdateError> {
    let setup = release.setup_asset().ok_or(UpdateError::NoInstaller)?;
    let sums_asset = release.asset(SUMS_NAME).ok_or(UpdateError::NotListed)?;
    let sig_asset = release.asset(SIG_NAME).ok_or(UpdateError::BadSignature)?;

    let sums = String::from_utf8(fetch_bytes(&sums_asset.browser_download_url)?)
        .map_err(|_| UpdateError::NotListed)?;
    let signature = String::from_utf8(fetch_bytes(&sig_asset.browser_download_url)?)
        .map_err(|_| UpdateError::BadSignature)?;
    let installer = fetch_bytes(&setup.browser_download_url)?;
    verify_download(
        &RELEASE_PUBLIC_KEY,
        &sums,
        &signature,
        &setup.name,
        &installer,
    )?;

    let dir = crate::storage::local_dir()
        .join("updates")
        .join(&release.tag_name);
    let path = dir.join(&setup.name);
    crate::storage::write_atomic(&path, &installer).map_err(|e| UpdateError::Io(e.to_string()))?;
    tracing::info!("update: {} verificado em {}", setup.name, path.display());
    Ok(path)
}

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{73F72B7B-8E4F-4704-8CBA-9FB568284922}_is1";

/// Instalado pelo instalador? Usa a pasta registrada pelo Inno Setup (o
/// usuário pode ter escolhido outra), não só a pasta padrão.
pub fn is_installed_copy() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    installed_location().is_some_and(|dir| is_inside(&exe, &dir))
}

#[cfg(target_os = "windows")]
fn installed_location() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(UNINSTALL_KEY)
        .ok()?;
    let location: String = key.get_value("InstallLocation").ok()?;
    Some(PathBuf::from(location))
}

#[cfg(not(target_os = "windows"))]
fn installed_location() -> Option<PathBuf> {
    None
}

/// Comparação de caminhos sem diferenciar maiúsculas (Windows).
fn is_inside(exe: &Path, dir: &Path) -> bool {
    let normalize = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_lowercase()
            .replace('/', "\\")
    };
    let (exe, dir) = (normalize(exe), normalize(dir));
    exe.starts_with(&dir) && exe[dir.len()..].starts_with('\\')
}

/// Inicia o instalador verificado; o app fecha logo em seguida.
pub fn launch_installer(path: &Path) -> Result<(), UpdateError> {
    std::process::Command::new(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| UpdateError::Io(e.to_string()))
}

// ── Estado para a UI ─────────────────────────────────────────────────────────

#[derive(Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking(Receiver<Result<Option<Release>, UpdateError>>),
    UpToDate,
    Available(Release),
    Downloading(Release, Receiver<Result<PathBuf, UpdateError>>),
    Ready(PathBuf),
    Failed(String),
}

impl UpdateState {
    pub fn start_check(&mut self, ctx: &eframe::egui::Context) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(check());
            ctx.request_repaint();
        });
        *self = Self::Checking(rx);
    }

    pub fn start_download(&mut self, release: Release, ctx: &eframe::egui::Context) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let job = release.clone();
        std::thread::spawn(move || {
            let _ = tx.send(download(&job));
            ctx.request_repaint();
        });
        *self = Self::Downloading(release, rx);
    }

    /// Avança estados assíncronos; chamado a cada frame com o diálogo aberto.
    pub fn poll(&mut self) {
        let next = match self {
            Self::Checking(rx) => match rx.try_recv() {
                Ok(Ok(Some(release))) => Some(Self::Available(release)),
                Ok(Ok(None)) => Some(Self::UpToDate),
                Ok(Err(error)) => Some(Self::Failed(error.to_string())),
                Err(_) => None,
            },
            Self::Downloading(_, rx) => match rx.try_recv() {
                Ok(Ok(path)) => Some(Self::Ready(path)),
                Ok(Err(error)) => Some(Self::Failed(error.to_string())),
                Err(_) => None,
            },
            _ => None,
        };
        if let Some(next) = next {
            *self = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn release(tag: &str, prerelease: bool) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("{RELEASES_PAGE}/tag/{tag}"),
            draft: false,
            prerelease,
            assets: Vec::new(),
        }
    }

    #[test]
    fn versions_order_like_semver() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("1.0.0") > v("1.0.0-rc.2"));
        assert!(v("1.0.0-rc.10") > v("1.0.0-rc.2"));
        assert!(v("v1.2.0") > v("1.1.9"));
        assert!(v("0.1.0") < v("1.0.0-rc.1"));
        assert_eq!(v("1.0"), v("1.0.0"));
        assert!(Version::parse("x.y").is_none());
    }

    #[test]
    fn numeric_prerelease_identifiers_precede_alphanumeric_ones() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("1.0.0-2") < v("1.0.0-1a"));
        assert!(v("1.0.0-rc.9") < v("1.0.0-rc.a"));
        assert!(v("1.0.0-alpha") < v("1.0.0-beta"));
    }

    #[test]
    fn stable_users_only_get_stable_releases() {
        let current = Version::parse("1.0.0").unwrap();
        let releases = [release("v1.1.0-rc.1", true), release("v1.0.1", false)];
        assert_eq!(
            select_update(&current, &releases).unwrap().tag_name,
            "v1.0.1"
        );
    }

    #[test]
    fn prerelease_users_get_the_newest_including_prereleases() {
        let current = Version::parse("1.0.0-rc.1").unwrap();
        let releases = [release("v1.0.0-rc.2", true), release("v0.9.0", false)];
        assert_eq!(
            select_update(&current, &releases).unwrap().tag_name,
            "v1.0.0-rc.2"
        );
    }

    #[test]
    fn stable_found_outside_the_first_page_still_wins() {
        // A lista só trouxe pré-releases; a estável veio do /releases/latest.
        let mut releases: Vec<Release> = (0..30)
            .map(|i| release(&format!("v1.2.0-rc.{i}"), true))
            .collect();
        releases.push(release("v1.1.0", false));
        let current = Version::parse("1.0.0").unwrap();
        let chosen = select_update(&current, &releases).expect("estável disponível");
        assert_eq!(chosen.tag_name, "v1.1.0");
    }

    #[test]
    fn no_update_when_current_is_newest_or_drafts_only() {
        let current = Version::parse("1.0.0").unwrap();
        let mut draft = release("v2.0.0", false);
        draft.draft = true;
        assert!(select_update(&current, &[release("v1.0.0", false), draft]).is_none());
    }

    fn signed(sums: &str) -> ([u8; 32], String) {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let signature = key.sign(sums.as_bytes());
        (
            key.verifying_key().to_bytes(),
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
        )
    }

    #[test]
    fn verified_download_requires_signature_and_matching_hash() {
        let installer = b"instalador de teste";
        let sums = format!("{}  MariPDF-1.0.0-setup.exe\n", sha256_hex(installer));
        let (public, sig) = signed(&sums);

        assert_eq!(
            verify_download(&public, &sums, &sig, "MariPDF-1.0.0-setup.exe", installer),
            Ok(())
        );
        // Instalador trocado.
        assert_eq!(
            verify_download(
                &public,
                &sums,
                &sig,
                "MariPDF-1.0.0-setup.exe",
                b"malicioso"
            ),
            Err(UpdateError::HashMismatch)
        );
        // Lista adulterada (hash do instalador malicioso) com assinatura antiga.
        let forged = format!("{}  MariPDF-1.0.0-setup.exe\n", sha256_hex(b"malicioso"));
        assert_eq!(
            verify_download(
                &public,
                &forged,
                &sig,
                "MariPDF-1.0.0-setup.exe",
                b"malicioso"
            ),
            Err(UpdateError::BadSignature)
        );
        // Arquivo fora da lista.
        assert_eq!(
            verify_download(&public, &sums, &sig, "outro.exe", installer),
            Err(UpdateError::NotListed)
        );
    }

    #[test]
    fn embedded_key_rejects_signatures_from_other_keys() {
        let sums = "abc  MariPDF-1.0.0-setup.exe\n";
        let (_, sig) = signed(sums);
        assert!(!verify_signature(
            &RELEASE_PUBLIC_KEY,
            sums.as_bytes(),
            &sig
        ));
    }

    /// Vetor assinado com a chave REAL de release por tools/sign_release.py:
    /// prova que a ferramenta e a chave pública embutida no app combinam.
    #[test]
    fn embedded_key_accepts_the_release_tool_signature() {
        let message = include_bytes!("../tests/fixtures/update_vector.txt");
        let signature = include_str!("../tests/fixtures/update_vector.txt.sig");
        assert!(verify_signature(&RELEASE_PUBLIC_KEY, message, signature));
        assert!(!verify_signature(
            &RELEASE_PUBLIC_KEY,
            b"adulterado",
            signature
        ));
    }

    #[test]
    fn installed_location_match_is_case_insensitive_and_exact() {
        let dir = Path::new(r"D:\Apps\MariPDF\");
        assert!(is_inside(Path::new(r"d:\apps\maripdf\maripdf.exe"), dir));
        assert!(!is_inside(Path::new(r"D:\Apps\MariPDF2\maripdf.exe"), dir));
        assert!(!is_inside(Path::new(r"C:\Downloads\maripdf.exe"), dir));
    }

    #[test]
    fn parses_sha256sums_with_binary_marker() {
        let sums = "AA11  MariPDF-1.0.0-portable.exe\nbb22 *MariPDF-1.0.0-setup.exe\n";
        assert_eq!(
            expected_hash(sums, "MariPDF-1.0.0-setup.exe").as_deref(),
            Some("bb22")
        );
        assert_eq!(
            expected_hash(sums, "MariPDF-1.0.0-portable.exe").as_deref(),
            Some("aa11")
        );
    }
}
