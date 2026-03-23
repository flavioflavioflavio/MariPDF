# 🎨 MariPDF — Design System

> Criado em: 2026-03-22

---

## Filosofia: "Invisible Interface"
> A UI some. O conteúdo fica.

Leve, instantâneo, sem frescura. Premium na estética, cirúrgico na performance.

---

## 🪙 Color Tokens

### Base
| Token | Light | Dark | Uso |
|---|---|---|---|
| `--bg-canvas` | `#F8F6F1` | `#1A1A1A` | Fundo da área de leitura |
| `--bg-surface` | `#FFFFFF` | `#2A2A2A` | Páginas do PDF |
| `--bg-panel` | `#F0EDE8` | `#242424` | Toolbar, painéis |
| `--bg-overlay` | `rgba(0,0,0,0.04)` | `rgba(255,255,255,0.04)` | Hover states |

### Texto
| Token | Light | Dark | Uso |
|---|---|---|---|
| `--text-primary` | `#1C1C1E` | `#F2F2F7` | Texto principal |
| `--text-secondary` | `#6E6E73` | `#8E8E93` | Labels, subtítulos |
| `--text-disabled` | `#AEAEB2` | `#48484A` | Inativo |

### Accent
| Token | Valor | Uso |
|---|---|---|
| `--accent` | `#5B4FE8` | CTAs, seleção, progresso |
| `--accent-soft` | `#EAE8FD` | Highlight de busca, badges |
| `--accent-hover` | `#4A3FD4` | Hover sobre accent |

### Feedback
| Token | Valor | Uso |
|---|---|---|
| `--success` | `#34C759` | Toast de sucesso |
| `--warning` | `#FF9F0A` | Avisos |
| `--error` | `#FF3B30` | Erros |

### Superfície & Sombra
| Token | Valor | Uso |
|---|---|---|
| `--border-subtle` | `rgba(0,0,0,0.08)` | Divisores suaves |
| `--shadow-page` | `0 4px 24px rgba(0,0,0,0.12)` | Sombra das páginas PDF |
| `--shadow-panel` | `0 2px 8px rgba(0,0,0,0.08)` | Painéis flutuantes |

### Modos de Leitura
| Modo | `--bg-canvas` | `--bg-surface` |
|---|---|---|
| Sepia | `#F4ECD8` | `#FAF3E0` |
| Dark | `#1A1A1A` | `#2A2A2A` |

---

## 🔵 Shape Language

| Padrão | `border-radius` | Uso |
|---|---|---|
| **Round** | `50%` | Botões de ação isolados (FAB-style) |
| **Pill** | `999px` | Tags, badges, busca, toasts, toolbar |
| **Rounded Square** | `12px` | Cards, painéis, dropdowns, thumbnails |
| **Subtle** | `6px` | Inputs, menus pequenos, tooltips |

### Exemplos por contexto
- Botão de fechar busca → **Round** `32×32`
- Badge "3 de 17" (resultados) → **Pill**
- Card de arquivo recente → **Rounded Square** `12px`
- Toolbar flutuante → **Pill** longa + `backdrop-blur`
- Dropdown de zoom → **Rounded Square** `8px`

---

## 🔷 Icon System — Phosphor Icons

