# 🤝 Contribuindo com o MariPDF

Obrigado por considerar contribuir! MariPDF é um projeto small-team — cada contribuição conta.

> **TL;DR:** Fork → branch → PR pequeno → CI verde → revisão → merge.

---

## 📋 Índice

- [Código de Conduta](#-código-de-conduta)
- [Formas de Contribuir](#-formas-de-contribuir)
- [Setup do Ambiente](#-setup-do-ambiente)
- [Estrutura do Projeto](#-estrutura-do-projeto)
- [Fluxo de Desenvolvimento](#-fluxo-de-desenvolvimento)
- [Convenções de Código](#-convenções-de-código)
- [Convenções de Commit](#-convenções-de-commit)
- [Abrindo um PR](#-abrindo-um-pr)
- [Reportando Bugs](#-reportando-bugs)

---

## 🧭 Código de Conduta

Seja respeitoso. Sem discriminação. Foco no código e nas ideias, não nas pessoas.  
Em caso de conflito, a decisão final é do mantenedor principal.

---

## 💡 Formas de Contribuir

| Tipo | Onde |
|---|---|
| 🐛 Bug report | [Issues](../../issues) — use o template |
| 💬 Feature request | [Discussions](../../discussions) |
| 🌍 Nova tradução (`.ftl`) | PR com arquivo em `locales/` |
| 🧪 Testes | Código em `src/**/tests/` |
| 🔧 Bug fix / feature | Fork + PR (veja abaixo) |

> **Dica:** Para mudanças grandes, abra uma Discussion antes para validar a ideia. PRs de arquitetura sem acordo prévio tendem a ser recusados.

---

## 🛠️ Setup do Ambiente

### Requisitos

| Ferramenta | Versão mínima | Como instalar |
|---|---|---|
| Rust | stable (≥ 1.75) | [rustup.rs](https://rustup.rs) |
| MSVC Build Tools | VS 2022 | [visualstudio.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/) |
| Git | qualquer recente | [git-scm.com](https://git-scm.com) |

> O **MSVC toolchain** é obrigatório no Windows para compilar as dependências C (MuPDF via FFI).

### Clonar e buildar

```bash
git clone https://github.com/[seu-usuario]/MariPDF
cd MariPDF

# Build de desenvolvimento (mais rápido, sem otimizações)
cargo build

# Build de release (binário final otimizado)
cargo build --release

# Executar direto
cargo run

# Rodar os testes
cargo test
```

### Verificar que está funcionando

```bash
# Deve abrir uma janela do MariPDF
cargo run -- caminho/para/arquivo.pdf
```

---

## 📁 Estrutura do Projeto

```
MariPDF/
├── src/
│   ├── main.rs              # Entry point — mínimo possível
│   ├── app.rs               # Estado global da aplicação
│   ├── ui/
│   │   ├── toolbar.rs       # Toolbar flutuante (Pill)
│   │   ├── viewer.rs        # Área de renderização PDF
│   │   ├── search.rs        # Overlay de busca (Ctrl+F)
│   │   ├── donation.rs      # Dialog de doação
│   │   └── recent_files.rs  # Tela inicial com recentes
│   ├── pdf/
│   │   ├── engine.rs        # Wrapper seguro sobre MuPDF (FFI)
│   │   ├── page_cache.rs    # Cache LRU de páginas renderizadas
│   │   └── text_search.rs   # Busca full-text
│   ├── i18n/
│   │   ├── mod.rs           # FluentBundle, hot-swap de locale
│   │   └── loader.rs        # Download on-demand + cache local
│   └── updater/
│       └── mod.rs           # Auto-update via GitHub Releases
├── locales/
│   ├── en.ftl               # Inglês (bundled no binário)
│   └── pt-BR.ftl            # Português BR (bundled no binário)
├── doc/                     # Documentação do projeto
│   └── adr/                 # Architecture Decision Records
├── .github/
│   ├── FUNDING.yml
│   └── workflows/
│       └── release.yml      # CI/CD de release
├── build.rs                 # Embed ícone e versão no binário
└── Cargo.toml
```

**Regras de arquitetura:**
- Um arquivo = uma responsabilidade (~100–150 linhas máximo)
- Lógica de negócio fica em `pdf/` e `i18n/` — **nunca** em `ui/`
- `unsafe` blocks só em `pdf/engine.rs` — documentar cada um
- Sem `unwrap()` em código de produção — use `?` ou trate o erro

---

## 🔄 Fluxo de Desenvolvimento

```
main (branch protegida)
  └── feat/nome-da-feature   ← seu PR vai aqui
  └── fix/numero-da-issue
  └── docs/o-que-muda
```

1. **Fork** o repositório
2. **Crie** uma branch a partir de `main`: `git checkout -b feat/minha-feature`
3. **Faça** commits pequenos e descritivos (veja Convenções de Commit)
4. **Rode** `cargo test` e `cargo clippy` antes de fazer push
5. **Abra** o PR contra `main`

---

## 📐 Convenções de Código

### Geral
- Siga o que `cargo fmt` e `cargo clippy` determinam — sem exceções
- Nomes em **snake_case** para funções e variáveis, **PascalCase** para structs/enums
- Strings visíveis ao usuário sempre via i18n — **nunca** hardcoded
- **Sem magic numbers** — use constantes nomeadas

### Comentários
- Comente o **porquê**, não o **o quê** (o código já diz o quê)
- Blocos `unsafe` **obrigam** comentário explicando o que é garantido pelo chamador

### Exemplo

```rust
// ✅ Correto
const MAX_CACHED_PAGES: usize = 10;

/// Returns the rendered bitmap for `page_number`, fetching from cache if available.
pub fn get_page(&mut self, page_number: usize) -> Result<PageBitmap, PdfError> {
    // We check cache first to avoid re-rendering — MuPDF render is ~15ms per page
    if let Some(cached) = self.cache.get(&page_number) {
        return Ok(cached.clone());
    }
    self.render_and_cache(page_number)
}

// ❌ Errado
pub fn gp(&mut self, n: usize) -> Option<PageBitmap> {
    // get page
    self.cache.get(&n).cloned()
}
```

---

## 📝 Convenções de Commit

Usamos **Conventional Commits**:

```
<tipo>(<escopo>): <descrição imperativa em minúsculas>

[corpo opcional — o porquê, não o quê]

[footer opcional — refs, breaking changes]
```

| Tipo | Quando usar |
|---|---|
| `feat` | Nova funcionalidade |
| `fix` | Correção de bug |
| `perf` | Melhoria de performance |
| `docs` | Somente documentação |
| `refactor` | Refatoração sem mudança de comportamento |
| `test` | Adição ou correção de testes |
| `chore` | Build, CI, deps — sem código de produto |
| `i18n` | Adição ou correção de traduções |

**Exemplos:**
```
feat(viewer): add continuous scroll mode
fix(cache): fix LRU eviction off-by-one on page count
perf(engine): render page 1 on separate thread for instant open
docs(adr): add ADR-003 for UI framework decision
i18n(pt-BR): add missing toast strings
```

---

## 🔀 Abrindo um PR

### Checklist antes de submeter

- [ ] `cargo fmt` aplicado
- [ ] `cargo clippy -- -D warnings` sem erros
- [ ] `cargo test` passando
- [ ] Strings novas adicionadas aos dois `.ftl` (`en` e `pt-BR`)
- [ ] Sem `unwrap()` ou `.expect()` sem comentário justificando
- [ ] PR pequeno e focado — PRs grandes são difíceis de revisar

### Tamanho ideal de PR
- ≤ 400 linhas alteradas → revisão rápida
- \> 800 linhas → provavelmente deve ser dividido

### Template de PR
O repositório tem template automático em `.github/PULL_REQUEST_TEMPLATE.md`.

---

## 🐛 Reportando Bugs

Antes de abrir uma issue:
1. Confirme que é reproduzível
2. Veja se já existe issue aberta para o mesmo problema

No report, inclua:
- Versão do MariPDF (`Ajuda → Sobre`)
- Versão do Windows
- PDF que causa o problema (se possível, um arquivo mínimo que reproduz)
- Passos para reproduzir
- Comportamento esperado vs observado

---

## ❤️ Agradecimento

Todo contribuidor que tiver PR mergeado vai aparecer no arquivo `CONTRIBUTORS.md` (criado automaticamente via GitHub Actions).

Se o projeto te ajudou, considere uma [doação voluntária](doc/05_donationware.md) — isso mantém o projeto vivo.
