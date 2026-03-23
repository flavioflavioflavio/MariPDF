# 🏗️ MariPDF — Plano de Arquitetura Técnica

> Criado em: 2026-03-22

---

## Stack Definitiva

| Camada | Tecnologia | Motivo |
|---|---|---|
| **Linguagem** | Rust | Performance, segurança de memória, binário pequeno |
| **Engine PDF** | MuPDF (via FFI bindings) | Melhor qualidade, usada no SumatraPDF |
| **UI Framework** | `egui` | Immediate mode, 60fps, 100% Rust, GPU-accelerated |
| **Renderização** | `wgpu` (backend egui) | DirectX/Vulkan/Metal, moderno |
| **Janela** | `winit` | Controle nativo Win32 |
| **Async** | `tokio` | I/O não-bloqueante (update, i18n download) |
| **Paralelo** | `rayon` | Render de páginas em paralelo |
| **i18n** | `fluent` + `sys-locale` | Mozilla Fluent, melhor que gettext |
| **Auto-update** | `self_update` | GitHub Releases, SHA-256 validado |
| **HTTP** | `reqwest` | Download de locales + check de versão |
| **Storage** | `toml` + `serde` | Persistência de estado em `%APPDATA%` |
| **Erros** | `thiserror` + `tracing` | Hierarquia de erros + log em arquivo |
| **Dirs** | `dirs` | Caminho `%APPDATA%` cross-platform |

---

## Estimativa de Binário Final

| Componente | Tamanho |
|---|---|
| Runtime Rust | ~0 (compilado estático) |
| egui + wgpu | ~3MB |
| MuPDF (FFI) | ~4MB |
| Locales embutidos (pt-BR + en) | ~50KB |
| Ícones SVG embutidos | ~100KB |
| **Total** | **~8MB** |

---

## Metas de Performance

| Métrica | Alvo |
|---|---|
| Abertura (PDF 10MB) | < 200ms |
| RAM em idle | < 30MB |
| Tamanho do installer | < 10MB |
| Busca full-text | < 500ms |
| Scroll suave | 60 FPS estável |

---

## Arquitetura de Abertura Instantânea

```
[Clique no arquivo]
       ↓
Processo inicia → Janela aparece VAZIA (< 30ms)
       ↓
Thread 1: Renderiza página 1 → exibe imediatamente
       ↓
Thread 2: Pré-renderiza páginas 2-5 em background (rayon)
       ↓
Scroll → Páginas já estão prontas no cache (LRU)
```

**Princípios:**
- Lazy loading — nunca carrega o PDF inteiro
- Tile rendering — divide páginas em tiles por demanda
- Page cache — N páginas em bitmap na RAM
- Sem checklist de plugins, sem splash screen

---

## Estrutura do Projeto

```
MariPDF/
├── Cargo.toml
├── build.rs                 # Embed ícone .ico, versão
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # Estado global
│   ├── ui/
│   │   ├── toolbar.rs       # Toolbar flutuante (Pill)
│   │   ├── viewer.rs        # Área de renderização PDF
│   │   ├── search.rs        # Overlay de busca
│   │   ├── donation.rs      # Dialog de doação
│   │   └── recent_files.rs  # Tela inicial
│   ├── pdf/
│   │   ├── engine.rs        # Wrapper MuPDF
│   │   ├── page_cache.rs    # Cache LRU de páginas
│   │   └── text_search.rs   # Full-text search
│   ├── i18n/
│   │   ├── mod.rs           # FluentBundle, hot-swap
│   │   └── loader.rs        # On-demand download + cache
│   └── updater/
│       └── mod.rs           # self_update async
├── locales/
│   ├── en.ftl               # Embutido (include_str!)
│   └── pt-BR.ftl            # Embutido
├── doc/                     # ← Esta pasta
│   └── adr/                 # Architecture Decision Records
└── .github/
    ├── FUNDING.yml
    └── workflows/
        └── release.yml      # CI/CD automático (build + SHA-256 + GitHub Release)
```

---

## i18n — Internacionalização On-Demand

### Arquivos bundled no binário
- `en.ftl` — fallback universal
- `pt-BR.ftl` — idioma padrão

### On-demand
- Outros idiomas baixados de CDN (GitHub Releases ou Cloudflare R2)
- Cache em `%APPDATA%/maripdf/locales/{lang}.ftl`
- Hot-swap sem reiniciar o app

### Formato Fluent
```fluent
nav-page = Página { $current } de { $total }
toast-opened = { $pages ->
    [one] Documento aberto — 1 página
   *[other] Documento aberto — { $pages } páginas
}
```

---

## Auto-Update

```rust
// Roda em tokio::spawn — nunca bloqueia UI
self_update::backends::github::Update::configure()
    .repo_owner("seu-usuario")
    .repo_name("MariPDF")
    .bin_name("maripdf")
    .current_version(cargo_crate_version!())
    .build()?
    .update()?;
```

- Fonte: GitHub Releases
- Validação: SHA-256 obrigatório
- UX: pill sutil na UI, sem popup intrusivo
- Aplica no próximo restart

---

## Distribuição

| Formato | Ferramenta | Público |
|---|---|---|
| Portable `.exe` | `cargo build --release` | Power users |
| Installer `.msi` | `cargo-wix` | Usuário final |
| Winget | Manifesto YAML | Distribuição ampla |

---

## Segurança — JavaScript em PDFs

MuPDF tem um interpretador JS embutido (usado em formulários PDF). Para um reader sem edição, isso é um vetor de risco desnecessário.

**Mitigação:**
- Compilar MuPDF com `#define FZ_ENABLE_JS 0` — desabilita o interpretador completamente
- Documentado no README como feature de segurança explícita
- Nenhum PDF pode executar código no MariPDF

---

## Tratamento de Erros

Ver [07_tratamento_de_erros.md](07_tratamento_de_erros.md) para o catálogo completo.

**Princípios:**
- Hierarquia de erros tipada via `thiserror` — sem `Box<dyn Error>` em APIs públicas
- Log técnico em `%APPDATA%\maripdf\maripdf.log` via `tracing`
- UI nunca exibe stack traces — apenas mensagens traduzidas via i18n
- Erros não-críticos → toast; erros críticos → dialog; erros silenciosos → log apenas

---

## Licença

### Código-fonte MariPDF
**GPLv3** — repositório público, forks devem manter open source.

### Binário distribuído
**AGPLv3** — pela incorporação estática do MuPDF (AGPLv3). Para app desktop, na prática equivale a GPLv3: o usuário recebe o código-fonte e pode modificar e redistribuir. A diferença (cláusula de servidor) não se aplica a readers de desktop.

> Ver [ADR-002](adr/ADR-002_engine_pdf.md) para análise detalhada da compatibilidade de licenças.
