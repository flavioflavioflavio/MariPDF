# 🚨 MariPDF — Tratamento de Erros

> Criado em: 2026-03-22

---

## Filosofia

> *"Erros são eventos, não surpresas."*

- Todo erro tem uma causa, uma mensagem para o usuário e uma ação sugerida
- Nunca mostrar stack traces ou mensagens técnicas para o usuário final
- Sempre deixar o usuário continuar usando o app após um erro (graceful degradation)
- Log técnico detalhado vai para arquivo — UI recebe apenas o essencial

---

## Hierarquia de Erros (`src/error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum MariError {
    #[error(transparent)]
    Pdf(#[from] PdfError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    I18n(#[from] I18nError),

    #[error(transparent)]
    Update(#[from] UpdateError),
}

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("Arquivo não encontrado")]
    FileNotFound(PathBuf),

    #[error("PDF protegido por senha")]
    PasswordProtected,

    #[error("PDF corrompido ou formato inválido")]
    InvalidFormat(String),

    #[error("Sem memória para renderizar página {0}")]
    OutOfMemory(usize),

    #[error("Erro interno do motor PDF")]
    EngineError(String),
}
```

---

## Catálogo de Erros e Respostas de UI

### 🔴 Erros Críticos (impedem leitura)

| Erro | Causa | UI para o Usuário | Ação |
|---|---|---|---|
| `FileNotFound` | Arquivo movido/deletado após abrir | Toast: *"Arquivo não encontrado. Pode ter sido movido."* | Remove da lista de recentes |
| `PasswordProtected` | PDF com senha | Dialog: *"Este PDF está protegido por senha."* + campo de senha | Retentar com senha ou cancelar |
| `InvalidFormat` | PDF corrompido ou não é PDF | Toast erro: *"Não foi possível abrir o arquivo. O PDF pode estar corrompido."* | Volta para tela inicial |
| `OutOfMemory` | PDF de altíssima resolução + RAM baixa | Toast: *"Memória insuficiente para renderizar esta página."* | Oferece reduzir zoom |

### 🟡 Erros Degradados (app continua funcionando)

| Erro | Causa | UI para o Usuário | Ação |
|---|---|---|---|
| Falha ao salvar posição | Disco cheio / permissão | Silencioso (log apenas) | Continua sem salvar |
| Falha ao baixar locale | Sem internet | Toast: *"Não foi possível baixar o idioma. Usando inglês."* | Fallback para `en` |
| Falha no auto-update | Sem internet / GitHub offline | Silencioso (log apenas) | Tenta novamente em 24h |
| Thumbnail falhou | Página com conteúdo complexo | Exibe ícone genérico de PDF na tela inicial | Sem bloqueio |

### 🟢 Erros Silenciosos (log apenas)

| Erro | Comportamento |
|---|---|
| Falha ao salvar config.toml | Loga, continua com configuração em memória |
| locale on-demand indisponível | Usa `en` como fallback |
| Arquivo de history corrompido | Reseta history para vazio, loga erro |

---

## UX dos Erros

### Toast de Erro
```
╭──────────────────────────────────────╮
│ ⚠  PDF corrompido ou inválido        │
╰──────────────────────────────────────╯
```
- Shape: Pill — mesmo padrão do design system
- Cor de fundo: `--error` com 90% opacidade
- Auto-dismiss: 4s (mais tempo que toasts normais — 2.5s)
- Posição: bottom-center

### Dialog de Senha (PDF protegido)
```
╭────────────────────────────────────╮
│  🔒  PDF protegido por senha       │
│                                    │
│  ┌──────────────────────────────┐  │
│  │ Digite a senha...            │  │
│  └──────────────────────────────┘  │
│                                    │
│  [Cancelar]          [Abrir]       │
╰────────────────────────────────────╯
```
- Campo de senha com eye icon (mostrar/ocultar)
- Máximo 3 tentativas antes de mostrar: *"Senha incorreta. Tente novamente."*
- Após 3 falhas: fecha o dialog e avisa que o arquivo não pôde ser aberto

---

## Logging Técnico

O app mantém log de erros técnicos em:
```
%APPDATA%\maripdf\maripdf.log
```

**Formato:**
```
2026-03-22T14:05:33Z [ERROR] pdf::engine: MuPDF error rendering page 42: out of memory
2026-03-22T14:05:34Z [WARN]  storage::config: Failed to write config.toml: permission denied
```

**Rotação:** Máximo 5MB. Quando atinge o limite, renomeia para `maripdf.log.old` e começa novo.

**Crate:** `tracing` + `tracing-subscriber` com `tracing_appender` para file sink.

---

## Estratégia Anti-Panic

Em código de produção:
- ❌ Proibido `unwrap()` sem comentário justificando
- ❌ Proibido `expect()` sem comentário
- ✅ Usar `?` para propagar erros
- ✅ Usar `unwrap_or_else(|e| { log::error!(...); default_value })` quando faz sentido continuar

O handler de panic global:
```rust
// main.rs
std::panic::set_hook(Box::new(|info| {
    log::error!("PANIC: {:?}", info);
    // Flush logs antes de morrer
}));
```

---

## Referências

- [thiserror crate](https://crates.io/crates/thiserror)
- [tracing crate](https://crates.io/crates/tracing)
- [anyhow vs thiserror — quando usar](https://nick.groenen.me/posts/rust-error-handling/)
