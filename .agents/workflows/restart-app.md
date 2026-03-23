---
description: Matar processo e reabrir o MariPDF após qualquer alteração de código
---

# Restart App (OBRIGATÓRIO após toda alteração)

**ORDEM DO USUÁRIO: após TODA mudança de código, matar o processo antigo e relançar imediatamente.**

// turbo-all

1. Terminar o processo cargo run anterior (via send_command_input com Terminate: true no CommandId ativo)

2. Compilar e relançar:
```
cargo run 2>&1
```
(Cwd: e:\_DEV_2\MariPDF\MariPDF, WaitMsBeforeAsync: 3000)
