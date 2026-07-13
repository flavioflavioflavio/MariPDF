# 🗺️ MariPDF — Roadmap de Desenvolvimento

> Criado em: 2026-03-22 | Atualizado em: 2026-07-13
> **Status:** roadmap histórico das fases iniciais. A fonte vigente de execução é
> [08_roadmap_blazing_fast_v1.md](08_roadmap_blazing_fast_v1.md).

---

## Como Ler Este Roadmap

- **Estimativas** são otimistas para dev solo experiente em Rust
- **DoD** = Definition of Done — critérios que definem quando a fase está concluída
- `⛔ depende de` = este item só pode começar após o outro estar 100% pronto
- Itens sem dependência explícita podem ser feitos em paralelo dentro da mesma fase

---

## ✅ Fase 0 — Setup & Fundação  `CONCLUÍDA`
*Objetivo: projeto rodando, janela abrindo, estrutura sólida*

- [x] Criar repositório GitHub (`MariPDF`)
- [x] Setup `Cargo.toml` com dependências core (egui 0.31, eframe, wgpu, pdfium-render, rfd, anyhow, tracing)
- [x] Estrutura de pastas: `src/{app,pdf,ui}`, `locales/`, `assets/`, `doc/`
- [x] Janela egui/wgpu abrindo com ícone embutido (PNG via `include_bytes!`)
- [x] `build.rs` com `winres` (metadados Win: ícone, versão, empresa)
- [x] `.gitignore` configurado
- [x] Documentação completa: ADRs (001/002/003), CONTRIBUTING.md, design system, roadmap

**✅ DoD da Fase 0:**
> `cargo run` abre janela com ícone correto na taskbar. Docs na pasta `doc/`. Build limpo.

---

## ✅ Fase 1 — MVP: Ler um PDF  `CONCLUÍDA`
*Objetivo: abrir e ler um PDF, nada mais*

- [x] Integrar `pdfium-render` (binários Chromium prebuilt, sem compilação C)
- [x] `pdfium.dll` embutida no executável via `include_bytes!` → app 100% portátil
- [x] Renderizar páginas como bitmap RGBA → `egui::TextureHandle`
- [x] Modo **Single-Page**: scroll do mouse muda página (1 notch = 1 página, `raw_scroll_delta`)
- [x] Modo **Contínuo**: todas as páginas empilhadas, lazy rendering com `PageCache`
- [x] Toggle Single ↔ Contínuo na toolbar
- [x] **Zoom**: `Ctrl+Scroll`, botões `+`/`−`/`↺` na toolbar
- [x] Navegação: `←`/`→`, `PgUp`/`PgDn`, `Home`/`End`
- [x] Abrir arquivo via **CLI** (`maripdf.exe arquivo.pdf`)
- [x] Abrir via **FileDialog nativo** (rfd) e via **drag & drop**
- [x] Toolbar premium: nome do arquivo, página atual/total, zoom %, botões de navegação
- [x] Tela inicial com drop zone animada

**✅ DoD da Fase 1:**
> PDF abre via CLI, drag & drop e dialog nativo. Navegação entre páginas funcional. Zoom `Ctrl+Scroll` operacional. Dois modos de visualização (Single/Contínuo). App portátil — único `.exe`.

---

## ✅ Fase 2 — Experiência de Leitura
*Concluída ✅*

