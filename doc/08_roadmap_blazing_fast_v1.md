# MariPDF — Roadmap Blazing Fast e v1.0 Redonda

> Criado em: 2026-07-13  
> Atualizado em: 2026-10-19
> Baseado em: auditoria do código, documentação, build release, testes e Clippy  
> Documento complementar ao roadmap histórico `04_roadmap.md`

## 1. Objetivo

Levar o MariPDF do estado atual — MVP visualmente avançado e funcional — até uma
v1.0 que possa sustentar, com evidência, a promessa:

> **Um leitor de PDF para Windows que abre rápido, nunca trava a interface e
> continua fluido em documentos grandes.**

“Redondo” neste roadmap significa:

- nenhuma operação pesada de PDF bloqueia a thread da UI;
- navegação e zoom permanecem responsivos em PDFs grandes;
- toda configuração exposta funciona e persiste;
- recursos centrais têm testes e métricas;
- build, empacotamento, atualização e integração Windows são reproduzíveis;
- documentação descreve o produto real, não uma arquitetura histórica.

## 2. Diagnóstico confirmado em 2026-07-13

### O que já está sólido

- Rust + egui/wgpu + PDFium formam uma base apropriada para a v1.0.
- PDFium é carregado uma vez e a DLL embarcada torna o app portátil.
- Há cache LRU, pré-render em background e cache separado de miniaturas.
- Leitura single/continuous, zoom, busca, seleção, links, TOC, miniaturas,
  recentes, temas, persistência, associação `.pdf`, logs e i18n já possuem
  implementação utilizável.
- `cargo build --release` conclui com sucesso.

### Evidência do baseline inicial da auditoria

| Gate | Resultado atual |
|---|---|
| `cargo build --release` | Passa com 14 warnings |
| `cargo test --all-targets` | Passa, mas executa 0 testes |
| `cargo clippy --all-targets --all-features -- -D warnings` | Falha com 33 ocorrências |
| CI no repositório | Ausente |
| Binário release local | 16,85 MB |
| `pdfium.dll` embarcada | 6,79 MB |
| Working set em smoke local | ~72,6 MB; precisa de benchmark controlado |
| Versão declarada | 0.1.0 |

Os números de memória e inicialização acima são somente baseline local. Antes de
virarem metas públicas, precisam ser repetidos em um protocolo controlado.

### Estado validado após a execução de 2026-07-13

| Gate | Resultado atual |
|---|---|
| `cargo fmt -- --check` | Passa |
| `cargo check` | Passa sem warnings |
| `cargo test --all-targets` | 31 passam; 5 testes de corpus/benchmark são manuais/ignorados |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passa |
| `cargo build --release` | Passa sem warnings |
| Stress PDFium no corpus local | 8 PDFs × 3 ciclos; render/texto/dimensões concorrentes em engine compartilhada |
| Smoke do processo | `MariPDF` aberto e `Responding=True` após rebuild |

### Gaps de núcleo e desempenho levantados na auditoria

Os itens marcados foram corrigidos durante a execução deste roadmap.

1. [x] O modo Single Page renderizava sincronamente na thread da UI.
2. [x] Fit Width e Fit Page renderizavam uma página apenas para obter dimensões e,
   depois, renderizavam novamente no zoom final.
3. [x] Busca full-text percorria e extraía todas as páginas na thread da UI.
4. [x] Extração de caracteres para seleção acontecia na thread da UI; agora usa
   job visível, deduplicado e aplicado somente à página ainda ativa.
5. [x] Bookmarks eram extraídos durante a abertura; agora chegam por job de fundo.
6. [x] O modo contínuo iterava `0..page_count` e montava placeholders para todas as
   páginas a cada frame; agora aloca apenas o range visível em slots estimados.
7. [x] Render, thumbnails, busca e metadados agora compartilham um scheduler
   priorizado, bounded e cancelável por geração.
