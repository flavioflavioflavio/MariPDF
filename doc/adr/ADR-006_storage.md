# ADR-006 — Persistência de Estado (Storage)

> **Status:** Aceita
> **Data:** 2026-03-22
> **Autores:** Flavio

---

## Contexto

O MariPDF precisa lembrar estado entre sessões: posição de leitura por documento, zoom preferido, tamanho e posição da janela, arquivos recentes, preferências de UI (tema, modo de leitura) e flags de controle (ex: `donation_dialog_shown`, `first_use_date`). A solução deve ser leve, não exigir banco de dados e ser human-readable para debugging.

---

## Opções Consideradas

### Opção A — TOML em `%APPDATA%` ✅
- ✅ Human-readable — fácil de debugar e editar manualmente
- ✅ Crate `serde` + `toml` — serialização nativa Rust, sem boilerplate
- ✅ Zero overhead — leitura na inicialização, escrita assíncrona no exit
- ✅ Alinhado com outros apps leves (Helix editor, ripgrep, etc.)
- ✅ Sem dependências de runtime (SQLite requer linking)
- ❌ Sem queries — toda leitura carrega o arquivo inteiro (aceitável dado o tamanho esperado)
- ❌ Sem transações — risco de escrita corrompida em crash (mitigado com escrita atômica)

### Opção B — SQLite (`rusqlite`)
- ✅ Queries eficientes para grandes volumes
- ✅ Transações — dados nunca corrompidos
- ❌ ~1MB de overhead no binário
- ❌ Overkill para o volume de dados esperado (~KB de preferências)
- ❌ Não é human-readable

### Opção C — Windows Registry
- ✅ API nativa do Windows
- ✅ Familiar para usuários que limpam manualmente preferências
- ❌ Fortemente acoplado ao Windows — dificulta futura portabilidade (macOS, Linux)
- ❌ API em `unsafe` complicada
- ❌ Não é human-readable

### Opção D — JSON (`serde_json`)
- ✅ Human-readable, universal
- ❌ TOML é mais legível para arquivos de configuração
- ❌ Sem distinção entre tipos de dados (timestamps viram strings)

---

## Decisão

**Escolhemos: Opção A — TOML em `%APPDATA%`**

Volume de dados minúsculo, desenvolvimento simples, debugging trivial. O TOML é mais legível que JSON para arquivos de configuração e é o padrão do ecossistema Rust (Cargo.toml).

---

## Estrutura de Arquivos

```
%APPDATA%\maripdf\
├── config.toml          # Preferências globais
├── history.toml         # Arquivos recentes + posição por documento
└── locales\             # Cache de idiomas baixados on-demand
    └── es.ftl
```

### `config.toml` — Formato

```toml
[ui]
theme = "dark"              # "light" | "dark" | "sepia"
toolbar_autohide = true
language = "pt-BR"          # locale tag

[window]
width = 1024
height = 768
x = 100
y = 80
maximized = false

[reader]
default_zoom = "fit_width"  # "fit_width" | "fit_page" | "75" (%)
scroll_mode = "continuous"  # "continuous" | "single_page"

[meta]
first_use_date = "2026-03-22"
donation_shown = false
donation_shown_date = ""
install_id = "a1b2c3d4"     # UUID v4 anônimo — só para métricas de crashes locais
```

### `history.toml` — Formato

```toml
[[recent]]
path = "C:\\Users\\User\\Documents\\artigo.pdf"
last_opened = "2026-03-22T14:00:00Z"
page = 42
zoom = "75"
scroll_offset_y = 1240.5

[[recent]]
path = "C:\\Users\\User\\Downloads\\manual.pdf"
last_opened = "2026-03-21T09:30:00Z"
page = 1
zoom = "fit_width"
scroll_offset_y = 0.0
```

---

## Estratégia de Escrita Segura

Para evitar corrupção em caso de crash durante escrita:

```rust
// Escrita atômica: escreve em .tmp → renomeia para .toml
fn write_atomic(path: &Path, content: &str) -> Result<(), StorageError> {
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)?;  // Operação atômica no Windows (mesmo volume)
    Ok(())
}
```

A escrita ocorre:
1. Ao fechar o app (flush completo)
2. A cada 30s em background thread (para não perder posição em caso de crash)
3. Ao trocar de documento

---

## Privacidade

- O `install_id` é gerado localmente, nunca transmitido
- Nenhum dado sai do disco do usuário
- O arquivo é legível e deletável pelo usuário sem consequências (app regenera com defaults)

---

## Consequências

- O módulo `src/storage/` será criado para encapsular toda leitura/escrita
- Adicionado ao `Cargo.toml`: `toml = "0.8"`, `serde = { version = "1", features = ["derive"] }`
- A estrutura do projeto é atualizada: `src/storage/mod.rs`, `src/storage/config.rs`, `src/storage/history.rs`

---

## Referências

- [crate toml](https://crates.io/crates/toml)
- [serde — serialização Rust](https://serde.rs)
- [dirs crate — caminho %APPDATA% cross-platform](https://crates.io/crates/dirs)
