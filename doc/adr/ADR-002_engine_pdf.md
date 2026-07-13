# ADR-002 — Engine PDF da v1.0

> **Status:** Aceita — revisada
> **Decisão atual:** PDFium
> **Data original:** 2026-03-22
> **Revisão:** 2026-07-13

## Contexto

A decisão original favorecia MuPDF com base em estimativas de tamanho e qualidade.
Durante a implementação, o projeto integrou PDFium por meio de `pdfium-render`,
embarcou `pdfium.dll` no executável e construiu sobre essa API render, texto,
links, bookmarks e miniaturas.

O código real, os custos de migração e as metas de v1.0 exigiram uma revisão
explícita deste ADR.

## Opções reavaliadas

### PDFium via `pdfium-render`

- já está integrado no produto;
- cobre os recursos atuais da v1.0;
- possui licença permissiva compatível com o projeto;
- é carregado uma vez e pode ser usado por workers com documentos próprios;
- permite concentrar a v1.0 no scheduler, virtualização e correção funcional;
- exige distribuir/carregar uma DLL e manter sua atualização sob controle.

### MuPDF

- continua sendo uma engine tecnicamente relevante;
- exigiria nova integração, nova matriz de compatibilidade e migração dos recursos;
- introduziria decisão de AGPL versus licença comercial;
- não resolve, sozinho, os bloqueios atuais da thread da UI e virtualização.

### Poppler/pdf.js

Não oferecem ganho suficiente para justificar uma terceira integração antes da
v1.0. Permanecem fora do escopo.

## Decisão

**PDFium é a engine oficial do MariPDF v1.0.**

Não haverá migração de engine durante o roadmap BF. O trabalho deve criar uma
fronteira interna clara entre UI/scheduler e engine, sem construir uma abstração
genérica maior do que o produto necessita.

## Condições para reabrir esta decisão

Uma proposta de troca precisa apresentar:

1. corpus idêntico e benchmark A/B de startup, primeira página, zoom, busca, RAM e
   tamanho distribuído;
2. paridade funcional demonstrada;
3. spike Windows compilável e distribuível;
4. threat model e impacto de manutenção;
5. decisão de licença aprovada;
6. plano de migração e rollback.

Sem esses seis itens, a discussão permanece exploratória e não altera a v1.0.

## Consequências

- `Cargo.toml`, arquitetura, README e CHANGELOG devem declarar PDFium.
- A licença do MariPDF permanece GPL-3.0-or-later; não há composição com MuPDF.
- A DLL embarcada deve ter versão, origem e atualização rastreáveis.
- O principal investimento de performance será o scheduler assíncrono, cache por
  memória, virtualização e, se medido, tiles/LOD.

## Referências internas

- `src/pdf/engine.rs`
- `Cargo.toml`
- `doc/02_arquitetura.md`
- `doc/08_roadmap_blazing_fast_v1.md`