8. [x] Cache era limitado por quantidade de páginas, não por bytes de CPU/GPU.
9. [x] Se o canal de resultado estivesse cheio, um bitmap podia ser descartado sem
   remover a página de `inflight`, deixando-a sem nova tentativa.
10. [x] Single Page, Continuous, Thumbnails e busca mantinham documentos separados;
    agora os pipelines lazy compartilham o mesmo `Arc<PdfEngine>` já aberto.
11. [x] O pipeline rasterizava a página inteira no zoom pedido; agora limita ao
    tamanho exibido (BF-3). Tiles por região ficam para a v1.1.
12. [x] A conversão BGRA → RGBA criava um segundo buffer e ainda revertia canais
    já normalizados; o render agora solicita RGBA e reutiliza a única cópia.
13. [x] A inicialização do singleton podia correr entre workers e bloquear para
    sempre no lock interno do PDFium.
14. [x] Workers chamavam simultaneamente uma engine nativamente single-threaded,
    causando `FormatError` ao abrir e `Unknown` ao carregar/renderizar páginas.

### Gaps funcionais confirmados

1. [x] Qualidade de renderização era salva, mas o renderer usava escala fixa.
2. [x] Margem de página era salva, mas o viewer usava constantes fixas.
3. [x] Scroll suave era salvo, mas não governava o comportamento.
4. [x] Painel padrão era salvo, mas não era aplicado ao abrir documento.
5. [x] Desativar histórico não impedia gravação de sessões/recentes.
6. [x] O botão “limpar histórico” não executava nenhuma ação.
7. [x] Escala e fonte salvas não eram reaplicadas integralmente no startup.
8. [x] Modo de cor não era persistido; agora há preferência salva com opção
   “Sistema”, que segue o tema do Windows no startup.
9. [x] O modo contínuo não mantinha de forma confiável a página atual visível para
   status, navegação e persistência.
10. [x] Links só eram acionados quando existia exatamente um na página; o clique
    agora é convertido para coordenadas PDF e testado contra cada retângulo.
11. [x] Busca navegava até a página sem destacar a ocorrência; agora destaca a
    ocorrência ativa e mostra o trecho de contexto.
12. [x] A seleção por arraste copiava ao soltar; agora arrastar só seleciona e a
    cópia é explícita (Ctrl+C/menu), com palavra, linha e Ctrl+A testados.
13. [x] Persistência de janela salvava largura aproximada e altura fixa; agora
    guarda posição, tamanho e maximização reais e recupera monitores ausentes.
14. [x] Havia strings PT-BR hardcoded fora do Fluent; a UI em inglês agora cobre
    a mesma superfície, protegida por testes.

### Gaps de robustez, produto e distribuição

1. A cobertura ainda é parcial: há 26 testes automáticos, stress de corpus e
   microbenchmark manuais, mas faltam manifesto versionado, integração de UI e
   benchmarks completos de produto.
2. Não há CI, build reproduzível, artefatos ou checksums automatizados.
3. Auto-update e doação são stubs.
4. Não há MSI, assinatura, manifesto Winget ou fluxo de desinstalação validado.
5. README, CHANGELOG, ADRs e Cargo divergem sobre PDFium versus MuPDF.
6. README e Cargo ainda contêm URLs e dados de doação placeholder.
7. O README promete aproximadamente 25 MB de RAM e instalador de 8–10 MB sem
   benchmark que sustente esses números.
8. [x] Escritas TOML não eram atômicas e erros eram descartados; agora `storage.rs`
   grava temp → sync → rename, recupera TOML corrompido com backup e avisa o usuário.
9. A associação de arquivo precisa registrar corretamente o aplicativo e orientar
   o usuário pelo fluxo do Windows, sem prometer que o app consegue forçar o
   `UserChoice` moderno.
10. [x] Não existia política para PDF malformado, crash do engine ou isolamento;
    ver testes de robustez e ADR-007.

## 3. Decisões arquiteturais para a v1.0

