# MariPDF — Roadmap Blazing Fast e v1.0 Redonda

> Criado em: 2026-07-13  
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

### Evidência de baseline

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

### Gaps de núcleo e desempenho levantados na auditoria

Os itens marcados foram corrigidos durante a execução deste roadmap.

1. [x] O modo Single Page renderizava sincronamente na thread da UI.
2. [x] Fit Width e Fit Page renderizavam uma página apenas para obter dimensões e,
   depois, renderizavam novamente no zoom final.
3. [x] Busca full-text percorria e extraía todas as páginas na thread da UI.
4. [ ] Extração de caracteres para seleção acontece na thread da UI ao mudar página.
5. [ ] Bookmarks são extraídos sincronamente durante a abertura.
6. [ ] O modo contínuo ainda itera `0..page_count` e monta placeholders para todas as
   páginas a cada frame; não é virtualização real.
7. [ ] Não há scheduler unificado, prioridade, cancelamento ou geração de jobs.
8. [ ] Cache é limitado por quantidade de páginas, não por bytes de CPU/GPU.
9. [x] Se o canal de resultado estivesse cheio, um bitmap podia ser descartado sem
   remover a página de `inflight`, deixando-a sem nova tentativa.
10. [ ] Single Page, Continuous e Thumbnails abrem pipelines/documentos separados,
    sem coordenação de carga; busca e thumbnails agora são inicializados sob demanda.
11. [ ] O pipeline sempre rasteriza a página inteira; zoom alto não usa tiles/LOD.
12. [ ] A conversão BGRA → RGBA cria um novo buffer e percorre todos os pixels.

### Gaps funcionais confirmados

1. [x] Qualidade de renderização era salva, mas o renderer usava escala fixa.
2. [x] Margem de página era salva, mas o viewer usava constantes fixas.
3. [x] Scroll suave era salvo, mas não governava o comportamento.
4. [x] Painel padrão era salvo, mas não era aplicado ao abrir documento.
5. [x] Desativar histórico não impedia gravação de sessões/recentes.
6. [x] O botão “limpar histórico” não executava nenhuma ação.
7. [x] Escala e fonte salvas não eram reaplicadas integralmente no startup.
8. Modo de cor não é persistido; o tema do Windows vence em toda inicialização.
9. O modo contínuo não mantém de forma confiável a página atual visível para
   status, navegação e persistência.
10. Links só são acionados no caso simplificado de existir exatamente um link na
    página; falta hit-test por retângulo para múltiplos links.
11. Busca navega até a página, mas não destaca a ocorrência encontrada.
12. A seleção por arraste copia imediatamente ao soltar, comportamento que deve
    ser decidido e testado como contrato de UX.
13. Persistência de janela salva largura aproximada e altura fixa; não salva
    posição real, maximização ou monitor.
14. Há strings PT-BR hardcoded fora do Fluent, então a UI em inglês é parcial.

### Gaps de robustez, produto e distribuição

1. Não há testes unitários, integração, corpus de PDFs ou benchmarks.
2. Não há CI, build reproduzível, artefatos ou checksums automatizados.
3. Auto-update e doação são stubs.
4. Não há MSI, assinatura, manifesto Winget ou fluxo de desinstalação validado.
5. README, CHANGELOG, ADRs e Cargo divergem sobre PDFium versus MuPDF.
6. README e Cargo ainda contêm URLs e dados de doação placeholder.
7. O README promete aproximadamente 25 MB de RAM e instalador de 8–10 MB sem
   benchmark que sustente esses números.
8. Escritas TOML não são atômicas e erros de persistência são frequentemente
   descartados; queda durante escrita pode perder configurações.
9. A associação de arquivo precisa registrar corretamente o aplicativo e orientar
   o usuário pelo fluxo do Windows, sem prometer que o app consegue forçar o
   `UserChoice` moderno.
10. Não existe política explícita de suporte a PDF malformado, crash do engine ou
    isolamento do renderizador.

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

- [ ] BF-001 Criar corpus versionado por manifesto, sem incluir PDFs restritos:
  pequeno textual, imagens/scans, 100 páginas, 1.000 páginas, bookmarks, muitos
  links, fontes complexas, corrompido, criptografado e páginas de tamanhos mistos.
- [ ] BF-002 Criar medidor de `startup → home`, `open → first_page_presented`,
  render por página, navegação cached/uncached, zoom, busca, working set e pico.
- [ ] BF-003 Registrar baseline em hardware e build identificados.
- [ ] BF-004 Adicionar testes unitários para persistência, settings, LRU, i18n,
  busca e conversões de coordenadas.
- [x] BF-005 Adicionar CI Windows com `cargo fmt --check`, `cargo check`, testes,
  Clippy acordado e build release.
- [x] BF-006 Zerar warnings do build e definir política de lints pragmática.
- [ ] BF-007 Criar smoke test que abre o app, abre um PDF conhecido e confirma
  processo responsivo sem panic no log.
- [ ] BF-008 Congelar os contratos funcionais atuais em uma matriz de testes.

### DoD

- CI verde em commit limpo.
- Pelo menos um teste real por módulo crítico.
- Baseline de release publicado no repositório.
- Toda mudança posterior consegue ser comparada ao baseline.

## Fase BF-1 — Scheduler e UI sempre responsiva

**Objetivo:** remover trabalho pesado da thread do egui.

### Entregas

- [ ] BF-101 Definir `DocumentWorker`, `RenderJob`, `JobPriority`, `JobResult` e
  `generation_id` atrás de uma interface independente do PDFium.
