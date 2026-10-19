# MariPDF — Arquitetura Técnica

> Criado em: 2026-03-22
> Revisado em: 2026-07-26
> Estado: arquitetura atual + direção aprovada para v1.0

## Stack atual

| Camada | Tecnologia | Estado |
|---|---|---|
| Linguagem | Rust 2021 | Implementado |
| Engine PDF | PDFium via `pdfium-render` | Decisão oficial da v1.0 |
| UI | `egui`/`eframe` 0.31 | Implementado |
| GPU | `wgpu` 24 | Backend da UI e texturas |
| Windowing | `winit` via eframe | Implementado |
| Concorrência | scheduler priorizado + canais `mpsc` | BF-1 implementado |
| Persistência | TOML + serde | Implementado; escrita atômica pendente |
| i18n | Mozilla Fluent | PT-BR e EN embutidos |
| Logging | tracing + rolling appender | Implementado |
| Integração Windows | `winreg` + recursos Win32 | Parcial |

Tokio está no manifesto, mas não é atualmente o centro do pipeline de render. O
roadmap não exige introduzi-lo no hot path: o scheduler pode usar workers e canais
com lifecycle explícito, desde que a UI nunca bloqueie.

## Decisões vigentes

1. PDFium permanece na v1.0.
2. Nenhuma operação pesada de PDF deve executar na thread da UI.
3. Render, thumbnails, busca, texto, links e bookmarks devem compartilhar um
   scheduler com prioridade, cancelamento lógico e backpressure.
4. Resultados pertencem a uma geração de documento/zoom e resultados antigos não
   podem alterar a UI.
5. Cache deve ser limitado por bytes de CPU/GPU, não por número fixo de páginas.
6. Claims de performance dependem de benchmark reproduzível.
7. PDFium é single-threaded: workers podem preparar/agendar trabalho em paralelo,
   mas toda chamada nativa e todo fechamento de documento passam pelo mesmo lock.

Ver [ADR-002](adr/ADR-002_engine_pdf.md) e
[Roadmap BF](08_roadmap_blazing_fast_v1.md).

## Fluxo atual

```text
egui update
  └─ DocumentWorker
       ├─ Immediate → páginas no viewport e texto solicitado agora
       ├─ Visible → páginas vizinhas, links e caracteres
       ├─ Background → thumbnails e bookmarks
       └─ Search → busca incremental de menor prioridade
            ↓
       caches CPU por bytes → texturas GPU sob demanda
```

Nenhum pipeline de PDF pesado bloqueia a UI. O modo contínuo usa um índice
de layout com alturas estimadas, corrigidas conforme bitmaps reais chegam. Ele
aloca widgets e texturas somente para o range visível, mantém overscan no cache e
preserva a página âncora ao recalcular offsets. Render principal, busca,
miniaturas, bookmarks, links e extração textual compartilham o mesmo
`DocumentWorker` e o mesmo `PdfEngine` já aberto, sem workers ou documentos
paralelos por recurso. O scheduler mantém uma fila global limitada a 32 jobs,
prioridade estável, backpressure e cancelamento lógico por geração. Cada job
aceito termina como `ready`, `failed` ou `cancelled`, com duração, fila e
contadores registrados em debug.

A DLL e sua instância são únicas por processo. A inicialização usa exclusão
explícita antes de publicar o singleton, e `PdfEngine` serializa abertura,
renderização, extração e `Drop` por um mutex global. Isso respeita o contrato
single-threaded do PDFium sem devolver trabalho pesado para a thread da UI.

Os caches de página usam o mesmo LRU por bytes: bitmaps CPU têm orçamento padrão
de 64 MiB e texturas do modo contínuo, 128 MiB. Uma página individual maior que o
orçamento continua utilizável, mas permanece sozinha. Hits, misses, inserções,
evictions, uso residente e pico ficam disponíveis em métricas de debug.

No modo contínuo, o viewport sempre entra primeiro na fila. O overscan favorece a
direção do scroll e cresce em saltos rápidos, enquanto mantém uma página de
retaguarda para reversões imediatas.

## Contrato do scheduler implementado

```text
egui update (agenda e apresenta; nunca renderiza PDF)
  ↓
DocumentScheduler
  ├─ prioridade: Immediate > Visible > Background > Search
  ├─ generation_id e cancelamento
  ├─ backpressure e estados terminalmente observáveis
  └─ limite global de trabalho; memória limitada nos caches por bytes
  ↓
PDFium workers (chamadas nativas serializadas)
  ├─ render de página/região
  ├─ texto, links e bookmarks
  └─ resultados tipados
  ↓
Caches CPU por bytes → upload sob demanda → Cache GPU por bytes
```

## Abertura de documento

1. `PdfEngine::open` roda na thread da UI. É só o parse do cabeçalho e da
   xref: 7–10 ms medidos em todo o corpus, inclusive com 1.000 páginas. Um
   arquivo ausente, corrompido ou com senha falha aqui, com mensagem clara.
2. Cria-se um `DocumentWorker` novo (nova geração); resultados do documento
   anterior são descartados.
3. A página inicial é agendada com prioridade máxima. Vizinhas, bookmarks,
   links, chars e miniaturas entram atrás, sem bloquear input.
4. Cada resultado acorda a UI pelo *waker* do worker, inclusive com a janela
   ociosa.