**Fonte:** [Phosphor Icons](https://icon-sets.iconify.design/ph) — variante `ph` (regular/line)

### Regras
- **Default:** `ph` (line) — clean, elegante
- **Ativo/Selecionado:** `ph-fill`
- **Stroke:** `1.5px` padrão, `2px` para ênfase
- **Tamanhos:** `16px` (inline) · `20px` (toolbar) · `24px` (ações principais)
- **Cor:** sempre `currentColor`

### Mapeamento Funcional
| Ação | Ícone |
|---|---|
| Menu | `ph-list` |
| Busca | `ph-magnifying-glass` |
| Zoom +/- | `ph-magnifying-glass-plus/minus` |
| Pág. anterior/próxima | `ph-caret-left/right` |
| Tela cheia | `ph-arrows-out` / `ph-arrows-in` |
| Fechar | `ph-x` |
| Dark mode | `ph-moon` / `ph-sun` |
| Sepia | `ph-book-open` |
| Miniaturas | `ph-squares-four` |
| Scroll contínuo | `ph-rows` |
| Configurações | `ph-gear` |
| Seleção de texto | `ph-text-cursor` |
| Download | `ph-download-simple` |

---

## ✍️ Typography

| Token | Fonte | Peso | Tamanho | Uso |
|---|---|---|---|---|
| `--font-ui` | Inter / Geist | 400–600 | 13–15px | Toda a UI |
| `--font-label` | Inter | 500 | 12px | Labels, badges |
| `--font-mono` | Geist Mono | 400 | 12px | Número de página, zoom % |

> Fontes embutidas no binário (~250KB) — sem dependência de sistema.

---

## ⚡ Motion & Animation

| Contexto | Duração | Easing |
|---|---|---|
| Toolbar aparece/some | `180ms` | `ease-out` |
| Toast entra | `200ms` | `spring` |
| Toast some | `150ms` | `ease-in` |
| Hover em botão | `80ms` | `ease` |
| Zoom transition | `120ms` | `ease-out` |

**Princípio:** rápido o suficiente para não irritar, suave o suficiente para parecer premium.

---

## 🗂️ Layout da Tela de Leitura

```
┌──────────────────────────────────────────────────┐
│  [≡]  arquivo.pdf                   [-][□][✕]    │  ← Titlebar minimalista
├──────────────────────────────────────────────────┤
│                                                  │
│               ╔══════════╗                       │
│               ║          ║                       │
│               ║   PDF    ║  ← Centralizado, sombra elegante
│               ║          ║                       │
│               ╚══════════╝                       │
│                                                  │
├──────────────────────────────────────────────────┤
│  1 / 42  [────●────────]  75%  🔍  [⊞][≡]       │  ← Toolbar pill flutuante
```

### Comportamento da Toolbar
- **Hover no topo** → desce com fade-in
- **Lendo** → some automaticamente após 1.5s
- **Hover no rodapé** → mini-barra com página e zoom

---

## 🧩 Componentes Core

### Toolbar Flutuante
- Shape: **Pill** longa
- BG: `--bg-panel` + `backdrop-blur(12px)` + `--shadow-panel`

### Toast
- Shape: **Pill**
- Posição: bottom-center
- Auto-dismiss: 2.5s
- Sem ícone — texto é suficiente

### Botão Round (FAB)
- `50%`, `40×40px`
- BG: `--bg-panel`, hover: `--bg-overlay`
- Ícone: `20px` Phosphor line

### Badge / Pill
- Padding: `2px 8px`
- Font: `--font-label`
- Cor: `--accent-soft` + texto `--accent`

### Card Arquivo Recente
- Shape: **Rounded Square** `12px`
- Hover: `scale(1.02)` + sombra intensificada
- Thumbnail: renderizado em background thread

---

## 🚫 Anti-patterns (Proibido)
- ❌ Bordas quadradas (0px radius)
- ❌ Ícones filled no estado padrão
- ❌ Mais de 2 pesos de fonte na mesma tela
- ❌ Sombras com `black` sólido
- ❌ Animações > 300ms em interações repetidas
- ❌ Tooltips sem delay mínimo de 400ms

---

## 📱 Layout da Tela Inicial (Recent Files)

```
┌──────────────────────────────────────────────────┐
│  [≡]  MariPDF                          [-][□][✕]    │  ← Titlebar
├──────────────────────────────────────────────────┤
│                                                  │
│   📄  MariPDF                                       │  ← Logo + nome (topo centralizado)
│                                                  │
│   Recentes                         [Abrir PDF...]│  ← Seção com botão CTA
│                                                  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │ [thumb]  │ │ [thumb]  │ │ [thumb]  │ │ [thumb]  │  │  ← Grid de cards
│  │          │ │          │ │          │ │          │  │     (12px radius)
│  │ arquivo..│ │ relatório.│ │ manual..  │ │ livro..   │  │
│  │ há 2 dias │ │ ontem    │ │ há 5 dias │ │ há 7 dias │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
│                                                  │
│     Arraste um PDF para abrir                    │  ← Hint quando não há recentes
└──────────────────────────────────────────────────┘
```

### Comportamento dos Cards de Recentes
- Hover: `scale(1.02)` + sombra intensificada (80ms ease)
- Thumbnail: renderizado em background thread, lazy (placeholder enquanto carrega)
- Nome truncado com `...` no final se muito longo (máximo 2 linhas)
- Data: relativa (`há 2 dias`, `ontem`, `há 1h`) — não timestamp absoluto
- Clique direito: context menu com "Abrir", "Remover da lista", "Mostrar no Explorer"

---

## ⏳ Estados de UI

### Estado de Loading (renderizando página)
```
┌──────────────────────────────┐
│                              │
│    ░░░░░░░░░░░░░░░░       │  ← Skeleton shimmer (--bg-overlay)
│    ░░░░░░░░░░             │     na forma da página
│    ░░░░░░░░░░░░░░░     │
│                              │
└──────────────────────────────┘
```
- Sem spinner girando — skeleton com animação shimmer (gradiente que passa da esquerda para direita)
- Duração máxima visível: < 200ms (P95) — alvo de performance, não loader de propagação
- Forma do skeleton = proporção estimada da página (A4 vertical por padrão)

### Estado de Erro
- Erros críticos (PDF inválido) → Toast pill vermelho, bottom-center, 4s
- Erros de senha → Dialog modal (ver [07_tratamento_de_erros.md](07_tratamento_de_erros.md))
- **Nunca mostrar** mensagem técnica ao usuário — mensagem via i18n
Cores:
```
--error-toast-bg: rgba(255, 59, 48, 0.92)   # --error + 92% opacidade
--error-toast-text: #FFFFFF
```

### Estado Empty (sem recentes)
```
│                                    │
│         📄                           │  ← Ícone grande (64px), --text-disabled
│   Arraste um PDF aqui               │  ← --text-secondary
│   ou [Abrir arquivo...]             │  ← botão accent
│                                    │
```

---

## 📝 Seleção de Texto

| Token | Valor | Uso |
|---|---|---|
| `--selection-bg` | `rgba(91, 79, 232, 0.25)` | Fundo da seleção (accent a 25%) |
| `--selection-border` | `rgba(91, 79, 232, 0.5)` | Borda da seleção (opcional) |

- A seleção é renderizada **sobre** a textura da página PDF como overlay colorido
- Cursor muda para `text` quando hover em área de texto selecionável
- Ctrl+C copia o texto selecionado (via MuPDF text extraction)

---

## 🌄 Modo Sepia — Especificação

O modo Sepia afeta **apenas o canvas ao redor do PDF** — não modifica o PDF renderizado:

```
Canvas: --bg-canvas = #F4ECD8     (tom sepia)
Superfície (página PDF): #FAF3E0  (branco levemente amarelado)
PDF renderizado: sem alteração   (MuPDF renderiza cores originais)
```

Isso é intencional: modificar o conteúdo do PDF mudaria a fidelidade visual do documento. O sepia é um aconchego ambiente, não um filtro de conteúdo.

> **Futuro (pós v1.0):** Avaliar overlay CSS-style de tint sepia sobre a textura GPU se demanda for alta.
