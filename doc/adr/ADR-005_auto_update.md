# ADR-005 — Auto-Update

> **Status:** Proposta aceita; implementação pendente no roadmap BF-6
> **Data:** 2026-03-22
> **Autores:** Flavio

---

## Contexto

O MariPDF precisa de um mecanismo de atualização automática que seja: não-intrusivo (não bloqueia a UI), seguro (valida integridade do binário), e funcional no Windows (que tem restrições sobre substituir executáveis em uso).

---

## Opções Consideradas

### Opção A — `self_update` crate ✅
- ✅ Integração nativa com GitHub Releases
- ✅ Validação SHA-256 do binário baixado
- ✅ 100% Rust, sem dependências externas
- ✅ Roda via `tokio::spawn` — nunca bloqueia a UI
- ⚠️ No Windows, não pode substituir o `.exe` em execução — requer estratégia de "staged update"
- ❌ Menos sofisticado que o Sparkle (macOS) ou WinSparkle

### Opção B — WinSparkle
- ✅ Solução madura específica para Windows
- ✅ Lida com o problema de "exe em uso" automaticamente
- ❌ É uma DLL C++ — FFI necessária, rompe o modelo de binário puro Rust
- ❌ Adiciona dependência de runtime nativo

### Opção C — Implementação própria (HTTP + GitHub API)
- ✅ Controle total
- ❌ Re-inventar a roda — surface de bugs desnecessária
- ❌ Validação de assinatura (GPG/SHA) é complexa de implementar certo

---

## Decisão

**Escolhemos: Opção A — `self_update`**

Simples, seguro, 100% Rust e integra nativamente com GitHub Releases que é onde as distribuições serão publicadas. O problema do Windows (exe em uso) é resolvido com a estratégia de staged update descrita abaixo.

---

## Estratégia de Staged Update no Windows

O Windows não permite substituir um `.exe` que está em execução. A solução:

```
1. Download do novo binário → salvo como maripdf_update.exe no mesmo diretório
2. Validação SHA-256 obrigatória
3. UI mostra pill sutil: "Nova versão disponível — reiniciar para aplicar"
4. Usuário clica em "Reiniciar"
5. App cria um script .bat temporário:
   - Aguarda o processo atual fechar (loop até PID sumir)
   - Renomeia maripdf.exe → maripdf_old.exe
   - Renomeia maripdf_update.exe → maripdf.exe
   - Deleta maripdf_old.exe
   - Reinicia maripdf.exe
6. App encerra (limpando estado)
```

Esse padrão é usado pelo Firefox, VSCode e outros apps no Windows.

---

## UX do Update

### O que fazemos
- Check silencioso no startup (tokio::spawn, delay de 10s após abrir)
- Pill sutil no rodapé da toolbar: `"v1.1.0 disponível ↑"` — clique para detalhes
- Dialog simples: notas de release + botão "Reiniciar e atualizar"
- Botão "Lembrar depois" — verifica novamente em 7 dias

### O que NUNCA fazemos (alinhado ao manifesto)
- ❌ Popup bloqueante no startup
- ❌ Download automático sem aviso
- ❌ Forçar update
- ❌ Interromper uma sessão de leitura ativa

---

## Consequências

- O **CI/CD** (`release.yml`) deve gerar SHA-256 para cada artefato e publicar no GitHub Release como arquivo `.sha256`
- A versão do app em `Cargo.toml` é a **fonte da verdade** — `self_update` usa `cargo_crate_version!()`
- O `.exe` portátil e o `.msi` terão processos de update **distintos**: portátil usa o mecanismo acima; `.msi` pode usar Windows Installer para atualizar

---

## Referências

- [self_update — crate](https://github.com/jaemk/self_update)
- [GitHub Releases API](https://docs.github.com/en/rest/releases)
- [Staged update pattern (Firefox source)](https://firefox-source-docs.mozilla.org/toolkit/mozapps/update/)
