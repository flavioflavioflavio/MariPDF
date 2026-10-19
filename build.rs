// build.rs — executado pelo Cargo antes de compilar o projeto
// Embute ícone e metadados de versão no executável Windows (BF-602).

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_windows_resources();
    }
}

fn embed_windows_resources() {
    let mut res = winres::WindowsResource::new();

    // Ícone da aplicação (taskbar, Alt+Tab, Explorer, atalhos e .pdf associado).
    // Gerado por tools/make_icon.py; sem ele o build falha cedo e claro.
    assert!(
        std::path::Path::new("assets/icon.ico").exists(),
        "assets/icon.ico ausente: rode `python tools/make_icon.py`"
    );
    res.set_icon("assets/icon.ico");

    // Metadados das propriedades do .exe (FileVersion/ProductVersion vêm do
    // Cargo.toml automaticamente).
    res.set("FileDescription", "MariPDF — leitor de PDF rápido e focado");
    res.set("ProductName", "MariPDF");
    res.set("CompanyName", "Flávio Takemoto");
    res.set(
        "LegalCopyright",
        "© 2026 Flávio Takemoto — GPL-3.0-or-later",
    );
    res.set("OriginalFilename", "maripdf.exe");
    res.set("InternalName", "maripdf");
    res.set("Comments", "Dedicado a Mariana Lima, com muito amor.");

    res.compile()
        .expect("Falha ao compilar recursos Windows (build.rs)");
}
