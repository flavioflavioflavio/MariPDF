# ADR-001 — Stack Técnica Principal

> **Status:** Aceita
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

### Opção C — Rust + egui + wgpu + MuPDF via FFI ✅
- ✅ Performance comparável ao C++
- ✅ Segurança de memória garantida pelo compilador
- ✅ egui permite UI moderna e animada em GPU
- ✅ Binário estático, sem runtime externo
- ✅ Ecossistema de crates maduro (tokio, rayon, fluent...)
- ✅ Binário final estimado: ~8MB
- ✅ RAM idle estimada: ~25MB
- ❌ Curva de aprendizado do Rust
- ❌ FFI com MuPDF (C) exige cuidado

---

## Decisão

**Escolhemos: Opção C — Rust**

A única opção que cumpre simultaneamente o manifesto de leveza, a meta de abertura instantânea E permite uma UI/UX moderna. Os trade-offs (curva de aprendizado, FFI) são aceitáveis dado o retorno em performance e segurança.

---

## Stack Detalhada

| Camada | Crate / Lib | Versão alvo |
|---|---|---|
| Engine PDF | `mupdf` (FFI) | latest |
| UI framework | `egui` | latest |
| Render backend | `wgpu` | latest |
| Windowing | `winit` | latest |
| Async runtime | `tokio` | latest |
| Paralelismo | `rayon` | latest |
| i18n | `fluent` + `sys-locale` | latest |
| Auto-update | `self_update` | latest |
| HTTP | `reqwest` | latest |
| Distribuição | `cargo-wix` (MSI) | latest |

---

## Consequências

- Todo o desenvolvimento será em Rust — novos contribuidores precisam conhecer a linguagem
- A integração MuPDF via FFI precisa de atenção especial em `unsafe` blocks
- egui (immediate mode) muda o paradigma de UI em relação a frameworks declarativos — isso é positivo para performance, mas diferente do comum
- O binário final será compilado estaticamente — sem dependências de runtime no Windows do usuário

---

## Referências

- [egui](https://github.com/emilk/egui)
- [MuPDF](https://mupdf.com/)
- [self_update](https://github.com/jaemk/self_update)
- [Mozilla Fluent](https://projectfluent.org/)
- [SumatraPDF source](https://github.com/sumatrapdfreader/sumatrapdf) — referência de arquitetura
