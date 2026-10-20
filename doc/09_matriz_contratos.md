# Matriz de contratos funcionais (BF-008)

Congela o comportamento que a v1.0 promete e aponta o teste que o protege.
Uma mudança que quebre um contrato deve atualizar esta matriz e o teste
correspondente no mesmo PR.

**Tipos de teste**

- **U:** teste unitário automático, roda no CI.
- **F:** teste automático com as fixtures de `tests/fixtures`, roda no CI.
- **M:** teste manual com PDFs reais (`MARIPDF_TEST_PDFS`, `--ignored`).
- **S:** smoke test do app real (`tools/smoke.ps1`).
- **B:** medidor de produto (`tools/bench.ps1`, ver `doc/benchmarks/`).

## Abertura e erros

| Contrato | Teste | Tipo |
|---|---|---|
| PDF válido abre, renderiza e expõe texto | `pdf::robustness_tests::sample_opens_renders_and_exposes_text` | F |
| PDF com senha mostra a mensagem de senha | `…::password_protected_pdf_reports_password_message` | F |
| PDF corrompido mostra a mensagem de corrompido, sem panic | `…::corrupted_pdf_reports_corruption_without_panicking` | F |
| Arquivo ausente mostra "não encontrado" | `…::missing_file_reports_not_found` | F |
| Caminho com Unicode/emoji abre | `…::unicode_path_opens` | F |
| Apagar o arquivo aberto não derruba o app | `…::file_removed_after_open_keeps_engine_usable` | F |
| Página com dimensão inválida é recusada | `…::invalid_page_sizes_are_rejected` | U |
| App abre um PDF, executa o roteiro e sai sem panic | `tools/smoke.ps1` | S |

## Worker e performance

| Contrato | Teste | Tipo |
|---|---|---|
| Fila priorizada: urgente primeiro, FIFO no empate | `pdf::document_worker::tests::priority_queue_prefers_urgent_jobs_and_preserves_fifo` | U |
| Jobs equivalentes são deduplicados | `…::job_keys_deduplicate_by_kind_and_page` | U |
| Fila cheia recusa sem contar o job | `…::bounded_queue_reports_backpressure_without_counting_rejected_job` | U |
| Todo job aceito termina e acorda a UI | `…::real_pdf_jobs_reach_terminal_state` | M |
| Troca de geração cancela jobs na fila | `…::generation_change_cancels_queued_real_pdf_jobs` | M |
| Panic em job vira falha recuperável | `pdf::robustness_tests::worker_guard_turns_panics_into_errors` | U |
| PDFium suporta uso concorrente serializado | `pdf::engine::tests::pdf_corpus_opens_renders_and_closes_concurrently` | M |
| Caches de CPU/GPU respeitam o orçamento em bytes (LRU) | `byte_cache::tests::*`, `ui::texture_cache::tests::*`, `pdf::page_cache_tests::*` | U |
| Prefetch segue a direção e a velocidade do scroll | `ui::prefetch::tests::*` | U |
| Nenhuma operação da UI passa de 50 ms no corpus | `tools/bench.ps1` (`Frame máx`) | B |

## Visualização

| Contrato | Teste | Tipo |
|---|---|---|
| 100% = tamanho físico; qualidade/DPI não mudam o tamanho | `ui::page_scale::tests::*` | U |
| Bitmap limitado a 8192 px / 40 M pixels | `…::huge_requests_are_capped_to_texture_and_memory_limits` | U |
| Contínuo: só o range visível, âncora preservada no zoom | `ui::page_layout_tests::*`, `ui::viewer_continuous_tests::programmatic_navigation_and_zoom_preserve_continuous_anchor` | U |
| Scroll suave anima saltos; desligado é instantâneo | `ui::viewer_continuous_tests::smooth_scroll_animates_jumps_and_lands_on_the_target`, `ui::scroll_motion::tests::*` | U |
| Conversão PDF ↔ tela (origem inferior esquerda) | `ui::viewer_helpers::tests::pdf_rect_maps_with_bottom_left_origin` | U |
| Clique em link acerta o retângulo certo entre vários | `app_update::tests::hit_test_*` | U |

