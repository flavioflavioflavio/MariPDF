# ❤️ MariPDF — Donationware & Open Source

> Criado em: 2026-03-22

---

## Filosofia

Software gratuito no espírito clássico: **WinRAR, SumatraPDF, Notepad++**.

> "Gratuito para sempre. Nenhuma feature bloqueada. Doação 100% voluntária."

---

## Licença

**GNU GPL v3.0**

- Open source garantido — ninguém pode fechar um fork
- Forks devem obrigatoriamente manter open source
- Compatível com MuPDF AGPL (mesma família de licenças)
- Permite uso comercial e pessoal gratuitamente

---

## Canais de Doação

| Canal | Público | Como integrar |
|---|---|---|
| **PIX** | Brasileiros | QR Code gerado no app + chave texto copiable |
| **PayPal** | Internacional | `paypal.me/seulink` |
| **Stripe** | Internacional | `buy.stripe.com/seulink` |
| **GitHub Sponsors** | Global | Melhor para OSS recórrente |

---

## UX da Doação — Regras de Ouro

1. **Nunca aparece no startup** — jamais
2. **Aparece 1x automaticamente** após 30 dias de uso
3. **Aparece no menu** "Ajuda → Apoiar o projeto" — sempre acessível
4. **Após clicar em qualquer link de doação** → nunca mais aparece automaticamente
5. **Sem countdown, sem bloqueio, sem pressão**

---

## Dialog de Doação

```
╭──────────────────────────────────────────────╮
│                                              │
│   ❤️  Gostou do MariPDF?                    │
│                                              │
│   É gratuito e sempre será.                 │
│   Uma doação mantém o projeto vivo.         │
│                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │   PIX    │  │  PayPal  │  │  Stripe  │  │
│  │  [QR]   │  │  →link   │  │  →link   │  │
│  └──────────┘  └──────────┘  └──────────┘  │
│                                              │
│             [Talvez depois]                  │
╰──────────────────────────────────────────────╯
```

---

## 📱 Especificação Técnica — QR Code PIX

### Implementação

O QR Code PIX é gerado **no app, em tempo de compilação** (não em runtime via request HTTP):

```rust
// Crate: qrcode (pure Rust, sem dependência externa)
// Cargo.toml: qrcode = "0.14"

use qrcode::QrCode;
use qrcode::render::unicode;

const PIX_PAYLOAD: &str = "00020126..."; // Payload PIX estático (EMV Co)

fn render_pix_qrcode() -> egui::ColorImage {
    let code = QrCode::new(PIX_PAYLOAD).expect("PIX payload é always valid");
    // Render como bitmap RGBA e carrega como texture egui
    qrcode_to_egui_texture(&code)
}
```

### Formato do Payload PIX

O payload PIX segue o padrão **EMV Co** (código estático, sem valor):

```
00020126...   (chave PIX email ou CPF ou chave aleatória)
```

Gerado uma vez em [gerador de payload PIX](https://www.geradordepix.com.br/) e hardcoded na constante. O QR é regenerado via `qrcode` crate em tempo de execução a partir dessa string.

### Na UI

- QR Code renderizado como imagem 160x160px no dialog de doação
- Abaixo do QR: chave PIX como texto selecionável (botão `[Copiar]` ao lado)
- Texto: *"Escaneie com qualquer app de banco"*

### Dependência a adicionar

```toml
# Cargo.toml
[dependencies]
qrcode = "0.14"   # Pure Rust, sem código nativo externo
```

---

## GitHub Setup

### FUNDING.yml
```yaml
# .github/FUNDING.yml
paypal: seu-usuario
custom:
  - https://buy.stripe.com/seu-link
  - https://pix.link/sua-chave
```
→ Adiciona botão **"Sponsor"** nativo no repositório GitHub.

### README Badges
```markdown
## ❤️ Apoie o projeto

[![PayPal](https://img.shields.io/badge/PayPal-Doe-blue)](link)
**PIX:** `sua@chave.pix`
[![Stripe](https://img.shields.io/badge/Stripe-Doe-purple)](link)
```

---

## About Dialog (In-App)

```
MariPDF v1.0.0
Feito com ❤️ por [Seu Nome]

Software gratuito. Código aberto. GPLv3.
Não coleta dados. Não tem telemetria.
Apenas lê PDFs, muito bem.

[GitHub]  [Apoiar o projeto]
```

---

## Sustentabilidade a Longo Prazo

- **GitHub Sponsors** — recorrente, melhor para OSS
- **Hall of Fame** no README — quem doa entra (opt-in)
- **Transparência anual** — post de receita vs custo do projeto
- **Ko-fi** como alternativa ao Sponsors (sem taxa)
- **Nunca freemium** — sem features pagas, nunca

---

## O que NUNCA fazer

- ❌ "Versão Pro" com features bloqueadas
- ❌ Trial de 30 dias
- ❌ Nag screen agressiva
- ❌ Popup de doação no startup
- ❌ Telemetria "para melhorar o produto"
- ❌ Vender dados de qualquer tipo
