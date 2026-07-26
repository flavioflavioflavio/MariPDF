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

1. Criar nova geração e invalidar resultados do documento anterior.
2. Abrir o documento em worker.
3. Entregar metadados mínimos e número de páginas.
4. Agendar página inicial com prioridade máxima.
5. Apresentar a primeira textura assim que pronta.
6. Agendar próxima/anterior e metadados secundários.
7. Extrair bookmarks, links, texto e miniaturas sem bloquear input.

## Renderização

PDFium rasteriza páginas com `reverse_byte_order` explícito e já entrega RGBA. O
MariPDF transfere esse único `Vec` para o cache, carrega uma textura no egui e
deixa o wgpu compor a interface; não há uma segunda alocação para trocar R/B.

Na v1.0:

- página visível tem prioridade máxima;
- textura anterior pode permanecer durante refinamento de zoom;
- Fit Width/Fit Page usam dimensões PDF sem render intermediário;
- modo contínuo virtualiza somente o range visível + overscan;
- tiles/LOD entram apenas se o benchmark BF-3 provar benefício.

## Estrutura relevante do código

```text
src/
├── main.rs                 entrada, fontes e janela
├── byte_cache.rs           LRU por bytes e métricas compartilhadas
├── app*.rs                 estado e coordenação da aplicação
├── pdf/
│   ├── engine.rs           integração PDFium
│   ├── document_worker.rs  scheduler global priorizado e cancelável
│   ├── extract.rs          texto, chars, links e bookmarks
│   ├── page_cache.rs       cache/render atual de páginas
│   ├── thumb_cache.rs      cache/render atual de miniaturas
│   ├── text_search.rs      busca textual atual
│   └── persistence.rs      sessões e janela
├── ui/
│   ├── page_layout.rs      offsets, alturas e âncoras do viewport virtual
│   ├── prefetch.rs         prioridade por direção/velocidade do scroll
│   ├── texture_cache.rs    orçamento e métricas de texturas GPU
│   ├── viewer.rs           viewer Single Page
│   ├── viewer_continuous.rs viewer Continuous
│   ├── side_panel_*.rs     miniaturas e TOC
│   └── settings_*.rs       painel de configurações
├── i18n/                   Fluent PT-BR/EN
└── platform/               tema e integração Windows
```

Extensões futuras do scheduler devem permanecer sob `src/pdf/` ou `src/render/`,
sem transferir lógica pesada para `src/ui/`.

## Persistência

O estado atual usa TOML nos diretórios retornados por `dirs`. A v1.0 deve:

- centralizar paths e versão de schema;
- escrever por arquivo temporário + flush + rename;
- recuperar backup quando o TOML estiver corrompido;
- respeitar a opção de histórico;
- persistir janela e settings com erros observáveis.

## Segurança e isolamento

PDFs são entrada não confiável. A v1.0 precisa de limites de dimensões/alocação,
tratamento de arquivos inválidos e workers recuperáveis. Isolar o PDFium em outro
processo é uma decisão de v1.x sujeita a threat model e benchmark; não é requisito
automático para a primeira release.

## Performance

O projeto não mantém mais estimativas antigas como fatos. O baseline de
2026-07-13 encontrou binário release de 16,85 MB e smoke local perto de 72,6 MB de
working set, mas a publicação de números exige protocolo BF-0 controlado.

Os SLOs internos propostos estão no roadmap BF e devem ser calibrados após o
primeiro benchmark reproduzível.

## Distribuição

Estado atual: build local de `.exe`; não há instalador ou updater suportado.

Direção aprovada:

1. EXE portátil e MSI gerados por tag;
2. SHA-256 e SBOM junto aos artefatos;
3. assinatura quando a estratégia de publisher estiver definida;
4. atualização verificada com rollback;
5. Winget somente depois do MSI estabilizado.

## Licença

O código do MariPDF declara `GPL-3.0-or-later`. PDFium possui licença permissiva e
não transforma o binário em AGPL. Todas as dependências conservam seus avisos e
licenças próprios.

Qualquer migração futura para MuPDF exige uma nova análise de licença antes de
entrar no produto.
