# ADR-002 — Engine PDF

> **Status:** Aceita
> **Data:** 2026-03-22
> **Autores:** Flavio

---

## Contexto

O MariPDF precisa de uma engine capaz de renderizar PDFs com alta fidelidade, suporte a fontes CJK, formulários, links e busca de texto. A engine é o coração do app — a escolha impacta diretamente qualidade de renderização, tamanho do binário, licença e manutenção.

---

## Opções Consideradas

### Opção A — MuPDF (Artifex) ✅
- ✅ Melhor qualidade de renderização disponível (usada no Adobe Acrobat Reader historicamente)
- ✅ Suporte completo: texto, imagens, formulários, links, anotações, CJK
- ✅ Bindings Rust disponíveis (`mupdf-rs`)
- ✅ Usada no SumatraPDF — prova de conceito de leveza
- ✅ Busca de texto nativa e eficiente
- ❌ Licença AGPLv3 — exige atenção (veja seção Consequências)
- ❌ FFI com C — requer blocos `unsafe` em Rust
- ❌ Compilação cross-platform exige MSVC no Windows

### Opção B — pdfium (Google)
- ✅ Mantida ativamente pelo Google (usada no Chrome)
- ✅ Bindings Rust disponíveis (`pdfium-render`)
- ✅ Licença BSD — mais permissiva
- ❌ Distribuída como `.dll` pré-compilada — não compila no binário estático
- ❌ Adiciona ~20MB de `.dll` na distribuição (quebra a meta de ~8MB)
- ❌ Dependência em runtime externo — contradiz o modelo portátil

### Opção C — poppler
- ✅ Muito usado no Linux/GTK
- ✅ Licença GPL v2+
- ❌ Focado em Linux — suporte Windows é segunda classe
- ❌ Qualidade de renderização inferior ao MuPDF
- ❌ Sem bindings Rust maduros para Windows

### Opção D — pdf.js (Mozilla) via WebView
- ✅ Maturíssimo, mantido pela Mozilla
- ✅ Renderização excelente
- ❌ Requer WebView/Chromium — ~100MB extra
- ❌ Contradiz completamente o manifesto de leveza

---

## Decisão

**Escolhemos: Opção A — MuPDF**

A única engine que entrega fidelidade de renderização máxima, compila estaticamente no binário, tem integração Rust viável e é comprovada em produção (SumatraPDF). O trade-off de licença é gerenciável (veja abaixo).

---

## Consequências

### ⚠️ Licença — Ponto Crítico

MuPDF é **AGPLv3**. O projeto MariPDF é **GPLv3**. Isso tem uma implicação importante:

- Quando o código MuPDF e o código MariPDF são **compilados juntos** (via FFI estática), o binário resultante fica sujeito à **AGPLv3** — a licença mais restritiva prevalece
- Para um **app desktop**, AGPLv3 é equivalente a GPLv3 na prática: o usuário recebe o binário, tem acesso ao código-fonte, pode modificar e redistribuir
- A única diferença do AGPL é a cláusula de "uso em servidor" — irrelevante para um reader de desktop

**Ação necessária:**
- O `README.md` e o cabeçalho de licença devem mencionar que o binário final é AGPLv3 (pela incorporação do MuPDF)
- A licença do **código-fonte** do MariPDF permanece GPLv3
- Adicionar nota no `LICENSE` explicando a composição

### ⚠️ JavaScript em PDFs

MuPDF tem suporte a JavaScript embutido em PDFs (via seu interpretador JS). Para um reader sem edição, isso é um **vetor de risco**.

**Mitigação:**
- Desabilitar o interpretador JS do MuPDF na compilação via flag `#define FZ_ENABLE_JS 0`
- Documentar isso explicitamente como feature de segurança
- Adicionar nota no README: "JavaScript em PDFs é desabilitado por design"

### Estrutura dos blocos `unsafe`

Todo código que faz FFI com MuPDF fica isolado em `src/pdf/engine.rs`. Cada bloco `unsafe` terá comentário obrigatório explicando:
1. Por que o `unsafe` é necessário
2. Quais invariantes garantem que é seguro

---

## Referências

- [MuPDF — documentação oficial](https://mupdf.com/docs/)
- [mupdf-rs — bindings Rust](https://github.com/messense/mupdf-rs)
- [SumatraPDF source — referência de integração MuPDF](https://github.com/sumatrapdfreader/sumatrapdf)
- [AGPLv3 — texto completo](https://www.gnu.org/licenses/agpl-3.0.html)
- [Compatibilidade AGPLv3 + GPLv3](https://www.gnu.org/licenses/license-compatibility.html)
