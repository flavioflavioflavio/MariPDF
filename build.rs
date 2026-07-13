// build.rs — executado pelo Cargo antes de compilar o projeto
// Embute ícone .ico e metadados de versão no executável Windows

fn main() {
    // Só embute recursos no Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_windows_resources();
    }
}

fn embed_windows_resources() {
    let mut res = winres::WindowsResource::new();

    // Ícone principal da aplicação (taskbar + Alt+Tab + explorador)
    // O arquivo assets/icon.ico deve existir antes do build
    if std::path::Path::new("assets/icon.ico").exists() {
        res.set_icon("assets/icon.ico");
    }

    // Metadados exibidos nas propriedades do .exe
    res.set("FileDescription", "MariPDF — Leitor de PDF Ultra-Leve");
    res.set("ProductName", "MariPDF");
    res.set("LegalCopyright", "MariPDF Contributors — GPLv3");

    // Compila e linka os recursos
    res.compile()
        .expect("Falha ao compilar recursos Windows (build.rs)");
}
