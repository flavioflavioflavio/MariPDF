// platform/file_assoc.rs — Associação .pdf no Windows 10/11, por usuário (BF-604).
//
// Desde o Windows 8 o padrão de cada extensão fica em `UserChoice`, protegido
// por hash: nenhum app pode se tornar padrão sozinho. O caminho correto é:
//   1. registrar ProgID, OpenWithProgids, Capabilities e RegisteredApplications
//      (HKCU, sem admin) para o MariPDF aparecer em "Abrir com" e nas
//      Configurações;
//   2. abrir "Aplicativos padrão" já na página do MariPDF para o usuário
//      confirmar com um clique.
// O instalador escreve exatamente as mesmas chaves (installer/maripdf.iss).

pub const PROG_ID: &str = "MariPDF.Document";
const APP_KEY: &str = r"Software\MariPDF";
const CAPABILITIES_KEY: &str = r"Software\MariPDF\Capabilities";
const REGISTERED_APP_NAME: &str = "MariPDF";

/// Uma entrada de registro em HKCU: caminho, nome do valor ("" = padrão) e dado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegEntry {
    pub key: String,
    pub name: String,
    pub value: String,
}

fn entry(key: impl Into<String>, name: &str, value: impl Into<String>) -> RegEntry {
    RegEntry {
        key: key.into(),
        name: name.into(),
        value: value.into(),
    }
}

/// Chaves que registram o MariPDF como leitor de PDF para o usuário atual.
/// Não toca no valor padrão de `.pdf` nem em `UserChoice`.
pub fn association_entries(exe_path: &str, type_name: &str, description: &str) -> Vec<RegEntry> {
    let prog = format!(r"Software\Classes\{PROG_ID}");
    let open_command = format!("\"{exe_path}\" \"%1\"");
    let icon = format!("{exe_path},0");
    vec![
        entry(&prog, "", type_name),
        entry(&prog, "FriendlyTypeName", type_name),
        entry(format!(r"{prog}\DefaultIcon"), "", &icon),
        entry(format!(r"{prog}\shell\open\command"), "", &open_command),
        entry(r"Software\Classes\.pdf\OpenWithProgids", PROG_ID, ""),
        entry(
            r"Software\Classes\Applications\maripdf.exe\shell\open\command",
            "",
            &open_command,
        ),
        entry(
            r"Software\Classes\Applications\maripdf.exe\SupportedTypes",
            ".pdf",
            "",
        ),
        entry(CAPABILITIES_KEY, "ApplicationName", "MariPDF"),
        entry(CAPABILITIES_KEY, "ApplicationDescription", description),
        entry(CAPABILITIES_KEY, "ApplicationIcon", &icon),
        entry(
            format!(r"{CAPABILITIES_KEY}\FileAssociations"),
            ".pdf",
            PROG_ID,
        ),
        entry(
            r"Software\RegisteredApplications",
            REGISTERED_APP_NAME,
            CAPABILITIES_KEY,
        ),
    ]
}

/// Página de "Aplicativos padrão" do Windows já filtrada no MariPDF.
pub fn default_apps_uri() -> String {
    format!("ms-settings:defaultapps?registeredAppUser={REGISTERED_APP_NAME}")
}

/// Abre a tela de apps padrão do Windows já filtrada no MariPDF.
pub fn open_default_apps() {
    if let Err(error) = open::that(default_apps_uri()) {
        tracing::warn!("associação .pdf: falha ao abrir apps padrão: {error}");
    }
}

/// Registra o MariPDF e abre as Configurações para o usuário confirmar.
pub fn register_pdf_association() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let exe_path = std::env::current_exe()?.to_string_lossy().into_owned();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for e in association_entries(
            &exe_path,
            &crate::t!("file-type-name"),
            &crate::t!("about-desc"),
        ) {
            let (key, _) = hkcu.create_subkey(&e.key)?;
            key.set_value(&e.name, &e.value)?;
        }
        notify_association_changed();
        tracing::info!("associação .pdf registrada para {exe_path}");
        let _ = open::that(default_apps_uri());
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Associação de arquivo só suportada no Windows")
    }
}

