# Changelog

Mudanças relevantes do MariPDF são registradas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) e o
versionamento segue [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Alterado

- Home redesenhada: logo do app, botão de abrir com ícone, recentes num único
  bloco alinhado (ícone, nome, pasta com o final visível, página onde parou) e
  remoção que só aparece no hover. Rodapé de atalhos centralizado.
- Acento da marca passa a ser o violeta do ícone (antes laranja) na paleta
  padrão dos temas escuro, claro e sépia.
- *Sobre* agora é uma aba das Configurações (`Ctrl+Shift+I` abre direto nela);
  o botão ⓘ avulso saiu da barra de ferramentas.

### Corrigido

- Bordas translúcidas apareciam brancas nos temas claro e sépia (cor
  premultiplicada inválida).

## [1.0.0-rc.1] — 2026-10-19

Primeira release candidate da v1.0. Fecha o roadmap Blazing Fast (BF-0 a BF-7);
tiles por região ficam para a v1.1 (decisão medida em `doc/benchmarks/bf3-tiles-spike.md`).

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
- Preferência de tema persistida (Sistema, Dark, Light, Sépia); “Sistema” segue
  o tema do Windows no startup e qualquer troca pela toolbar ou Home é salva.
- “Limpar histórico” pede confirmação, permite cancelar e informa sucesso ou
  falha da gravação.
- Testes garantem que PT-BR e EN expõem as mesmas chaves Fluent, que toda chave
  usada no código existe e que widgets, tooltips e toasts não recebem texto fixo.
- Busca destaca a ocorrência ativa sobre a página (Single e Continuous) e mostra
  o trecho de contexto com o termo marcado; no modo contínuo a navegação
  centraliza a linha da ocorrência, não só o topo da página.
- Modo Flutuante (Ctrl+Shift+F ou botão na toolbar): o PDF vira a própria janela,
  sem decoração, travada na proporção da página. Arrastar move a janela, bordas
  redimensionam, roda troca de página, Ctrl + roda dá zoom no cursor com pan,
  duplo clique reenquadra e o botão direito abre o menu com Sempre no topo,
  navegação, abrir arquivo e sair. Segue o tema do app e não persiste estado.
- Controles flutuantes do modo Flutuante: chevron no canto superior direito, invisível
  fora da janela, semitransparente sobre a página e opaco no hover. Expande uma
  pílula com zoom −/%/+, mão de pan, fixar no topo e sair.
- Stress manual parametrizado para abrir, renderizar e fechar um corpus real em
  concorrência dentro do mesmo processo.

### Changed

- Distribuição: EXE portátil e instalador **por usuário** (sem admin/UAC), com
  ícone, metadados de versão, associação `.pdf` moderna (Capabilities +
  Aplicativos padrão), upgrade que fecha o app aberto e downgrade bloqueado.
- Atualização manual em *Sobre*: só consulta a rede ao clicar, baixa o
  instalador e só executa com assinatura Ed25519 e hash válidos.
- Release por tag gera portátil, instalador, SBOM CycloneDX, SHA256SUMS
  assinado e manifestos Winget num rascunho de release.
- Toolbar redesenhada em três zonas (documento, página centralizada e
  visualização), com um único estilo de botão, hover por baixo do ícone, grupos
  para página/zoom/modo, menu de zoom (100%, ajustar largura, ajustar página) e
  tooltips traduzidos com atalhos.
- Modo Moldura passa a se chamar Modo Flutuante.
- Seleção de texto: arrastar só seleciona; Ctrl+C ou o menu do botão direito
  copiam. Duplo/triplo clique selecionam palavra/linha, Shift+clique estende,
  Ctrl+A seleciona a página e Esc limpa.
- Scroll Suave passa a valer de fato no modo contínuo: ligado anima saltos de
  página; desligado aplica a roda instantaneamente.
- Rasterização limitada ao tamanho realmente exibido: zoom além da largura da
  janela não gera bitmaps maiores (4× menos pixels e 2,7–4,5× mais rápido a
  400%); bitmaps pequenos demais são re-renderizados ao ampliar a janela.
- Backend gráfico padrão passa a ser DX12 (GL de reserva) em GPU de baixo
  consumo: ~95 MB a menos de memória e ~90 ms a menos até o primeiro frame.
- Qualidade de render agora é relativa ao pixel físico (0,75× a 2×) e só muda a
  nitidez; 100% de zoom equivale ao tamanho físico do documento.
- A janela reabre com a posição, o tamanho e a maximização reais da sessão
  anterior (GetWindowPlacement, pixels físicos), gravados com debounce e
  escrita atômica; posições fora dos monitores atuais são trazidas para dentro
  ou centralizadas no principal. Flutuante e tela cheia não são gravados.
- Home, status bar, painel lateral, sumário automático, toasts, erros de
  abertura, diálogo de arquivo e rótulos das configurações passam pelo Fluent;
  a UI em inglês cobre a mesma superfície do PT-BR.
- PDFium é a engine oficial da v1.0; MuPDF permanece apenas como alternativa
  futura sujeita a benchmark e decisão de licença.
- O roadmap vigente passa a ser `doc/08_roadmap_blazing_fast_v1.md`.
- Render de páginas e busca saíram da thread da UI; o modo contínuo agora usa
  virtualização, índice incremental de layout e preservação de âncora.
- Render, miniaturas, busca e metadados passam pelo mesmo scheduler global
  priorizado; os workers dedicados por recurso foram removidos.
- Caches de bitmaps CPU e texturas GPU agora usam orçamento em bytes, LRU e
  métricas de hits, misses, evictions, uso residente e pico.
- Prefetch contínuo prioriza o viewport e adapta o overscan à direção e à
  velocidade da rolagem.
- Acesso ao PDFium agora é serializado do `open` ao `drop`, respeitando o contrato
  single-threaded da engine; a inicialização do singleton também ficou atômica.
- Render principal, miniaturas e busca compartilham o mesmo documento aberto e a
  mesma fila limitada, eliminando I/O, parsing e concorrência descoordenada.
- O render solicita RGBA diretamente ao PDFium e elimina uma segunda alocação e
  varredura do bitmap; microbenchmark local do handoff mediu redução de 63,5%.
- A busca recebe texto e geometria do PDFium em uma única passada, ignora
  maiúsculas/minúsculas e casa frases que atravessam quebras de linha.
- Todos os trabalhos PDF usam um `DocumentWorker` priorizado, bounded, deduplicado
  quando necessário e cancelável por geração.
- Jobs documentais expõem estados terminais e métricas de fila, duração, sucesso,
  falha e cancelamento nos logs de debug.

### Removed

- Stubs vazios (`ui/search`, `ui/donation`, `ui/recent_files`, `updater`,
  `toast.rs`), `.ftl` órfãos e o job `Text`, substituído pela camada de chars.
- `src/color_mode.rs`, cópia morta de `app_color_mode.rs` fora da árvore de módulos.

### Fixed

- Configurações, sessões e janela agora usam escrita atômica (temp → sync →
  rename); TOML corrompido é movido para `*.corrupt-<unix>.toml`, o app volta ao
  padrão e avisa. Falhas de gravação e da associação `.pdf` viram toasts.
- PDF protegido por senha, corrompido ou ausente mostra a mensagem certa;
  caminhos Unicode abrem normalmente (testado com fixtures versionadas).
- Panic em job do worker vira falha daquele job (`panic = "unwind"`), sem
  derrubar o app nem envenenar o lock do PDFium; páginas com dimensões
  inválidas são recusadas com erro claro.
- A `pdfium.dll` extraída é validada pelo conteúdo inteiro (não só tamanho) e
  gravada atomicamente.
- A qualidade de render alterava o tamanho da página na tela e telas HiDPI
  recebiam bitmaps ampliados; zoom alto em Ultra podia exceder o limite de
  textura da GPU.
- Ícones Phosphor apontavam para glifos errados (ex.: sumário exibia uma
  lâmpada); a tabela foi regenerada a partir do CSS oficial do Phosphor 2.1.
- A troca de arquivo só substitui o documento atual após a nova abertura passar;
  falhas preservam a leitura em andamento e registram o caminho no log.
- Drag-and-drop aceita extensões `.pdf` sem diferenciar maiúsculas e minúsculas.
- Canais vermelho e azul deixam de ser trocados novamente depois de o PDFium já
  ter produzido RGBA.
- Páginas com múltiplos links agora abrem o URI cujo retângulo contém o clique,
  em vez de aceitar somente o caso de link único.

### Known limitations

- Binários sem assinatura Authenticode: o SmartScreen avisa na primeira
  execução (integridade garantida por SHA256SUMS assinado com Ed25519).
- Instalador validado no Windows 11; Windows 10 ainda não passou pela matriz RC.
- Tiles por região e zoom horizontal além da largura da janela ficam para a v1.1.
- PDFium roda no mesmo processo (ADR-007): crash nativo do motor fecha o app.
- Seleção de texto com destaque visual só no modo página única.

## [1.0.0] — planejada

Publicada a partir da RC quando a matriz RC (incluindo Windows 10) estiver
completa e não houver bloqueadores abertos.