### D-01 — Manter PDFium na v1.0

Não migrar para MuPDF durante este roadmap. PDFium não é o gargalo estrutural
principal, já está integrado e evita somar risco técnico/licenciamento à release.
MuPDF poderá ser reavaliado depois com benchmark A/B e decisão de licença.

### D-02 — Nenhuma operação de PDF na thread da UI

Render, texto, bookmarks, links, miniaturas, metadados e busca devem passar por
jobs assíncronos. A UI apenas agenda trabalho, apresenta o último estado válido e
consome resultados prontos.

### D-03 — Scheduler único com prioridade e cancelamento

Ordem de prioridade:

1. página/região visível;
2. próxima e anterior;
3. caracteres e links da página visível;
4. miniaturas visíveis;
5. busca e indexação;
6. prefetch distante.

Cada abertura, troca de documento, zoom ou modo cria uma nova `generation_id`.
Resultados de gerações antigas são descartados sem alterar a UI ou o cache.

### D-04 — Cache limitado por memória

Substituir “12 páginas” por orçamento explícito em bytes, contabilizando bitmap
CPU e textura GPU. O orçamento deve ter defaults seguros e se adaptar ao zoom.

### D-05 — Verdade medida

Nenhuma promessa pública de tempo, RAM ou tamanho será mantida no README sem um
benchmark reproduzível e resultado de release associado.

## 4. Sequência de execução

## Fase BF-0 — Baseline, contratos e gates

**Objetivo:** criar uma base em que regressões fiquem visíveis antes de mexer no
renderer.

### Entregas

- [x] BF-001 Criar corpus versionado por manifesto, sem incluir PDFs restritos:
  pequeno textual, imagens/scans, 100 páginas, 1.000 páginas, bookmarks, muitos
  links, fontes complexas, corrompido, criptografado e páginas de tamanhos mistos. Gerador `tools/make_corpus.py` + manifesto `bench/corpus.toml` (10 PDFs determinísticos).
- [x] BF-002 Criar medidor de `startup → home`, `open → first_page_presented`,
  render por página, navegação cached/uncached, zoom, busca, working set e pico. Medidor embutido `MARIPDF_BENCH` + `tools/bench.ps1`/`bench_report.py`.
- [x] BF-003 Registrar baseline em hardware e build identificados. Ver `doc/benchmarks/baseline-2026-10-19.md`.
- [x] BF-004 Adicionar testes unitários para persistência, settings, LRU, i18n,
  busca e conversões de coordenadas. Persistência, settings, LRU, i18n, busca, seleção e coordenadas cobertos.
- [x] BF-005 Adicionar CI Windows com `cargo fmt --check`, `cargo check`, testes,
  Clippy acordado e build release.
- [x] BF-006 Zerar warnings do build e definir política de lints pragmática.
- [x] BF-007 Criar smoke test que abre o app, abre um PDF conhecido e confirma
  processo responsivo sem panic no log. `tools/smoke.ps1`, também no CI.
- [x] BF-008 Congelar os contratos funcionais atuais em uma matriz de testes. Ver `doc/09_matriz_contratos.md`.
- [x] BF-009 Adicionar stress manual parametrizado por corpus real para abrir,
  renderizar e fechar vários documentos concorrentemente no mesmo processo.

### DoD

- CI verde em commit limpo.
- Pelo menos um teste real por módulo crítico.
- Baseline de release publicado no repositório.
- Toda mudança posterior consegue ser comparada ao baseline.

## Fase BF-1 — Scheduler e UI sempre responsiva

**Objetivo:** remover trabalho pesado da thread do egui.

### Entregas

- [x] BF-101 Definir `DocumentWorker`, `RenderJob`, `JobPriority`, `JobResult` e
  `generation_id` atrás de uma interface independente do PDFium.