/// Remove todo o registro feito por `register_pdf_association`.
pub fn unregister_pdf_association() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        // Ausente = já removido; qualquer outro erro (ACL, etc.) é reportado.
        let tolerate_missing = |result: std::io::Result<()>| match result {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
        tolerate_missing(hkcu.delete_subkey_all(format!(r"Software\Classes\{PROG_ID}")))?;
        tolerate_missing(hkcu.delete_subkey_all(r"Software\Classes\Applications\maripdf.exe"))?;
        tolerate_missing(hkcu.delete_subkey_all(APP_KEY))?;
        for (key, value) in [
            (r"Software\Classes\.pdf\OpenWithProgids", PROG_ID),
            (r"Software\RegisteredApplications", REGISTERED_APP_NAME),
        ] {
            match hkcu.open_subkey_with_flags(key, KEY_READ | KEY_WRITE) {
                Ok(k) => tolerate_missing(k.delete_value(value))?,
                Err(error) => tolerate_missing(Err(error))?,
            }
        }
        // Versões antigas gravavam o padrão legado de `.pdf`; limpa se for nosso.
        if let Ok(ext) = hkcu.open_subkey_with_flags(r"Software\Classes\.pdf", KEY_READ | KEY_WRITE)
        {
            let current: String = ext.get_value("").unwrap_or_default();
            if current == PROG_ID {
                let _ = ext.delete_value("");
            }
        }
        notify_association_changed();
        tracing::info!("associação .pdf removida");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// Estado real da associação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationState {
    /// O usuário escolheu o MariPDF como padrão (UserChoice).
    Default,
    /// Registrado como opção, mas o padrão ainda é outro app.
    Registered,
    NotRegistered,
}

pub fn association_state() -> AssociationState {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let user_choice: String = hkcu
            .open_subkey(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\UserChoice",
            )
            .and_then(|k| k.get_value("ProgId"))
            .unwrap_or_default();
        if user_choice == PROG_ID {
            AssociationState::Default
        } else if hkcu.open_subkey(CAPABILITIES_KEY).is_ok() {
            AssociationState::Registered
        } else {
            AssociationState::NotRegistered
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        AssociationState::NotRegistered
    }
}

/// Avisa o Explorer que associações mudaram (ícones e "Abrir com").
#[cfg(target_os = "windows")]
fn notify_association_changed() {
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0x0000;
    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            event_id: i32,
            flags: u32,
            item1: *const std::ffi::c_void,
            item2: *const std::ffi::c_void,
        );
    }
    // SAFETY: notificação global sem ponteiros de item.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<RegEntry> {
        association_entries(r"C:\Apps\MariPDF\maripdf.exe", "Documento PDF", "Leitor")
    }

    #[test]
    fn never_overrides_the_legacy_pdf_default_or_user_choice() {
        for e in entries() {
            assert!(
                !(e.key == r"Software\Classes\.pdf" && e.name.is_empty()),
                "não deve sobrescrever o padrão legado de .pdf"
            );
            assert!(!e.key.contains("UserChoice"));
        }
    }

    #[test]
    fn registers_progid_capabilities_and_open_with() {
        let all = entries();
        let has = |key: &str, name: &str, value: &str| {
            all.iter()
                .any(|e| e.key == key && e.name == name && e.value == value)
        };
        assert!(has(
            r"Software\Classes\MariPDF.Document\shell\open\command",
            "",
            r#""C:\Apps\MariPDF\maripdf.exe" "%1""#
        ));
        assert!(has(r"Software\Classes\.pdf\OpenWithProgids", PROG_ID, ""));
        assert!(has(
            r"Software\MariPDF\Capabilities\FileAssociations",
            ".pdf",
            PROG_ID
        ));
        assert!(has(
            r"Software\RegisteredApplications",
            "MariPDF",
            r"Software\MariPDF\Capabilities"
        ));
    }

    #[test]
    fn settings_uri_targets_the_registered_app() {
        assert_eq!(
            default_apps_uri(),
            "ms-settings:defaultapps?registeredAppUser=MariPDF"
        );
    }
}
