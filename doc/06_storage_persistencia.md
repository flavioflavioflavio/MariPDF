# 💾 MariPDF — Persistência de Estado

> Criado em: 2026-03-22 | Ver também: [ADR-006](adr/ADR-006_storage.md)

---

## Visão Geral

O MariPDF persiste estado local em dois arquivos TOML em `%APPDATA%\maripdf\`:

| Arquivo | Conteúdo |
|---|---|
| `config.toml` | Preferências globais (tema, idioma, janela) |
| `history.toml` | Arquivos recentes + posição por documento |

**Princípios:**
- Human-readable — editável manualmente sem consequências
- Escrita atômica — nunca corrompe em caso de crash
- Privado — nada sai do disco do usuário
- Graceful degradation — se o arquivo sumir, o app recria com defaults

---

## Estrutura de Diretórios

```
%APPDATA%\maripdf\
├── config.toml
├── history.toml
└── locales\
    └── es.ftl       (cache de idiomas baixados on-demand)
```

O caminho é obtido via crate `dirs`:
```rust
dirs::config_dir()  // → %APPDATA% no Windows
    .join("maripdf")
```

---

## `config.toml` — Schema

```toml
[ui]
theme = "light"             # "light" | "dark" | "sepia"
toolbar_autohide = true     # Toolbar some após 1.5s sem hover
language = "pt-BR"          # Locale tag (BCP 47)

[window]
width = 1024
height = 768
x = 100                     # Posição na tela (px)
y = 80
maximized = false

[reader]
default_zoom = "fit_width"  # "fit_width" | "fit_page" | "75" (porcentagem como string)
scroll_mode = "continuous"  # "continuous" | "single_page"

[meta]
first_use_date = "2026-03-22"   # Para lógica do dialog de doação
donation_shown = false
donation_shown_date = ""
version = "1.0.0"               # Versão que gerou este config (para migração)
```

---

## `history.toml` — Schema

```toml
# Lista ordenada por mais recente primeiro (máx. 20 entradas)

[[recent]]
path = "C:\\Users\\User\\Documents\\artigo.pdf"
last_opened = "2026-03-22T14:00:00Z"
page = 42
zoom = "75"
scroll_offset_y = 1240.5    # Offset em pixels da área de scroll

[[recent]]
path = "C:\\Users\\User\\Downloads\\manual.pdf"
last_opened = "2026-03-21T09:30:00Z"
page = 1
zoom = "fit_width"
scroll_offset_y = 0.0
```

**Limite:** 20 arquivos recentes. O mais antigo é removido ao adicionar o 21º.

---

## Módulo `src/storage/`

```
src/storage/
├── mod.rs          # StorageManager — API pública
├── config.rs       # ConfigStore (lê/escreve config.toml)
└── history.rs      # HistoryStore (lê/escreve history.toml)
```

### API Pública

```rust
// Inicialização (no startup, após janela abrir)
let storage = StorageManager::load()?;

// Leitura
let zoom = storage.config.reader.default_zoom;
let recents = storage.history.recent_files();

// Escrita (non-blocking — enfileira em background)
storage.save_position(pdf_path, page, scroll_offset_y);
storage.set_theme(Theme::Dark);

// Ao fechar o app — flush síncrono
storage.flush()?;
```

---

## Escrita Atômica

Para evitar corrupção em caso de crash:

```
1. Serializa conteúdo para String
2. Escreve em {arquivo}.toml.tmp
3. fs::rename(.tmp → .toml) — operação atômica no Windows
4. Deleta .tmp em caso de erro
```

---

## Migração de Versão

Se o campo `[meta].version` for diferente da versão atual do app, o `StorageManager` aplica migrações:

```rust
fn migrate(config: &mut Config, from_version: &str) {
    if semver_lt(from_version, "1.1.0") {
        // Migração de campos renomeados entre versões
    }
}
```

Isso evita panic ao ler configs de versões antigas.

---

## Privacidade

- Nenhum dado é transmitido
- `install_id` é UUID v4 gerado localmente — nunca sai do disco
- O usuário pode deletar `%APPDATA%\maripdf\` para reset completo
- Documentado no `Privacy` section do README e About dialog