- [x] BF-102 Implementar fila priorizada, cancelamento lógico e backpressure.
- [x] BF-103 Migrar Single Page para render assíncrono.
- [x] BF-104 Manter a última textura válida durante zoom/navegação e exibir loading
  não bloqueante somente quando não houver textura utilizável.
- [x] BF-105 Migrar bookmarks, links, extração de caracteres e seleção de texto
  integral para `DocumentWorker`, sem bloquear o loop da UI.
- [x] BF-106 Migrar busca para execução incremental assíncrona, entregando hits por
  página e permitindo cancelamento ao mudar o termo.
- [x] BF-107 Unificar render principal e thumbnails sob limites globais de carga.
- [x] BF-108 Corrigir jobs/resultados descartados para nunca deixarem `inflight`
  preso; todo job termina em `ready`, `failed` ou `cancelled`.
- [x] BF-109 Remover render duplicado de Fit Width/Fit Page usando dimensões PDF.
- [x] BF-110 Instrumentar duração e fila dos jobs em logs de debug.
- [x] BF-111 Serializar todas as operações nativas do PDFium e tornar atômica a
  inicialização do singleton, incluindo fechamento de documentos sob o lock.

### DoD

- Nenhuma chamada a `render_page`, extração de texto/chars/bookmarks ou varredura
  completa de páginas parte do loop da UI.
- Trocar rapidamente página/zoom/documento não mostra resultado obsoleto.
- Busca pode ser cancelada sem congelar a janela.
- Teste de stress de 60 segundos não deixa job permanentemente `inflight`.

## Fase BF-2 — Virtualização, cache e navegação de documentos grandes

**Objetivo:** fazer custo por frame depender do viewport, não do número total de
páginas.

### Entregas

- [x] BF-201 Criar índice de layout com altura estimada/real por página.
- [x] BF-202 Renderizar e alocar widgets somente para o range visível + overscan.
- [x] BF-203 Remover loops por `page_count` do hot path de cada frame.
- [x] BF-204 Atualizar `current_page` pela página dominante no viewport contínuo.
- [x] BF-205 Preservar âncora visual ao trocar zoom, modo ou dimensões conhecidas.
- [x] BF-206 Implementar cache CPU e GPU por bytes, com métricas hit/miss/eviction.
- [x] BF-207 Priorizar prefetch conforme direção e velocidade do scroll.
- [x] BF-208 Compartilhar o mesmo documento aberto entre render principal,
  thumbnails e busca, sem reabrir/reprocessar o arquivo por pipeline.
- [x] BF-209 Eliminar a segunda alocação/conversão BGRA → RGBA. Microbenchmark
  release local, 16 × 2560×1440: 211,3 ms → 77,1 ms no handoff (-63,5%).

### DoD

- PDF de 1.000 páginas não cria 1.000 widgets/texturas por frame.
- Tempo de frame não cresce linearmente com o total de páginas fora da viewport.
- Página atual, status e persistência ficam corretos no modo contínuo.
- Memória estabiliza dentro do orçamento após navegação prolongada.

## Fase BF-3 — Tiles e níveis de detalhe

**Objetivo:** manter zoom alto e páginas gigantes responsivos.

### Entregas

- [x] BF-301 Fazer spike de render por região/tile no PDFium usado pelo projeto. Ver `doc/benchmarks/bf3-tiles-spike.md`; tile bate pixel a pixel com a página.
- [→ v1.1] BF-302 Definir tiles por página, escala e coordenadas, com chave de cache. Adiado pela decisão do gate.
- [→ v1.1] BF-303 Renderizar primeiro tiles visíveis e compor a página no painter. Adiado pela decisão do gate.
- [x] BF-304 Implementar LOD: preview de baixa resolução seguido de refinamento. Atendido: textura anterior esticada até o bitmap novo chegar.
- [→ v1.1] BF-305 Cancelar tiles que saíram da viewport ou perderam geração. Adiado pela decisão do gate.
- [x] BF-306 Medir overhead; manter fallback de página inteira para zoom baixo. Medido; página inteira limitada ao tamanho exibido é o caminho da v1.0.

