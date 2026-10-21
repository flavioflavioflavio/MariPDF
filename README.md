<div align="center">

# MariPDF

**Um leitor de PDF rápido e focado para Windows.**

*Abrir. Ler. Pronto.*

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-1.0%20release%20candidate-yellow.svg)](doc/08_roadmap_blazing_fast_v1.md)

<img src="doc/screenshots/navegacao.gif" alt="Navegação contínua no MariPDF" width="720">

*Dedicado a Mariana Lima, com muito amor.*

</div>

---

## Status

**Release candidate da v1.0.** O roadmap
[Blazing Fast v1.0](doc/08_roadmap_blazing_fast_v1.md) foi executado. Os
critérios e as decisões estão registrados, e cada contrato funcional tem um
teste correspondente ([matriz](doc/09_matriz_contratos.md)).

## Princípios

- A interface nunca trava: todo trabalho com PDF roda fora da thread da UI.
- Sem login, conta ou telemetria. A rede só é usada quando você clica em
  *Verificar atualizações*.
- Leitor focado, sem edição de PDF.
- Open source (GPLv3) e donationware, sem versão Pro.
- Números públicos só com medição reproduzível.

## Recursos

- Página única e rolagem contínua virtualizada. Documentos de 1.000 páginas
  abrem sem custo linear.
- Zoom com 100% = tamanho físico, ajustar à largura ou à página.
- Busca com destaque da ocorrência na página e trecho de contexto. Ignora
  maiúsculas/minúsculas e encontra frases que quebram de linha.
- Seleção de texto: arrastar, duplo/triplo clique, `Shift`+clique e `Ctrl+A`.
  `Ctrl+C` e o botão direito copiam.
- Sumário, miniaturas e links clicáveis.
- **Modo Flutuante** (`Ctrl+Shift+F`): o PDF vira a própria janela, sem
  bordas, no estilo PureRef, e pode ficar sempre no topo.
- Temas Dark, Light e Sépia, ou seguindo o Windows.
- Interface em português e inglês.
- Janela reabre onde você deixou. Monitores desconectados são tratados.
- Associação `.pdf` pelo fluxo oficial do Windows 10/11.

| Leitor | Busca | Modo Flutuante |
|---|---|---|
| ![Leitor](doc/screenshots/reader.png) | ![Busca](doc/screenshots/search.png) | ![Modo Flutuante](doc/screenshots/float.png) |

## Desempenho

Medido com o medidor embutido sobre um corpus sintético reproduzível. Máquina:
Ryzen 9 5950X, RTX 3080, Windows 11. Detalhes e método em
[`doc/benchmarks`](doc/benchmarks/baseline-2026-10-19.md).

| Métrica (mediana) | Resultado |
|---|---|
| Abrir um PDF | 7–10 ms |
| Abrir até a 1ª página na tela | 58–90 ms |
| Salto para página sem cache | 12–74 ms |
| Página vizinha (já pré-carregada) | 5–17 ms (scan 300 dpi: ~70 ms) |
| Pior frame da UI no roteiro | < 40 ms (p95 < 9 ms) |
| Memória no fim do roteiro | ~220–280 MB (pico ~260–300 MB) |
| Executável portátil | ~20 MB, com o PDFium embutido |

## Instalação

Baixe da página de [Releases](https://github.com/flavioflavioflavio/MariPDF/releases):

- **`MariPDF-<versão>-setup.exe`**: instalador **por usuário**. Não pede
  administrador nem UAC e cria atalho no Menu Iniciar. Opcionalmente registra
  o `.pdf`; depois é só escolher o MariPDF em *Aplicativos padrão*.
- **`MariPDF-<versão>-portable.exe`**: um único arquivo, sem instalação. Roda
  de qualquer pasta ou pendrive.

Os binários ainda não têm assinatura Authenticode. Na primeira execução, o
SmartScreen pode mostrar *"O Windows protegeu o computador"* → **Mais
informações → Executar assim mesmo**. Para conferir a integridade:

```powershell
Get-FileHash .\MariPDF-<versão>-setup.exe -Algorithm SHA256
# compare com SHA256SUMS.txt (assinado com Ed25519 — ver doc/10_distribuicao.md)
```

O botão *Configurações → Sobre → Verificar atualizações* baixa a versão nova. O instalador só
roda se a assinatura e o hash conferirem.

## Build local

Requisitos: Windows 10/11, Rust stable (MSVC) e Visual Studio Build Tools 2022.

```powershell
git clone https://github.com/flavioflavioflavio/MariPDF.git
cd MariPDF
cargo run --release
```

| Comando | Para quê |
|---|---|
| `pwsh tools/gates.ps1` | `fmt` + `clippy -D warnings` + testes, como no CI |
| `pwsh tools/smoke.ps1` | abre o app real com um PDF conhecido e exige o roteiro completo |
| `python tools/make_corpus.py bench/corpus --manifest bench/corpus.toml` | gera o corpus de benchmark |
| `pwsh tools/bench.ps1` | mede o corpus e gera `summary.md` |
| `pwsh tools/package.ps1` | portátil + instalador + SHA256SUMS em `dist/` (requer Inno Setup 6) |
| `pwsh tools/installer_test.ps1` | instala, atualiza, testa downgrade e desinstala, conferindo o registro |

## Documentação

| Documento | Papel |
|---|---|
| [Roadmap v1.0](doc/08_roadmap_blazing_fast_v1.md) | Plano executado, critérios e decisões |
| [Matriz de contratos](doc/09_matriz_contratos.md) | Cada comportamento prometido → teste que o protege |
| [Benchmarks](doc/benchmarks/baseline-2026-10-19.md) | Baseline, método e decisão de tiles ([BF-3](doc/benchmarks/bf3-tiles-spike.md)) |
| [Distribuição](doc/10_distribuicao.md) | Instalador, associação, assinatura, atualização e release |
| [Arquitetura](doc/02_arquitetura.md) | Arquitetura implementada |
| [ADR-002](doc/adr/ADR-002_engine_pdf.md) · [ADR-007](doc/adr/ADR-007_isolamento_pdfium.md) | PDFium na v1.0 e decisão sobre isolamento |
| [Manifesto](doc/MANIFESTO.md) | Valores permanentes do projeto |

## Contribuindo

Leia o [CONTRIBUTING.md](CONTRIBUTING.md). Mudanças de performance trazem
números antes/depois do `tools/bench.ps1`. Mudanças de comportamento atualizam
a matriz de contratos e o teste correspondente.

## Autor

Criado por **Flávio Takemoto**: diretor de arte, motion designer e criador de
produtos digitais, fundador da [EvoMind](https://evomind.app/investors) ·
[LinkedIn](https://www.linkedin.com/in/takemotoflavio/).

## Licença e apoio

GPL-3.0-or-later. O PDFium e as demais dependências mantêm suas licenças (SBOM
publicado em cada release).

O MariPDF seguirá gratuito e sem recursos pagos. Um canal oficial de doação
será anunciado aqui e no *Sobre* quando existir. Não envie valores para links
encontrados em documentos antigos.
