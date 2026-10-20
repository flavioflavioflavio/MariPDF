# Matriz RC — MariPDF 1.0.0-rc.1 (BF-704 / BF-707 / BF-708)

Validação da release candidate na máquina de referência descrita em
[baseline-2026-10-19](baseline-2026-10-19.md): Ryzen 9 5950X, RTX 3080,
Windows 11 build 26200, escala 125%, 3 monitores. Build: `cargo build --release`
de `codex/v1-release` (commit `f8b9c17`).

## Resumo

| Área | Resultado |
|---|---|
| Gates (`fmt`, `clippy -D warnings`, testes) | ✅ 121 testes automáticos, rustc 1.99.1 (CI) e 1.93.1 |
| Testes manuais com PDFium real | ✅ 7/7: stress concorrente, worker, busca e spike de tiles |
| Smoke test do app real | ✅ local, e no CI com WARP (sem GPU) |
| Corpus (10 PDFs × 3 rodadas) | ✅ 8 abrem e passam o roteiro; senha e corrompido são recusados em ~12 ms |
| Soak de 20 min | ✅ sem panic, sem falha de render, memória estável |
| Instalador (21 verificações) | ✅ no Windows 11 |
| Windows 10 | ⚠️ **não executado**: sem máquina Windows 10 disponível |

## Corpus — medidor embutido (mediana das rodadas 2–3)

| PDF | Págs | OK | 1º frame frio (ms) | 1º frame (ms) | Abrir (ms) | 1ª página (ms) | Salto sem cache (ms) | Vizinha c/ prefetch (ms) | Zoom (ms) | Busca 1º hit (ms) | Frame p95 (ms) | Frame máx (ms) | Working set (MB) | Pico (MB) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bookmarks-60.pdf | 60 | ✅ | 582 | 570 | 8.5 | 71.5 | 16.7 | 12.7 | 45.8 | 8.7 | 7.4 | 9.8 | 219 | 259 |
| complex-fonts.pdf | 5 | ✅ | 597 | 578 | 8.6 | 79.7 | 15.1 | 14.1 | 21.6 | 55.3 | 6.5 | 12.6 | 277 | 295 |
| corrupt.pdf | 0 | ✅ recusado | 586 | 601 | 7.7 | — | — | — | — | — | 10.3 | 10.4 | 212 | 214 |
| encrypted.pdf | 0 | ✅ recusado | 582 | 584 | 7.8 | — | — | — | — | — | 10.4 | 10.8 | 212 | 214 |
| links-20.pdf | 20 | ✅ | 589 | 575 | 8.1 | 70.4 | 15.8 | 11.5 | 17.7 | 299.2 | 7.4 | 13.8 | 247 | 272 |
| mixed-sizes.pdf | 12 | ✅ | 620 | 603 | 8.7 | 78.8 | 19.0 | 13.0 | 42.5 | 9.1 | 6.2 | 9.7 | 265 | 272 |
| scan-10.pdf | 10 | ✅ | 587 | 575 | 8.2 | 86.2 | 69.4 | 67.9 | 83.7 | 249.8 | 6.0 | 9.8 | 256 | 273 |
| text-100.pdf | 100 | ✅ | 594 | 608 | 8.0 | 68.6 | 17.6 | 12.8 | 36.4 | 6.1 | 6.4 | 14.1 | 219 | 259 |
| text-1000.pdf | 1000 | ✅ | 621 | 608 | 9.4 | 75.3 | 20.9 | 18.3 | 38.9 | 8.5 | 6.2 | 10.4 | 220 | 260 |
| text-small.pdf | 2 | ✅ | 579 | 581 | 8.6 | 92.0 | 12.3 | 8.9 | 29.8 | 8.7 | 6.8 | 10.0 | 229 | 250 |

Em `links-20` e `scan-10`, a busca percorre o documento inteiro sem achar o
termo, porque esses PDFs não o contêm.

### SLOs (§5 do roadmap)

| Métrica | Gate | RC | |
|---|---|---|---|
| Cold start até o 1º frame | p50 ≤ 500 ms | ~570–620 ms | ❌ ~100 ms acima; janela + DX12 dominam (ver "Abertos") |
| Abrir até a 1ª página | p50 ≤ 250 ms | 8 ms para abrir + 68–92 ms para apresentar | ✅ |
| Página já em prefetch | ≤ 50 ms | 9–18 ms (scan: 68 ms) | ✅ exceto scan |
| Página sem cache | p95 ≤ 350 ms | 12–69 ms | ✅ |
| Nenhuma tarefa da UI > 50 ms | — | máx 14,1 ms | ✅ |
| Busca, 1º resultado | ≤ 300 ms | 6–55 ms; varredura vazia ≤ 299 ms | ✅ |
| 1.000 páginas sem custo linear | — | 1ª página em 75 ms (2 páginas: 92 ms) | ✅ |
| Crash/panic | zero | zero (30 execuções do corpus + soak) | ✅ |

## Soak — 20 minutos (BF-708)

`MARIPDF_SOAK_SECS=1200` com os 8 PDFs do corpus, incluindo o corrompido e o
com senha:

| Métrica | Valor |
|---|---|
| Ações (navegar, saltar, zoom, modo, busca) | 3.337 |
| Trocas de documento | 40, das quais 10 recusas esperadas (senha/corrompido) |
| Falhas de render | 0 |
| Panics no log | 0 |
| Working set, 1º quarto → último quarto | 303 MB → 307 MB |
| Tendência estável | −52 MB/h (sem vazamento; limite: ≤ 50 MB/h) |
| Faixa das amostras / pico do processo | 220–486 MB / 678 MB |

O pico de 678 MB acontece com o scan de 300 dpi em zoom alto no modo
contínuo, quando várias páginas grandes ficam ao mesmo tempo no orçamento dos
caches de CPU e GPU. A memória volta ao patamar estável, mas esse pico é o
próximo alvo de ajuste de orçamento.

## Instalador (BF-608)

`pwsh tools/installer_test.ps1`, 21/21 ✅ no Windows 11:

- instalação limpa sem admin, atalho e entrada de desinstalação;
- ProgID, Capabilities e RegisteredApplications corretos, sem sobrescrever o
  padrão legado de `.pdf`;
- upgrade 0.9.0 → 0.9.1 e upgrade para `1.0.0-rc.10` (versão com pré-release
  compila e instala);
- downgrade bloqueado: 0.9.1 → 0.8.0 e rc.10 → rc.2;
- desinstalação limpa, com HKCU idêntico ao estado anterior;
- associação recusada no setup e feita depois pelo app: removida na
  desinstalação;
- pasta escolhida com arquivos alheios: arquivos preservados, executável
  removido.

## Abertos para a 1.0.0 estável

1. **Windows 10:** executar `tools/installer_test.ps1`, `tools/smoke.ps1` e
   um soak curto numa máquina ou VM com Windows 10.
2. **Cold start (~600 ms):** medir separadamente a criação da janela, o DX12 e
   o registro de fontes, e reduzir para ≤ 500 ms.
3. **Pico de memória em scans:** ajustar o orçamento de cache em zoom alto.
4. **Assinatura Authenticode:** SignPath Foundation ou Azure Trusted Signing
   ([distribuição](../10_distribuicao.md)).
