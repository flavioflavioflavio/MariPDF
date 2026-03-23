# Strings em inglês — bundled no binário via include_str!
# Fase 4: expandir com todas as chaves da UI

app-title = MariPDF
app-subtitle = Ultra-Lightweight PDF Reader for Windows

# Navegação
nav-page = Page { $current } of { $total }
nav-zoom = Zoom: { $percent }%

# Toasts e mensagens
toast-file-opened =
    { $pages ->
        [one] Document opened — 1 page
       *[other] Document opened — { $pages } pages
    }
toast-file-error = Failed to open file: { $reason }
toast-update-available = Update available: v{ $version }

# Erros
error-pdf-corrupted = This PDF appears to be corrupted.
error-pdf-password = This PDF is password-protected.
error-pdf-not-found = File not found.

# UI
action-close = Close
action-copy = Copy
action-open = Open PDF
