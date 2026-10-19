# 📋 MariPDF — Log da Consultoria

> Sessão iniciada em: 2026-03-22

> **Nota de revisão (2026-07-13):** este arquivo preserva as decisões da sessão
> original. A implementação adotou PDFium, mantido oficialmente para a v1.0 pelo
> [ADR-002 revisado](adr/ADR-002_engine_pdf.md). Metas numéricas abaixo são
> hipóteses históricas até validação pelo roadmap BF-0.

---

## Contexto

Sessão de consultoria técnica e de produto para definição do **MariPDF** — um leitor de PDF para Windows com foco em leveza extrema e UI/UX premium.

---

## Decisões Tomadas

### Produto

| Decisão | Escolha | Motivo |
|---|---|---|
| Plataforma | Windows | Foco inicial |
| Categoria | Leitor PDF (sem edição) | Escopo claro, mantém leveza |
| Grande feature | Abertura instantânea + levez | Diferencial principal |
| Modelo de negócio | Donationware open source | Filosofia clássica, honesta |
| Licença | GPL-3.0-or-later | Open source; dependências mantêm suas licenças |

### Técnica

| Decisão | Escolha |
|---|---|
| Linguagem | **Rust** |
| Engine PDF | **PDFium** via `pdfium-render` — revisão oficial de 2026-07-13 |
| UI Framework | **egui** (immediate mode) |
| Renderização | **wgpu** (GPU) |
| Janela | **winit** |
| Async | **tokio** |
| Paralelismo | **rayon** |
| i18n | **Mozilla Fluent** + `sys-locale` |
| Auto-update | **self_update** (GitHub Releases) |
| Distribuição | `.exe` portátil + `.msi` (cargo-wix) + Winget |

### Design

| Decisão | Escolha |
|---|---|
| Filosofia | "Invisible Interface" |
| Ícones | Phosphor Icons (ph — line/regular) |
| Shape language | Round (50%) + Pill (999px) + Rounded Square (12px) |
| Accent color | `#5B4FE8` (violeta moderno) |
| Background leitura | `#F8F6F1` (papel quente, não branco puro) |
| Tipografia | Inter / Geist embutidas no binário |
| Cores | Totalmente tokenizadas |
| Animações | 80–200ms, sem exagero |

### Doação

| Canal | Público |
|---|---|
| PIX (QR Code) | Brasileiros |
| PayPal | Internacional |
| Stripe | Internacional (cartão direto) |

---

## Referências

- **SumatraPDF** — referência técnica de leveza
- **Apple Books** — minimalismo e foco no conteúdo
- **Linear App** — micro-animações premium
- **Arc Browser** — UI que some quando não precisa
- **Phosphor Icons** — https://icon-sets.iconify.design/ph
- **PDFium** — engine implementada e mantida para a v1.0
- **egui** — https://github.com/emilk/egui

---

## Metas de Performance

| Métrica | Alvo |
|---|---|
| Abertura | Calibrar e validar no baseline BF-0 |
| RAM | Medir e limitar por orçamento explícito |
| Tamanho do installer | Medir a partir do artefato real |
| Scroll | 60 FPS estável |

---

## Próximos Passos Sugeridos

1. ✅ Definir wireframes detalhados das telas (home + leitura) — concluído em 03_design_system.md
2. Criar repositório GitHub com estrutura base (checar [Checklist Pré-Publish](../README.md))
3. Setup do projeto Rust (`Cargo.toml`, estrutura de pastas conforme [02_arquitetura.md](02_arquitetura.md))
4. ✅ Spike técnico egui + wgpu + PDFium concluído na implementação atual
5. Implementar Fase 0 do roadmap (ver [04_roadmap.md](04_roadmap.md))

---

## Documentos Criados Nesta Sessão

| Documento | Descrição |
|---|---|
| [MANIFESTO.md](MANIFESTO.md) | Valores e compromissos permanentes |
| [01_ideias.md](01_ideias.md) | Visão, diferenciais, modelo de negócio |
| [02_arquitetura.md](02_arquitetura.md) | Stack, estrutura, performance, segurança |
| [03_design_system.md](03_design_system.md) | Tokens, icons, motion, estados de UI |
| [04_roadmap.md](04_roadmap.md) | Fases com DoD e estimativas |
| [05_donationware.md](05_donationware.md) | Estratégia de doação + spec do QR Code PIX |
| [06_storage_persistencia.md](06_storage_persistencia.md) | Schema TOML e API do módulo storage |
| [07_tratamento_de_erros.md](07_tratamento_de_erros.md) | Catálogo de erros e respostas de UI |
| [adr/ADR-001](adr/ADR-001_stack_tecnica.md) | Stack Rust vs C++ vs Electron |
| [adr/ADR-002](adr/ADR-002_engine_pdf.md) | Engine PDF oficial da v1.0: PDFium |
| [adr/ADR-003](adr/ADR-003_ui_framework.md) | UI: egui vs iced vs Tauri vs Slint |
| [adr/ADR-004](adr/ADR-004_i18n.md) | i18n: Mozilla Fluent vs gettext vs rust-i18n |
| [adr/ADR-005](adr/ADR-005_auto_update.md) | Auto-update: self_update + staged update Windows |
| [adr/ADR-006](adr/ADR-006_storage.md) | Storage: TOML em %APPDATA% vs SQLite vs Registry |