- [ ] BF-102 Implementar fila priorizada, cancelamento lógico e backpressure.
- [x] BF-103 Migrar Single Page para render assíncrono.
- [x] BF-104 Manter a última textura válida durante zoom/navegação e exibir loading
  não bloqueante somente quando não houver textura utilizável.
- [ ] BF-105 Migrar bookmarks, links e extração de caracteres para jobs.
- [x] BF-106 Migrar busca para execução incremental assíncrona, entregando hits por
  página e permitindo cancelamento ao mudar o termo.
- [ ] BF-107 Unificar render principal e thumbnails sob limites globais de carga.
- [ ] BF-108 Corrigir jobs/resultados descartados para nunca deixarem `inflight`
  preso; todo job termina em `ready`, `failed` ou `cancelled`.
- [x] BF-109 Remover render duplicado de Fit Width/Fit Page usando dimensões PDF.
- [ ] BF-110 Instrumentar duração e fila dos jobs em logs de debug.

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

- [ ] BF-201 Criar índice de layout com altura estimada/real por página.
- [ ] BF-202 Renderizar e alocar widgets somente para o range visível + overscan.
- [ ] BF-203 Remover loops por `page_count` do hot path de cada frame.
- [ ] BF-204 Atualizar `current_page` pela página dominante no viewport contínuo.
- [ ] BF-205 Preservar âncora visual ao trocar zoom, modo ou dimensões conhecidas.
- [ ] BF-206 Implementar cache CPU e GPU por bytes, com métricas hit/miss/eviction.
- [ ] BF-207 Priorizar prefetch conforme direção e velocidade do scroll.
- [ ] BF-208 Evitar reabrir/reprocessar o documento sem necessidade entre pipelines.
- [ ] BF-209 Avaliar reutilização de buffer/conversão in-place BGRA → RGBA com
  benchmark antes/depois.

### DoD

- PDF de 1.000 páginas não cria 1.000 widgets/texturas por frame.
- Tempo de frame não cresce linearmente com o total de páginas fora da viewport.
- Página atual, status e persistência ficam corretos no modo contínuo.
- Memória estabiliza dentro do orçamento após navegação prolongada.

## Fase BF-3 — Tiles e níveis de detalhe

**Objetivo:** manter zoom alto e páginas gigantes responsivos.

### Entregas

- [ ] BF-301 Fazer spike de render por região/tile no PDFium usado pelo projeto.
- [ ] BF-302 Definir tiles por página, escala e coordenadas, com chave de cache.
- [ ] BF-303 Renderizar primeiro tiles visíveis e compor a página no painter.
- [ ] BF-304 Implementar LOD: preview de baixa resolução seguido de refinamento.
- [ ] BF-305 Cancelar tiles que saíram da viewport ou perderam geração.
- [ ] BF-306 Medir overhead; manter fallback de página inteira para zoom baixo.

### Gate de decisão

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

- [ ] BF-401 Aplicar escala, fonte, idioma e tema persistidos no startup.
- [ ] BF-402 Ligar qualidade de render ao scheduler/LOD (o pipeline atual já respeita a escala).
- [x] BF-403 Ligar margem de página ao layout single/continuous.
- [ ] BF-404 Definir e implementar comportamento real de scroll suave.
- [x] BF-405 Aplicar painel padrão ao abrir documento.
- [x] BF-406 Respeitar `track_history` nas sessões e na Home.
- [ ] BF-407 Implementar “limpar histórico” com confirmação e feedback (a limpeza já tem efeito imediato).
- [ ] BF-408 Persistir modo de cor ou oferecer explicitamente “seguir sistema”.
- [ ] BF-409 Implementar hit-test correto para todos os links da página.
- [ ] BF-410 Destacar ocorrência ativa da busca e expor trecho de contexto.
- [ ] BF-411 Fechar contrato de seleção/cópia e cobrir mouse + teclado.
- [ ] BF-412 Persistir tamanho, posição, maximização e monitor com recuperação
  segura quando a topologia de monitores mudar.
- [ ] BF-413 Completar i18n e remover strings de UI hardcoded.
- [ ] BF-414 Remover módulos stubs mortos ou conectá-los explicitamente.

### DoD

- Cada setting possui teste de persistência e teste de efeito observável.
- Nenhum botão visível é no-op.
- PT-BR e EN cobrem a mesma superfície.
- Links múltiplos e busca destacada passam no corpus funcional.

## Fase BF-5 — Persistência, erros e resiliência

**Objetivo:** evitar perda de estado e transformar falhas em estados recuperáveis.

### Entregas

- [ ] BF-501 Centralizar diretório, schema e versão de settings/sessões.
- [ ] BF-502 Implementar escrita atômica `temp → flush → rename`.
- [ ] BF-503 Fazer backup/recovery de TOML corrompido e logar motivo.
- [ ] BF-504 Propagar erros hoje descartados e mostrar feedback quando acionável.
- [ ] BF-505 Definir limites para páginas, dimensões, alocações e zoom.
- [ ] BF-506 Testar PDF corrompido, senha, arquivo removido, path Unicode, rede lenta
  e falta de espaço/permissão.
- [ ] BF-507 Tratar falha/panic de worker sem derrubar silenciosamente a feature.
- [ ] BF-508 Decidir, por threat model, se isolamento do PDFium em processo separado
  é requisito de v1.x; documentar custo e benefício.

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
7. BF-401 a BF-414 — fechar promessas funcionais.
8. BF-5, BF-6 e BF-7 — robustez, distribuição e release.

Esse recorte preserva a base visual já construída e concentra esforço no que
transforma o MariPDF em um produto realmente rápido, previsível e publicável.
