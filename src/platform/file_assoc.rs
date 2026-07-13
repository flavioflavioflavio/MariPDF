// platform/file_assoc.rs — Associação de arquivo .pdf no Windows (HKCU)
// Registra MariPDF como handler de .pdf sem necessitar permissão de admin.

/// Registra o MariPDF como leitor padrão de .pdf no HKCU.
/// Não requer privilégios de admin — escreve apenas em HKCU.
/// Retorna Ok(()) se bem sucedido, Err com mensagem de falha.
pub fn register_pdf_association() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let exe_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "maripdf.exe".to_string());

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 1. Registra a extensão .pdf → ProgID
        let (ext_key, _) = hkcu.create_subkey(r"Software\Classes\.pdf")?;
        ext_key.set_value("", &"MariPDF.Document")?;

        // 2. Registra o ProgID com descrição
        let (prog_key, _) = hkcu.create_subkey(r"Software\Classes\MariPDF.Document")?;
        prog_key.set_value("", &"Documento PDF")?;

        // 3. Registra o ícone padrão
        let (icon_key, _) = hkcu.create_subkey(r"Software\Classes\MariPDF.Document\DefaultIcon")?;
        icon_key.set_value("", &format!("{exe_path},0"))?;

        // 4. Registra o comando de abertura
        let (cmd_key, _) =
            hkcu.create_subkey(r"Software\Classes\MariPDF.Document\shell\open\command")?;
        cmd_key.set_value("", &format!("\"{}\" \"%1\"", exe_path))?;

        // 5. Notifica o Windows para atualizar associações (sem windows_sys)
        // ie4uinit.exe -show força refresh das associações no HKCU
        let _ = std::process::Command::new("ie4uinit.exe")
            .arg("-show")
            .spawn();

        tracing::info!("Associação .pdf registrada: {exe_path}");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Associação de arquivo só suportada no Windows")
    }
}

/// Verifica se o MariPDF já está registrado como handler de .pdf.
pub fn is_pdf_associated() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(r"Software\Classes\MariPDF.Document")
            .is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Remove o registro de associação .pdf do MariPDF.
pub fn unregister_pdf_association() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(r"Software\Classes\MariPDF.Document");
        let ext = hkcu.open_subkey_with_flags(r"Software\Classes\.pdf", KEY_READ | KEY_WRITE);
        if let Ok(ext_key) = ext {
            let current: String = ext_key.get_value("").unwrap_or_default();
            if current == "MariPDF.Document" {
                let _ = ext_key.delete_value("");
            }
        }
        tracing::info!("Associação .pdf removida.");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}