### Gate de decisão

**Decisão (2026-10-19):** limitar a rasterização ao tamanho exibido entra na v1.0
(4× menos pixels, 2,7–4,5× mais rápido em zoom alto); tiles ficam para a v1.1.
Dados em `doc/benchmarks/bf3-tiles-spike.md`.

Tiles só entram na v1.0 se o spike provar ganho material em PDFs grandes/zoom alto
sem degradar o caso comum. Caso contrário, a fase fica como v1.1 e a v1.0 usa
page rendering assíncrono com LOD simples.

### DoD

- Zoom alto não exige bitmap da página inteira quando exceder o limite definido.
- Pan mostra preview imediato e refina sem bloquear a UI.
- Cache de tiles respeita orçamento de memória.

## Fase BF-4 — Correção funcional e settings verdadeiras

**Objetivo:** fazer cada controle exposto cumprir seu contrato.

### Entregas

- [x] BF-401 Aplicar escala, fonte, idioma e tema persistidos no startup.
- [x] BF-402 Ligar qualidade de render ao scheduler/LOD: qualidade e DPI definem só a
  resolução do bitmap (com teto de textura/memória); o tamanho na tela depende só do
  zoom, com 100% = tamanho físico. LOD por tiles segue na BF-304.
- [x] BF-403 Ligar margem de página ao layout single/continuous.
- [x] BF-404 Definir e implementar comportamento real de scroll suave.
- [x] BF-405 Aplicar painel padrão ao abrir documento.
- [x] BF-406 Respeitar `track_history` nas sessões e na Home.
- [x] BF-407 Implementar “limpar histórico” com confirmação e feedback.
- [x] BF-408 Persistir modo de cor ou oferecer explicitamente “seguir sistema”.
- [x] BF-409 Implementar hit-test correto para todos os links da página, com
  conversão tela ↔ PDF coberta por testes de múltiplos retângulos.
- [x] BF-410 Destacar ocorrência ativa da busca e expor trecho de contexto.
- [x] BF-411 Fechar contrato de seleção/cópia e cobrir mouse + teclado.
- [x] BF-412 Persistir tamanho, posição, maximização e monitor com recuperação
  segura quando a topologia de monitores mudar.
- [x] BF-413 Completar i18n e remover strings de UI hardcoded.
- [x] BF-414 Remover módulos stubs mortos ou conectá-los explicitamente.
- [x] BF-415 Modo Flutuante (antes “Moldura”): exibir o PDF sem janela, contido num frame, no
  estilo PureRef. Escopo:
  - janela sem decoração do Windows, borda fina do tema e página ocupando
    toda a área, com a janela travada na proporção da página;
  - arrastar a página move a janela; bordas e cantos redimensionam;
  - roda troca de página e Ctrl + roda dá zoom/pan dentro da moldura,
    mantendo o contrato do resto do app;
  - menu de botão direito com Sempre no topo, navegação, Ir para página,
    Abrir arquivo, Sair do modo e Fechar; atalho dedicado e botão na toolbar;
  - reaproveita o pipeline de Single Page (worker, cache e textura), sem
    novo caminho de render;
  - sem persistência por padrão: cada sessão abre no modo normal. Lembrar
    modo, posição, tamanho e Sempre no topo fica como opção futura em
    Configurações, desligada por padrão;
  - fora do escopo inicial: transparência/opacidade da janela (exige janela
    transparente na criação e chamada Win32 fora do eframe);
  - decidido: a moldura segue o tema do app, incluindo a inversão no Dark;
  - atalho Ctrl+Shift+F (Ctrl+Shift+M já alterna Single/Continuous); Esc sai;
  - controles flutuantes discretos: chevron semitransparente que fica opaco
    no hover e expande zoom, mão de pan, fixar no topo e sair;
  - pendente: Sempre no topo ainda não é lembrado entre sessões, conforme a
    decisão de persistência acima.

