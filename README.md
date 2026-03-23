<div align="center">

# 📄 MariPDF

**A blazing fast, beautiful PDF reader for Windows.**

*Open. Read. Done.*

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Donationware](https://img.shields.io/badge/Model-Donationware-pink.svg)](#-support-the-project)

</div>

---

## ✨ Why MariPDF?

Because every other PDF reader forgot what a PDF reader is supposed to do.

| | MariPDF | Adobe Reader | Foxit | SumatraPDF |
|---|---|---|---|---|
| Opens instantly | ✅ | ❌ | ❌ | ✅ |
| Beautiful UI | ✅ | ❌ | ❌ | ❌ |
| Zero telemetry | ✅ | ❌ | ❌ | ✅ |
| No account required | ✅ | ❌ | ❌ | ✅ |
| RAM usage (idle) | ~25MB | ~300MB | ~150MB | ~20MB |
| Installer size | ~8MB | ~700MB | ~200MB | ~10MB |

---

## 🚀 Features

- **Instant open** — feels like the file was already open
- **Beautiful, modern UI** — dark mode, sepia mode, fluid animations
- **Zero friction** — no popups, no accounts, no nags
- **Full-text search** — fast, highlighted, non-intrusive
- **Keyboard-first navigation** — every action has a shortcut
- **Remembers everything** — position, zoom, window size, per document
- **Drag & drop** — just throw a PDF at the window
- **Multi-language** — ships with PT-BR + EN, others on-demand

---

## 📦 Download

> Releases available on [GitHub Releases](../../releases)

| Format | Best for |
|---|---|
| `MariPDF-x.x.x.exe` | Portable — just run it |
| `MariPDF-x.x.x.msi` | Installer — with file association |

---

## 🏗️ Building from Source

```bash
# Requirements: Rust stable, MSVC toolchain
git clone https://github.com/[your-username]/MariPDF
cd MariPDF
cargo build --release
# Binary: target/release/maripdf.exe
```

---

## 📜 Manifesto

MariPDF was born with a clear set of values. Read the full [MANIFESTO](doc/MANIFESTO.md).

**The short version:**
- 🔓 Open source. Forever. No forks, no "Pro" version.
- ⚡ Blazing fast. Always. Non-negotiable.
- 🪶 Zero friction. It opens, it works.
- 🎨 Beautiful and easy. Aesthetics *is* UX.
- ❤️ Donationware. Free forever, sustained by gratitude.

---

## ❤️ Support the Project

MariPDF is free and always will be. If it saved you time or frustration, consider a donation:

| Method | Link |
|---|---|
| 💳 PIX | `your@pix.key` |
| 🅿️ PayPal | [paypal.me/yourlink](https://paypal.me/yourlink) |
| 💜 Stripe | [buy.stripe.com/yourlink](https://buy.stripe.com/yourlink) |

---

## 📁 Project Documentation

All project docs live in [`/doc`](doc/):

| Arquivo | Conteúdo |
|---|---|
| [MANIFESTO.md](doc/MANIFESTO.md) | Valores e compromissos do projeto |
| [00_consultoria_log.md](doc/00_consultoria_log.md) | Log de decisões arquiteturais |
| [01_ideias.md](doc/01_ideias.md) | Ideias, visão, inspirações |
| [02_arquitetura.md](doc/02_arquitetura.md) | Arquitetura técnica & stack |
| [03_design_system.md](doc/03_design_system.md) | Cores, ícones, shapes, motion, estados |
| [04_roadmap.md](doc/04_roadmap.md) | Fases de desenvolvimento & DoD |
| [05_donationware.md](doc/05_donationware.md) | Estratégia open source & doação |
| [06_storage_persistencia.md](doc/06_storage_persistencia.md) | Schema de persistência de estado |
| [07_tratamento_de_erros.md](doc/07_tratamento_de_erros.md) | Catálogo de erros e respostas de UI |
| [adr/ADR-001](doc/adr/ADR-001_stack_tecnica.md) | Stack Rust vs C++ vs Electron |
| [adr/ADR-002](doc/adr/ADR-002_engine_pdf.md) | Engine PDF: MuPDF vs pdfium |
| [adr/ADR-003](doc/adr/ADR-003_ui_framework.md) | UI: egui vs iced vs Tauri |
| [adr/ADR-004](doc/adr/ADR-004_i18n.md) | i18n: Mozilla Fluent vs gettext |
| [adr/ADR-005](doc/adr/ADR-005_auto_update.md) | Auto-update: self_update + staged update |
| [adr/ADR-006](doc/adr/ADR-006_storage.md) | Storage: TOML em %APPDATA% |

---

## ⚠️ Checklist Pré-Publish

> **Antes do primeiro commit público**, substitua todos os placeholders:

- [ ] `[your-username]` no README e no clone URL
- [ ] `your@pix.key` → sua chave PIX real
- [ ] `paypal.me/yourlink` → seu link PayPal real
- [ ] `buy.stripe.com/yourlink` → seu link Stripe real
- [ ] URL do repositório no MANIFESTO.md
- [ ] `.github/FUNDING.yml` com dados reais
- [ ] Screenshots reais no README (após Fase 3)

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

- Bug reports → [Issues](../../issues)
- Feature requests → [Discussions](../../discussions)
- Code → Fork + PR (GPLv3)

---

## 📄 License

[GNU GPL v3.0](LICENSE) — Open source. Always.

---

<div align="center">

*Made with ❤️ for people who just want to read.*

</div>
