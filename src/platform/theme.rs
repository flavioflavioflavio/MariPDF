// platform/theme.rs — Detecta tema do Windows (Dark / Light) via registro
//
// HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize
//   AppsUseLightTheme = 0 → Dark
//   AppsUseLightTheme = 1 → Light (default quando não existe)

#[cfg(target_os = "windows")]
pub fn system_prefers_dark() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key_path = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(key_path) {
        let val: u32 = hkcu.get_value("AppsUseLightTheme").unwrap_or(1);
        return val == 0;
    }
    false // fallback: considera Light
}

#[cfg(not(target_os = "windows"))]
pub fn system_prefers_dark() -> bool {
    false // non-Windows: assume Light
}