### DoD

- Cada setting possui teste de persistência e teste de efeito observável.
- Nenhum botão visível é no-op.
- PT-BR e EN cobrem a mesma superfície.
- Links múltiplos e busca destacada passam no corpus funcional.

## Fase BF-5 — Persistência, erros e resiliência

**Objetivo:** evitar perda de estado e transformar falhas em estados recuperáveis.

### Entregas

- [x] BF-501 Centralizar diretório, schema e versão de settings/sessões.
- [x] BF-502 Implementar escrita atômica `temp → flush → rename`.
- [x] BF-503 Fazer backup/recovery de TOML corrompido e logar motivo.
- [x] BF-504 Propagar erros hoje descartados e mostrar feedback quando acionável.
- [x] BF-505 Definir limites para páginas, dimensões, alocações e zoom.
- [x] BF-506 Testar PDF corrompido, senha, arquivo removido, path Unicode, rede lenta
  e falta de espaço/permissão.
- [x] BF-507 Tratar falha/panic de worker sem derrubar silenciosamente a feature.
- [x] BF-508 Decidir, por threat model, se isolamento do PDFium em processo separado
  é requisito de v1.x; documentar custo e benefício. Decisão: ADR-007.

### DoD

- Interromper o app durante gravação não destrói a última configuração válida.
- PDF inválido não derruba o processo principal.
- Falhas de worker são recuperáveis ou apresentadas com ação clara.

## Fase BF-6 — Windows e distribuição profissional

**Objetivo:** produzir uma instalação confiável e atualizável.

### Entregas

- [ ] BF-601 Definir identidade final: publisher, URLs, licença e canais de suporte.
- [ ] BF-602 Gerar ícone `.ico` correto e validar recursos/version info do EXE.
- [ ] BF-603 Criar pipeline para EXE portátil e MSI com instalação/desinstalação.
- [ ] BF-604 Registrar `ProgID`, `OpenWithProgids`, capabilities e fluxo correto para
  o usuário escolher o app padrão no Windows.
- [ ] BF-605 Assinar EXE/MSI ou documentar conscientemente a estratégia inicial.
- [ ] BF-606 Implementar update assinado/verificado, com rollback e rollout manual.
- [ ] BF-607 Gerar SHA-256, SBOM e artefatos a partir de tag.
- [ ] BF-608 Testar instalação limpa, upgrade, downgrade bloqueado, desinstalação e
  portable em Windows 10/11.
- [ ] BF-609 Preparar manifesto Winget depois de o MSI estar estável.
- [ ] BF-610 Implementar doação apenas depois do core/release, sem modal agressivo.

### DoD

- Uma tag produz artefatos reproduzíveis e checksums.
- MSI instala, associa, atualiza e desinstala sem resíduos indevidos.
- Portable não depende de arquivos pré-instalados.
- Update falho não deixa o app inutilizável.

## Fase BF-7 — Verdade documental e release candidate

**Objetivo:** alinhar promessa, código e evidência antes da v1.0.

### Entregas

- [x] BF-701 Atualizar ADR da engine para refletir PDFium e registrar MuPDF como
  alternativa futura, incluindo implicações de licença.
- [x] BF-702 Reconciliar arquitetura, roadmap histórico, manifesto e CHANGELOG.
- [x] BF-703 Remover placeholders do README/Cargo/Funding.
- [ ] BF-704 Publicar benchmarks reais e máquina de referência.
- [x] BF-705 Substituir claims comparativos não medidos por resultados reproduzíveis.
- [ ] BF-706 Criar screenshots e GIF curto do fluxo real.
- [ ] BF-707 Executar matriz RC em PDFs e versões Windows suportadas.
- [ ] BF-708 Fazer soak test prolongado com navegação, zoom, busca e troca de docs.
- [ ] BF-709 Congelar escopo, corrigir somente blockers e publicar RC.

