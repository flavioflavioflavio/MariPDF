# ADR-007 — Isolamento do PDFium em processo separado

> **Status:** Aceita
> **Data:** 2026-10-19
> **Autores:** Flávio Takemoto, com apoio do Claude Code

---

## Contexto

O MariPDF abre arquivos de origem desconhecida com o PDFium (C++), carregado
no mesmo processo da UI. A BF-508 pede uma decisão, baseada em modelo de
ameaça, sobre isolar o PDFium em um processo separado na v1.x.

Riscos considerados:

1. **Crash nativo** (falha de segmentação, abort) causado por um PDF malformado,
   que derruba a janela inteira.
2. **Exploração de memória** via PDF malicioso, com código executando com os
   privilégios do usuário.
3. **Panic em Rust** no worker de documento.

O que já existe na v1.0:

- O PDFium é o mesmo motor do Chrome. Recebe fuzzing contínuo (OSS-Fuzz) e
  correções de segurança frequentes.
- O MariPDF não executa JavaScript de PDF nem formulários XFA e não abre anexos.
  A superfície se limita a parse, render e extração de texto.
- Panics em Rust viram falha do job (`catch_unwind` no worker, com
  `panic = "unwind"`). O lock do PDFium se recupera de envenenamento, e uma
  página que falha fica isolada como placeholder no modo contínuo.
- Erros de abertura (senha, corrompido, ausente) chegam ao usuário como
  mensagens claras.

---

## Opções Consideradas

### Opção A — Manter o PDFium no processo da UI (atual)
- ✅ Zero custo de IPC. Bitmaps vão direto do PDFium para a textura, o que
  sustenta o objetivo "blazing fast".
- ✅ Distribuição simples: um único executável portátil, com a DLL embutida.
- ❌ Um crash nativo do PDFium fecha o app.
- ❌ Uma exploração bem-sucedida roda com os privilégios do usuário.

### Opção B — Processo filho para parse/render, comunicando por pipe ou memória compartilhada
- ✅ Um crash do PDFium derruba só o filho, e a UI reabre o documento.
- ✅ Permite reduzir privilégios do filho (job object, token restrito,
  AppContainer).
- ❌ Copia ou compartilha bitmaps de dezenas de MB por página. Exige memória
  compartilhada e sincronização, além de serializar chars, links e bookmarks.
- ❌ Complexidade relevante: ciclo de vida do filho, restart, timeouts e
  testes. Também atrasa a v1.0.

### Opção C — Sandbox completo no estilo Chrome
- ✅ Isolamento máximo.
- ❌ Custo desproporcional para um leitor desktop open source na v1.

---

## Decisão

**Escolhemos: Opção A para a v1.0.** A Opção B fica como candidata para a v1.x.

O ganho real da B é contra crashes e explorações do PDFium. Hoje esses riscos
já são mitigados pela origem do motor (Chromium, com fuzzing) e por uma
superfície pequena: sem JS, sem XFA, sem anexos. Os problemas de Rust estão
contidos no worker. O custo da B cai justamente no caminho crítico de
performance, que é o diferencial do produto.

---

## Consequências

- O processo único continua. Os crashes de Rust ficam contidos, mas os crashes
  nativos do PDFium ainda encerram o app.
- **Gatilhos para reavaliar** e partir para a Opção B:
  - relato de crash reproduzível do PDFium com PDF real de usuário;
  - CVE crítica no PDFium sem atualização rápida da DLL embarcada;
  - adoção de recursos que ampliem a superfície (formulários, JS, anexos).
- **Manutenção:** atualizar a `pdfium.dll` embarcada a cada release estável
  do PDFium que corrija problemas de segurança. O processo de release (BF-6)
  registra a versão da DLL nas notas.
