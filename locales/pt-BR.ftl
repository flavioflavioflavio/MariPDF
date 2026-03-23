# Strings em português brasileiro — bundled no binário via include_str!
# Fase 4: expandir com todas as chaves da UI

app-title = MariPDF
app-subtitle = Leitor de PDF Ultra-Leve para Windows

# Navegação
nav-page = Página { $current } de { $total }
nav-zoom = Zoom: { $percent }%

# Toasts e mensagens
toast-file-opened =
    { $pages ->
        [one] Documento aberto — 1 página
       *[other] Documento aberto — { $pages } páginas
    }
toast-file-error = Falha ao abrir arquivo: { $reason }
toast-update-available = Atualização disponível: v{ $version }

# Erros
error-pdf-corrupted = Este PDF parece estar corrompido.
error-pdf-password = Este PDF está protegido por senha.
error-pdf-not-found = Arquivo não encontrado.

# UI
action-close = Fechar
action-copy = Copiar
action-open = Abrir PDF
