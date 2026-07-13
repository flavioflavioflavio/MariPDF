# Changelog

Mudanças relevantes do MariPDF são registradas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) e o
versionamento seguirá [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Leitura de PDF com PDFium.
- Modos Single Page e Continuous.
- Zoom, Fit Width, Fit Page e navegação por teclado.
- Busca full-text, seleção/cópia de texto e abertura de links.
- Painel de miniaturas e sumário/bookmarks.
- Temas Dark, Light e Sepia.
- Tela de recentes e persistência por documento.
- Configurações persistidas e i18n PT-BR/EN com Mozilla Fluent.
- Associação `.pdf` no Windows, About e logs locais.
- Stress manual parametrizado para abrir, renderizar e fechar um corpus real em
  concorrência dentro do mesmo processo.

### Changed

- PDFium é a engine oficial da v1.0; MuPDF permanece apenas como alternativa
  futura sujeita a benchmark e decisão de licença.
- O roadmap vigente passa a ser `doc/08_roadmap_blazing_fast_v1.md`.
- Render de páginas e busca saíram da thread da UI; o modo contínuo agora usa
  virtualização, índice incremental de layout e preservação de âncora.
- Busca e miniaturas abrem seus workers somente quando usadas.
- Caches de bitmaps CPU e texturas GPU agora usam orçamento em bytes, LRU e
  métricas de hits, misses, evictions, uso residente e pico.
- Prefetch contínuo prioriza o viewport e adapta o overscan à direção e à
  velocidade da rolagem.
- Acesso ao PDFium agora é serializado do `open` ao `drop`, respeitando o contrato
  single-threaded da engine; a inicialização do singleton também ficou atômica.
- Render principal, miniaturas e busca compartilham o mesmo documento aberto,
  eliminando I/O e parsing repetidos por pipeline.
- O render solicita RGBA diretamente ao PDFium e elimina uma segunda alocação e
  varredura do bitmap; microbenchmark local do handoff mediu redução de 63,5%.
- Bookmarks, links, caracteres de seleção e extração integral de texto agora usam
  um `DocumentWorker` priorizado, bounded, deduplicado e cancelável por geração.
- Jobs documentais expõem estados terminais e métricas de fila, duração, sucesso,
  falha e cancelamento nos logs de debug.

### Fixed

- A troca de arquivo só substitui o documento atual após a nova abertura passar;
  falhas preservam a leitura em andamento e registram o caminho no log.
- Drag-and-drop aceita extensões `.pdf` sem diferenciar maiúsculas e minúsculas.
- Canais vermelho e azul deixam de ser trocados novamente depois de o PDFium já
  ter produzido RGBA.

### Known limitations

- Bookmarks, links e extração de caracteres ainda possuem operações síncronas.
- Algumas configurações ainda não controlam integralmente o comportamento.
- Não há instalador, auto-update ou release pública suportada.
- O corpus de PDFs e os benchmarks controlados ainda estão sendo construídos.

## [1.0.0] — planejada

A release será criada somente quando o Definition of Done do roadmap BF estiver
integralmente atendido.