### Prioridade Alta (impacto imediato na leitura)
- [x] Modos Single-Page e Contínuo com toggle na toolbar
- [x] **Fit Width** — ajusta zoom para a página preencher a largura disponível (`W`)
- [x] **Fit Page** — ajusta zoom para a página inteira caber na tela (`Ctrl+0`)
- [x] **Ir para página** — `Ctrl+G` abre overlay de input numérico
- [x] **Tela cheia** — `F11` (oculta toolbar e barra de título)
- [x] **Persistência básica** — lembra zoom e página ao reabrir o mesmo arquivo (TOML em `%APPDATA%\maripdf\`)
- [x] **Status Bar** — barra inferior fixa com: página atual/total, zoom %, nome do arquivo, modo de visualização e indicador de carregamento
- [x] **Modos de leitura** — `Dark` / `Light` / `Sepia` com toggle na toolbar + tintura aplicada ao bitmap PDF (zero custo CPU)
- [x] **Toast de erro** — exibido 5s com fade ao abrir PDF com senha, corrompido ou não encontrado

### Prioridade Média
- [x] **Page cache LRU** — pré-renderiza N páginas adjacentes em thread separada (mpsc channel, eviction LRU, prefetch PREFETCH_AHEAD/BEHIND)
- [x] **Cursor contextual** — I-beam sobre área de texto da página (via `viewer.last_page_rect`)

### Prioridade Baixa (complexidade alta)
- [x] **Busca full-text `Ctrl+F`** — overlay flutuante, contador de resultados, navegação Enter/Shift+Enter/F3 e botões ◄ ►, navega para a página do hit
- [x] **Seleção de texto `Ctrl+A`** — seleciona todo o texto da página atual (extrakt via PDFium)
- [x] **Cópia `Ctrl+C`** — copia texto selecionado para clipboard do sistema com toast de confirmação
- [x] **Links externos** — extrai links URI via PDFium (`page.links().iter()`, `as_uri_action().uri()`) e abre no browser (`crate open`)

**DoD da Fase 2:**
> ✅ COMPLETO. Fit Width funcional. `Ctrl+G` navega para página correta. `F11` entra/sai de tela cheia. App lembra última posição ao reabrir o mesmo PDF. Modos Dark/Light/Sepia com tintura do conteúdo. Status Bar com informações de página, zoom e modo. Toasts de erro. Page cache LRU com thread de background. Busca full-text `Ctrl+F`. Cursor I-beam sobre página. Seleção e cópia de texto `Ctrl+A`+`Ctrl+C`. Links externos abrindo no browser.

---

## ✅ Fase 3 — Polimento Visual (CONCLUÍDA)
*Objetivo: UI/UX premium conforme design system*

- [x] Tokens de cor consolidados no egui (constantes `BG_BASE`, `VIOLET_PRIMARY`, etc no `app.rs`)
- [x] Tela inicial com arquivos **recentes** (layout 2 colunas: lista de recentes + drop zone)
- [x] Dark mode **system-aware** (detecta `AppsUseLightTheme` via `winreg`)
- [x] Estados de loading com **skeleton animado** (pulso sin, fase por página, 3 stripes shimmer)
- [x] Toast **Info** (violet/verde ✓) vs **Error** (vermelho ⚠) com cores distintas
- [x] Sombra de página estilo livro tridimensional (3 camadas, opacidade progressiva)
- [x] **Scrollbar violet** temática (7px, handle violet via `ctx.style_mut`)
- [x] Phosphor Icons via fonte embutida
- [x] ~~Fade toolbar/hover~~ — egui 0.31 gerencia hover internamente via Visuals

**✅ DoD da Fase 3:**
> Tela inicial com histórico de recentes. App inicia no tema do Windows automaticamente. Skeleton animado no loading. Toasts diferenciados por tipo. Sombra multicamadas premium. Scrollbar temática violet.

---

## ✅ Fase 4 — Qualidade & Distribuição `CONCLUÍDA (P1/P2)`
*Objetivo: app pronto para o público*
*Atualizado: 2026-03-23*

### ✅ P1 — Alta prioridade (concluídos)
- [x] Lembra tamanho/posição da janela (`window.toml` em `%APPDATA%\maripdf\`)
- [x] Lembra posição de leitura por documento e zoom preferido (já existia desde Fase 2)
- [x] Associação de arquivo `.pdf` no Windows (HKCU registry, sem admin)
  - Botão "Definir como leitor padrão" nas Settings → Privacidade
  - Botão "Remover" quando já associado, com status em tempo real
- [x] Múltiplas instâncias — eframe 0.31 suporta nativamente, confirmado
- [x] About dialog com versão, link GitHub e licença (botão `i` na toolbar + `Ctrl+Shift+I`)
- [x] Log de erros em `%APPDATA%\maripdf\maripdf.log` (rolling diário, max 5 arquivos)

### P3 — Estado revisado
- [x] i18n: `pt-BR` e `en` embutidos (Fluent; cobertura total ainda no roadmap BF)
- [ ] i18n on-demand via CDN para outros idiomas
- [ ] Auto-update via GitHub Releases (`self_update`)
- [ ] `cargo-wix` → gerar `.msi` assinado
- [ ] Manifesto `winget` (submit para repositório oficial)
- [ ] Dialog de doação 1x após 30 dias de uso

**✅ DoD da Fase 4 (P1/P2):**
> Associação `.pdf` funcional (HKCU). Lembra posição e zoom ao reabrir. Lembra tamanho da janela. Log em arquivo rolling. About dialog completo. Múltiplas instâncias funcionando.

---

## ⏳ Fase 5 — substituída pelo Roadmap BF da v1.0
*Objetivo: launch público*
*Sem estimativa até o baseline BF-0. Consulte o roadmap vigente.*

- [ ] README completo com screenshots reais
- [ ] CHANGELOG.md retroativo (v0.1 → v1.0)
- [ ] CONTRIBUTING.md atualizado
- [ ] `.github/FUNDING.yml` (PayPal + PIX)
- [ ] GitHub Release: `.exe` portátil + `.msi`, SHA-256 publicado
- [ ] Post em comunidades: HN, Reddit r/rust, r/windows

**✅ DoD da Fase 5 (= v1.0.0):**
> GitHub Release publicado. README com screenshots. CHANGELOG completo. Post publicado em ≥2 comunidades.

---

## Futuro (pós v1.0)

*Ordenado por impacto estimado vs esforço:*

| Feature | Impacto | Esforço |
|---|---|---|
| Miniaturas de páginas (painel lateral) | Alto | Médio |
| Histórico de posição de leitura | Alto | Baixo |
| Marcadores por documento | Médio | Médio |
| Suporte a CBZ/CBR (quadrinhos) | Médio | Alto |
| Atalhos customizáveis | Médio | Alto |
| Mais idiomas on-demand | Baixo | Baixo |
| Anotações/highlights | Alto | Muito Alto |
