# BF-3 — Spike de tiles e decisão (2026-10-19)

## Pergunta

A BF-3 só entra na v1.0 se o spike provar ganho material em PDFs grandes e
zoom alto, sem degradar o caso comum. Caso contrário, fica para a v1.1.

## BF-301: render por região no PDFium

- O `clip()` do pdfium-render limita o que é pintado, mas **aloca o bitmap da
  página inteira**. Não serve para tiles.
- Os handles brutos (`FPDF_PAGE` e `FPDF_BITMAP`) são `pub(crate)` no crate,
  então o truque clássico de offsets negativos em `FPDF_RenderPageBitmap` não
  está disponível.
- **Caminho que funciona:** `render_into_bitmap_with_config` num `PdfBitmap`
  do tamanho do tile, com a translação `(-x/scale, -y/scale)` em pontos. A
  matriz do usuário é composta antes da escala.
- **Teste de correção:** `pdf::tile_spike_tests::tile_matches_the_same_region_of_the_full_page`
  roda no CI e mostra que o tile bate pixel a pixel com o recorte da página
  inteira.

## Achado que muda a conta

No MariPDF a página **nunca fica mais larga que a área útil da janela**, nos
dois modos. Mesmo assim, o bitmap era rasterizado no zoom pedido. Em 400%,
isso significa ~22 milhões de pixels (~90 MB de RAM + VRAM) para exibir uma
página de ~1.500 px de largura.

## Medição

Release na máquina de referência (ver `baseline-2026-10-19.md`), página 2 de
cada PDF, zoom pedido de 400%, viewport de 1600×1000 pontos a 125%,
qualidade 1×.

```
MARIPDF_CORPUS=bench/corpus cargo test --release -- --ignored --nocapture tile_spike
```

| PDF | Caso | Bitmap (MPx) | Tempo (ms) |
|---|---|---|---|
| text-100 | página inteira no zoom pedido (antes) | 22,3 | 79,7 |
| text-100 | **página inteira limitada ao exibido** | **5,7** | **17,7** |
| text-100 | só a viewport (tiles) | 2,5 | 7,3 |
| scan-10 | página inteira no zoom pedido (antes) | 22,3 | 160,1 |
| scan-10 | **limitada ao exibido** | **5,7** | **58,8** |
| scan-10 | só a viewport (tiles) | 2,5 | 33,1 |
| complex-fonts | página inteira no zoom pedido (antes) | 22,3 | 50,0 |
| complex-fonts | **limitada ao exibido** | **5,7** | **12,4** |
| complex-fonts | só a viewport (tiles) | 2,5 | 5,3 |

## Decisão

1. **Entra na v1.0: limitar a rasterização ao tamanho exibido.** Usa
   `page_scale::effective_zoom` e `viewer::page_raster_scale`. Dá 4× menos
   pixels e é 2,7–4,5× mais rápido em zoom alto, com uma mudança pequena. Se a
   janela crescer depois do render, um bitmap abaixo de 85% da resolução útil
   é re-renderizado (`UPGRADE_RATIO`), e o teto de textura evita loop.
2. **Tiles (BF-302/303/305) ficam para a v1.1.** Com o limite aplicado, o pior
   caso medido (scan, 59 ms) já está muito dentro do SLO de navegação sem
   cache (350 ms). Os tiles trariam mais ~2×, mas exigem cache por tile,
   composição e cancelamento por viewport. Isso só compensa quando existir
   zoom horizontal além da largura da janela, com pan lateral.
3. **LOD (BF-304) já existe:** ao mudar zoom ou página, a textura anterior
   aparece esticada até o bitmap novo chegar, sem tela vazia.
4. **Fallback e overhead (BF-306):** a página inteira continua sendo o caminho
   único. O render por região fica pronto (`render_region`, hoje só em teste)
   para a v1.1.

## Efeito no medidor do app (mediana, DX12)

| Métrica | Antes do limite | Depois |
|---|---|---|
| Zoom 160% (text-100) | 45,7 ms | 30,0 ms |
| Zoom 160% (complex-fonts) | 40,2 ms | 22,5 ms |
| 1ª página (text-1000) | 74,0 ms | 59,9 ms |
| Pico de memória (faixa do corpus) | 272–324 MB | 258–302 MB |