### DoD da v1.0

- CI, testes, Clippy acordado e build release verdes.
- Zero operação pesada de PDF na UI.
- Nenhum setting ou botão visível sem efeito.
- Benchmarks atendem aos SLOs aprovados.
- Instalador/portable e atualização passam na matriz Windows.
- README, CHANGELOG, ADRs e binário contam a mesma história.

## 5. SLOs propostos

Estes valores devem ser calibrados após BF-0. São gates internos iniciais, não
promessas públicas automáticas.

| Métrica | Gate inicial proposto |
|---|---|
| Cold start até home utilizável | p50 ≤ 500 ms; p95 ≤ 900 ms |
| Open até primeira página, PDF textual 10 MB | warm p50 ≤ 250 ms; p95 ≤ 500 ms |
| Navegação para página em prefetch | textura apresentada ≤ 50 ms |
| Navegação uncached comum | p95 ≤ 350 ms sem bloquear input |
| Long task na UI | nenhuma > 50 ms; p95 de frame ≤ 16,7 ms durante interação |
| Busca | primeiro resultado ≤ 300 ms; restante incremental e cancelável |
| PDF de 1.000 páginas | abrir sem custo linear de UI/layout |
| Memória | estabilizar no orçamento definido; sem crescimento contínuo |
| Crash/panic no corpus RC | zero |
| Binário/installer | medir e publicar; remover claim de 8–10 MB até comprovação |

## 6. Ordem crítica e paralelismo seguro

```text
BF-0 Baseline e gates
  └─ BF-1 Scheduler assíncrono
       ├─ BF-2 Virtualização/cache
       │    └─ BF-3 Tiles/LOD (gateado)
       └─ BF-4 Correção funcional/settings
            └─ BF-5 Resiliência
                 └─ BF-6 Distribuição
                      └─ BF-7 Release candidate
```

BF-4 pode avançar em paralelo com BF-2 após os contratos do scheduler estarem
estáveis. BF-6 pode preparar identidade e CI de artefatos cedo, mas update e MSI
final só devem fechar depois do core estabilizado.

## 7. Recorte recomendado de entregas

### Marco A — “Não trava”

BF-0 + BF-1 completos. Esta é a maior mudança de percepção do produto.

### Marco B — “Escala”

BF-2 completo e decisão de BF-3 tomada com benchmark.

### Marco C — “Tudo que aparece funciona”

BF-4 + BF-5 completos.

### Marco D — “Pode ser instalado por outra pessoa”

BF-6 + BF-7 completos e RC validada.

## 8. O que não fazer antes da v1.0

- Não migrar para MuPDF sem benchmark, plano de licença e protótipo comparável.
- Não adicionar anotações, edição, formulários avançados, CBZ/CBR ou plugins.
- Não publicar claims de RAM, startup ou tamanho baseados em estimativa.
- Não implementar tiles antes de o scheduler e a virtualização estarem medidos.
- Não chamar feature de concluída porque a UI existe; exigir efeito observável e
  teste correspondente.
- Não bloquear release por idiomas via CDN; PT-BR e EN embutidos são suficientes.

## 9. Primeira sequência de implementação recomendada

1. BF-001 a BF-007 — corpus, métricas, testes e CI.
2. BF-101/BF-102 — contrato do scheduler e geração/cancelamento.
3. BF-103/BF-104/BF-109 — Single Page assíncrono e Fit sem render duplicado.
4. BF-106/BF-108 — busca cancelável e lifecycle correto de jobs.
5. BF-201 a BF-206 — virtualização real e cache por memória.
6. Reexecutar benchmark e decidir BF-3.
7. BF-401 a BF-415 — fechar promessas funcionais e modo Flutuante.
8. BF-5, BF-6 e BF-7 — robustez, distribuição e release.

Esse recorte preserva a base visual já construída e concentra esforço no que
transforma o MariPDF em um produto realmente rápido, previsível e publicável.
