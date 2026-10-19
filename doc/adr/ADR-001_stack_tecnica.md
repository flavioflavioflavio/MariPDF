# ADR-001 — Stack Técnica Principal

> **Status:** Aceita — revisada em 2026-07-13
> **Data:** 2026-03-22
> **Autores:** Flavio

---

## Contexto

Precisamos definir a stack completa para o MariPDF. O produto tem como grande feature ser **extremamente leve e de abertura instantânea**. A escolha de tecnologia define diretamente se isso é possível ou não. As principais alternativas avaliadas foram Electron (JS/TS), C++ nativo e Rust.

---

## Opções Consideradas

### Opção A — Electron + mupdf.js
- ✅ Desenvolvimento mais rápido (HTML/CSS para UI)
- ✅ UI moderna com CSS sem esforço
- ✅ Ecossistema npm vasto
- ❌ ~150–200MB RAM em idle
- ❌ ~80MB de instalador só do Chromium
- ❌ Startup lento por natureza
- ❌ Contradiz o manifesto de leveza

### Opção B — C++ + Win32 + MuPDF
- ✅ Performance máxima
- ✅ Binário minúsculo
- ✅ Controle total
- ❌ Gerenciamento manual de memória — vetor de bugs difíceis
- ❌ UI Win32 raw = UX de 2005
- ❌ Desenvolvimento muito lento

### Opção C — Rust + egui + wgpu + engine PDF nativa ✅
- ✅ Performance comparável ao C++
- ✅ Segurança de memória garantida pelo compilador
- ✅ egui permite UI moderna e animada em GPU
- ✅ Binário estático, sem runtime externo
- ✅ Ecossistema de crates maduro (tokio, rayon, fluent...)
- ✅ Binário nativo sem runtime web externo
- ❌ Curva de aprendizado do Rust
- ⚠️ Integração com engine nativa exige lifecycle e concorrência cuidadosos

---

## Decisão

**Escolhemos: Opção C — Rust**

A única opção que cumpre simultaneamente o manifesto de leveza, a meta de abertura instantânea E permite uma UI/UX moderna. Os trade-offs (curva de aprendizado, FFI) são aceitáveis dado o retorno em performance e segurança.

---

## Stack Detalhada

| Camada | Crate / Lib | Versão alvo |
|---|---|---|
| Engine PDF | PDFium via `pdfium-render` | definida no `Cargo.toml` |
| UI framework | `egui`/`eframe` | definida no `Cargo.toml` |
| Render backend | `wgpu` | definida no `Cargo.toml` |
| Windowing | `winit` via eframe | definida no `Cargo.toml` |
| Concorrência | workers + canais | scheduler BF-1 |
| i18n | Mozilla Fluent | definida no `Cargo.toml` |
| Distribuição | EXE + MSI planejado | roadmap BF-6 |

---

## Consequências

- Todo o desenvolvimento será em Rust — novos contribuidores precisam conhecer a linguagem
- A integração PDFium fica isolada em `src/pdf/engine.rs`
- egui (immediate mode) muda o paradigma de UI em relação a frameworks declarativos — isso é positivo para performance, mas diferente do comum
- A DLL do PDFium é embarcada no EXE e extraída para diretório temporário em runtime

---

## Referências

- [egui](https://github.com/emilk/egui)
- [PDFium](https://pdfium.googlesource.com/pdfium/)
- [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- [Mozilla Fluent](https://projectfluent.org/)
- [SumatraPDF source](https://github.com/sumatrapdfreader/sumatrapdf) — referência de arquitetura