## Renderização

- **Tamanho na tela:** depende só do zoom, com 100% = tamanho físico (96/72
  pontos de UI por ponto PDF).
- **Resolução do bitmap:** zoom efetivo × DPI da tela × qualidade
  (0,75×–2×), com o zoom **limitado ao que cabe na área útil**. Teto de
  8192 px por lado e 40 M pixels (`ui/page_scale.rs`).
- **Entrega:** o PDFium devolve RGBA direto (`reverse_byte_order`), sem uma
  segunda cópia.
- **LOD:** a textura anterior fica esticada até o bitmap novo chegar.
  Bitmaps abaixo de 85% da resolução útil (janela ampliada) são
  re-renderizados.
- **Contínuo:** virtualiza só o range visível + overscan direcional.
- **Tiles por região:** spike feito e correto, adiado para a v1.1 por decisão
  medida ([BF-3](benchmarks/bf3-tiles-spike.md)).
- **Backend gráfico:** DX12 com GL de reserva, GPU de baixo consumo (−95 MB e
  −90 ms em relação ao Vulkan).

## Estrutura relevante do código

```text
src/
├── main.rs                  entrada, fontes, janela e backend gráfico
├── app.rs, app_*.rs         estado e coordenação (toolbar, leitor, overlays…)
├── app_frame.rs, app_frame/ Modo Flutuante (sem bordas) e sua barra no hover
├── app_window.rs            posição/tamanho/maximização da janela
├── storage.rs               paths, schema, escrita atômica e recuperação de TOML
├── notices.rs               avisos de fora da UI → toasts
├── text_selection.rs        regras puras de seleção e cópia
├── updater.rs               atualização manual verificada (Ed25519 + SHA-256)
├── bench.rs, soak.rs        medidor de produto e teste de longa duração
├── showcase.rs              capturas da própria janela para a documentação
├── byte_cache.rs            LRU por bytes e métricas
├── pdf/
│   ├── engine.rs            integração PDFium (render, região, limites)
│   ├── document_worker.rs   scheduler priorizado, cancelável, com waker e catch_unwind
│   ├── extract.rs           chars, camada de busca, links e bookmarks
│   ├── page_cache.rs        bitmaps por página (com upgrade de resolução)
│   ├── thumb_cache.rs       miniaturas
│   ├── text_search.rs       busca incremental com geometria e contexto
│   └── persistence.rs       sessões e janela
├── ui/
│   ├── page_layout.rs       offsets, alturas e âncoras do viewport virtual
│   ├── page_scale.rs        tamanho exibido × resolução do bitmap
│   ├── scroll_motion.rs     contrato do Scroll Suave
│   ├── prefetch.rs          prioridade por direção/velocidade do scroll
│   ├── texture_cache.rs     orçamento de texturas GPU
│   ├── viewer*.rs           Single Page e Continuous
│   └── side_panel_*.rs, settings_*.rs, icons.rs
├── i18n/                    Fluent PT-BR/EN com guardas de teste
└── platform/                tema, associação .pdf, placement de janela, memória
```

## Persistência

Implementada em `storage.rs` (BF-5):

- **Paths:** centralizados em um só lugar. `MARIPDF_DATA_DIR` isola o estado
  em bench, smoke e testes.
- **Schema:** `schema_version` em settings, sessões e janela.
- **Escrita atômica:** temporário → `sync_all` → rename.
- **TOML corrompido:** é movido para `*.corrupt-<unix>.toml`, o app volta ao
  padrão e avisa o usuário.
- **Erros:** falhas de gravação viram toast, não somem.

## Segurança e isolamento

- **Limites:** páginas com dimensão inválida são recusadas, e os bitmaps
  respeitam o teto de textura e memória.
- **Panics:** um panic de job vira falha daquele job (`panic = "unwind"` +
  `catch_unwind`), e o lock do PDFium se recupera de envenenamento.
- **Contínuo:** uma página que falha vira placeholder local; o resto do
  documento segue navegável.
- **DLL:** a `pdfium.dll` extraída é validada pelo conteúdo inteiro.
- **Isolamento:** PDFium em processo separado foi decidido contra na v1.0,
  com gatilhos de reavaliação ([ADR-007](adr/ADR-007_isolamento_pdfium.md)).

## Performance

- **Números atuais e método:**
  [baseline-2026-10-19](benchmarks/baseline-2026-10-19.md).
- **Medidor:** embutido (`MARIPDF_BENCH`), roda no corpus sintético versionado.
- **Smoke test:** roda no CI.

## Distribuição

Ver [doc/10_distribuicao.md](10_distribuicao.md):

- **Artefatos:** EXE portátil e instalador Inno Setup **por usuário** (sem
  admin/UAC; substitui o MSI planejado).
- **Associação `.pdf`:** pelo fluxo do Windows 10/11.
- **Atualização:** manual e verificada (Ed25519).
- **Release:** por tag, com SBOM e rascunho de release.
- **Winget:** manifestos gerados e validados.

## Licença

O código do MariPDF declara `GPL-3.0-or-later`. PDFium possui licença permissiva e
não transforma o binário em AGPL. Todas as dependências conservam seus avisos e
licenças próprios.

Qualquer migração futura para MuPDF exige uma nova análise de licença antes de
entrar no produto.
