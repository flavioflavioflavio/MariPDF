# ADR-003 — UI Framework

> **Status:** Aceita
> **Data:** 2026-03-22
> **Autores:** Flavio

---

## Contexto

O MariPDF precisa de um UI framework que permita renderizar PDFs em GPU (para suavidade dos 60fps), implementar animações premium (toolbar, toasts, hover) e manter o binário pequeno. O framework define diretamente a experiência do desenvolvedor e os limites do que é possível na UI.

---

## Opções Consideradas

### Opção A — egui (immediate mode) ✅
- ✅ 100% Rust nativo
- ✅ Renderização via wgpu (GPU) — direto no pipeline desejado
- ✅ Immediate mode: UI é código, sem estado separado de widgets
- ✅ Muito rápido para UIs dinâmicas (todo frame é recomputado)
- ✅ Binário muito pequeno (~1–2MB para egui + wgpu backend)
- ✅ Ecossistema `eframe` para janela + loop de app pronto
- ⚠️ Acessibilidade limitada (MSAA/UIA parcial no Windows)
- ⚠️ Immediate mode tem curva de aprendizado para devs vindos de React/Flutter
- ❌ Widgets prontos são funcionais mas precisam de estilo customizado

### Opção B — iced
- ✅ 100% Rust nativo
- ✅ Arquitetura The Elm Architecture (mensagens, update, view) — familiar para devs funcionais
- ✅ Boa acessibilidade
- ❌ Renderização via `wgpu` ou `tiny-skia` — boa, mas sem o controle granular do egui
- ❌ Menor — ecossistema menor, menos exemplos
- ❌ Maturidade inferior ao egui (ainda em flux de API)

### Opção C — Tauri (WebView)
- ✅ UI em HTML/CSS/JS — liberdade máxima de design
- ✅ Usado em produção por muitos apps (1Password, Zed parcialmente)
- ❌ Depende do WebView2 do Windows (~100MB instalado separado)
- ❌ IPC entre Rust e JS adiciona latência para operações de scroll/render
- ❌ Renderização do PDF via wgpu ficaria fora do contexto da WebView — integração complexa
- ❌ Contradiz a meta de binário ~8MB portátil

### Opção D — Slint
- ✅ DSL declarativa para UI — design muito legível
- ✅ Suporte Windows nativo, renderização GPU
- ✅ Bom suporte a acessibilidade
- ❌ Licença GPL para open source mas com restrições em uso comercial — potencial conflito
- ❌ Menor comunidade, menos bindings externos maduros
- ❌ Rendering de PDF via Slint é não-trivial (sem suporte a textura customizada direto)

---

## Decisão

**Escolhemos: Opção A — egui**

A única opção que combina: 100% Rust, GPU-nativo via wgpu, controle total do pipeline de renderização (necessário para desenhar páginas PDF como texturas), binário pequeno e produtividade adequada. A limitação de acessibilidade é conhecida e documentada.

---

## Consequências

### Acessibilidade — Posição Oficial do Projeto

`egui` tem suporte **parcial** a accessibility APIs no Windows (MSAA/UIA). Isso significa que leitores de tela como o NVDA podem ter experiência degradada.

**Posição do projeto:**
- MariPDF v1.0 não tem garantias de acessibilidade total
- Isso será explicitamente documentado no README
- Contribuições que melhorem acessibilidade são bem-vindas
- A equipe monitorará o progresso do egui em `AccessKit` (integração em desenvolvimento)

### Modelo de Renderização PDF em egui

As páginas PDF serão renderizadas pelo MuPDF como bitmaps e carregadas como **texturas wgpu** dentro do painter do egui. O fluxo:

```
MuPDF renderiza página → bitmap RGBA
     ↓
upload para GPU como wgpu::Texture
     ↓
egui recebe TextureId → desenha com Image widget
     ↓
wgpu composita na tela
```

Isso dá controle total sobre a renderização sem sair do pipeline egui/wgpu.

### Customização de Estilo

egui usa um sistema de `Style` e `Visuals` para customização. Todos os tokens de cor do Design System (`03_design_system.md`) serão mapeados para `egui::Visuals` no `app.rs`:

```rust
let mut visuals = egui::Visuals::dark();
visuals.panel_fill = Color32::from_rgb(0x2A, 0x2A, 0x2A); // --bg-surface dark
// ...
ctx.set_visuals(visuals);
```

---

## Referências

- [egui — repositório oficial](https://github.com/emilk/egui)
- [AccessKit — acessibilidade para egui](https://github.com/AccessKit/accesskit)
- [egui — custom textures (PDF rendering pattern)](https://github.com/emilk/egui/blob/master/crates/egui/src/painter.rs)
- [iced](https://github.com/iced-rs/iced)
- [Slint](https://slint.dev)
