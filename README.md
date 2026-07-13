<div align="center">

# MariPDF

**A fast, focused PDF reader for Windows.**

*Open. Read. Done.*

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-pre--release-yellow.svg)](doc/08_roadmap_blazing_fast_v1.md)

</div>

---

## Status

MariPDF está em desenvolvimento ativo e ainda não possui uma release pública
estável. O foco atual é tornar o pipeline de renderização totalmente assíncrono,
medir performance de forma reproduzível e fechar os contratos funcionais da v1.0.

O plano vigente está em
[Roadmap Blazing Fast e v1.0](doc/08_roadmap_blazing_fast_v1.md).

## Princípios

- abertura e navegação responsivas;
- interface nativa em Rust com egui/wgpu;
- nenhum login, conta ou telemetria;
- leitor focado, sem edição de PDF;
- open source e donationware, sem versão Pro.

Claims públicos de tempo, memória e tamanho só serão publicados depois de
benchmarks reproduzíveis. Os números antigos eram estimativas e foram removidos.

## Recursos implementados

- abertura por CLI, diálogo nativo e drag-and-drop;
- modos página única e rolagem contínua;
- zoom, Fit Width, Fit Page e navegação por teclado;
- busca textual, seleção/cópia e links externos;
- miniaturas e sumário/bookmarks;
- modos Dark, Light e Sepia;
- recentes, persistência por documento e preferências;
- PT-BR e EN embutidos com Mozilla Fluent;
- associação `.pdf` no Windows e logs locais.

Alguns desses recursos ainda possuem gaps registrados no roadmap da v1.0 e não
devem ser interpretados como contratos finais de release.

## Engine e arquitetura

A v1.0 usa **PDFium** através de `pdfium-render`. A DLL é embarcada no executável
e extraída para o diretório temporário na primeira utilização. A UI usa egui com
backend wgpu.

MuPDF é somente uma alternativa futura. Qualquer reavaliação exige benchmark A/B,
spike técnico e decisão explícita de licença.

## Build local

Requisitos:

- Windows 10/11;
- Rust stable com toolchain MSVC;
- Visual Studio Build Tools 2022.

```powershell
git clone https://github.com/flavioflavioflavio/MariPDF.git
cd MariPDF
cargo build
cargo run
```

Build otimizado:

```powershell
cargo build --release
```

Binário: `target/release/maripdf.exe`.

## Gates de desenvolvimento

```powershell
cargo fmt --check
cargo check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

O objetivo da fase BF-0 é manter todos esses gates verdes no CI Windows.

## Distribuição

Ainda não há instalador ou canal de update suportado. O roadmap prevê:

- `.exe` portátil;
- MSI com instalação/desinstalação validada;
- checksums e artefatos gerados por tag;
- atualização verificada com rollback;
- Winget depois da estabilização do MSI.

## Documentação

| Documento | Papel |
|---|---|
| [Roadmap v1.0](doc/08_roadmap_blazing_fast_v1.md) | Fonte vigente de execução e critérios de aceite |
| [Arquitetura](doc/02_arquitetura.md) | Arquitetura implementada e direção aprovada |
| [ADR-001](doc/adr/ADR-001_stack_tecnica.md) | Escolha de Rust + egui/wgpu |
| [ADR-002](doc/adr/ADR-002_engine_pdf.md) | Decisão oficial de manter PDFium na v1.0 |
| [ADR-003](doc/adr/ADR-003_ui_framework.md) | Escolha do egui |
| [Roadmap histórico](doc/04_roadmap.md) | Registro das fases iniciais, não fonte vigente |
| [Manifesto](doc/MANIFESTO.md) | Valores permanentes do projeto |

## Contribuindo

Leia [CONTRIBUTING.md](CONTRIBUTING.md). Mudanças de performance devem incluir
evidência antes/depois; mudanças de UI devem preservar a responsividade da thread
principal.

## Licença

MariPDF é distribuído sob GNU GPL v3.0 ou posterior. PDFium e as demais
dependências mantêm suas respectivas licenças.

## Apoio

MariPDF seguirá gratuito e sem recursos pagos. Os canais oficiais de doação serão
publicados somente quando estiverem configurados; não envie valores para chaves ou
links encontrados em documentos históricos.