## Busca

| Contrato | Teste | Tipo |
|---|---|---|
| Maiúsculas/minúsculas e mapeamentos expandidos (İ) | `pdf::text_search::tests::finds_multiple_case_insensitive_hits`, `…::expanding_lowercase_mappings_still_match` | U |
| Frase atravessa quebra de linha, um retângulo por linha | `…::phrase_matches_across_line_break_with_one_rect_per_line` | U |
| Trecho com caixa original e reticências | `…::context_keeps_original_case_and_marks_the_match`, `…::long_context_is_truncated_with_ellipsis` | U |
| Geometria do hit fica dentro da página real | `…::real_pdf_hits_carry_geometry_inside_the_page` | M |
| Navegação centraliza a linha da ocorrência | `app_overlays::tests::hit_fraction_points_at_the_line_center` | U |

## Seleção e cópia

| Contrato | Teste | Tipo |
|---|---|---|
| Vão vira espaço, mudança de linha vira `\n`, sem `\r` | `text_selection::tests::gaps_become_spaces_and_lines_become_newlines`, `…::explicit_line_breaks_are_normalized` | U |
| Duplo clique = palavra (respeita vãos); triplo = linha | `…::word_range_*`, `…::line_range_covers_the_visual_line` | U |
| Seleção para trás e índices fora de faixa são seguros | `…::ordered_handles_backwards_drags`, `…::out_of_range_requests_are_safe` | U |

## Modo Flutuante

| Contrato | Teste | Tipo |
|---|---|---|
| Página enquadrada sem distorção | `app_frame::tests::fit_rect_letterboxes_wide_and_tall_containers` | U |
| Zoom mantém o ponto sob o cursor e nunca fica abaixo do enquadramento | `app_frame::tests::zoom_keeps_the_point_under_the_cursor`, `…::zoom_never_goes_below_fit` | U |
| Pan limitado à área ampliada | `…::pan_is_limited_to_the_zoomed_overflow` | U |
| Bordas e cantos redimensionam; a lateral preserva a largura | `…::resize_direction_detects_edges_and_corners`, `…::horizontal_edge_resize_keeps_the_new_width`, `…::vertical_edges_corners_and_first_snap_keep_height` | U |
| Barra some fora da janela, fica translúcida na página e opaca no hover | `app_frame::bar::tests::*` | U |

## Persistência e janela

| Contrato | Teste | Tipo |
|---|---|---|
| Escrita atômica; falha preserva o arquivo anterior e avisa | `storage::tests::atomic_write_replaces_and_leaves_no_temp_files`, `…::failed_write_keeps_previous_file_and_reports` | U |
| TOML corrompido vira backup + padrão + aviso | `storage::tests::corrupted_toml_is_quarantined_and_defaults_are_used` | U |
| Tema persistido; "Sistema" segue o Windows | `settings_types::tests::theme_*` | U |
| Limpar histórico grava estado vazio | `pdf::persistence::tests::clear_all_persists_an_empty_history` | U |
| Janela reabre no lugar; monitor removido é recuperado | `platform::window_placement::tests::*` | U |

## Internacionalização

| Contrato | Teste | Tipo |
|---|---|---|
| PT-BR e EN têm as mesmas chaves | `i18n::tests::locales_expose_the_same_keys` | U |
| Toda chave usada no código existe | `i18n::tests::every_key_used_in_code_exists` | U |
| Widgets, tooltips e toasts não recebem texto fixo | `i18n::tests::ui_text_is_not_hardcoded` | U |
| Rótulos dinâmicos resolvem nos dois idiomas | `settings_types::tests::dynamic_labels_resolve_in_both_languages` | U |

## Toolbar

| Contrato | Teste | Tipo |
|---|---|---|
| Rótulos centralizados pelo contorno real dos glifos | `app_toolbar::tests::ink_bounds_are_centered_with_equal_padding` | U |
| Grupos mais baixos que a linha, com padding igual | `app_toolbar::tests::group_is_shorter_than_the_row` | U |
