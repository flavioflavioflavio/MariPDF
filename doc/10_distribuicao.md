# Distribuição, assinatura e releases (BF-6)

## Identidade (BF-601)

| Campo | Valor |
|---|---|
| Produto | MariPDF |
| Publisher | Flávio Takemoto |
| Licença | GPL-3.0-or-later |
| Código e releases | https://github.com/flavioflavioflavio/MariPDF |
| Suporte | GitHub Issues do repositório |
| Contato do autor | https://www.linkedin.com/in/takemotoflavio/ |
| Dedicatória | Dedicado a Mariana Lima, com muito amor. |

Ícone multi-resolução em `assets/icon.ico`, gerado a partir de `assets/icon.png`
por `tools/make_icon.py`. Os metadados do `.exe` (publisher, copyright,
descrição) vêm de `build.rs` (BF-602).

## Artefatos (BF-603)

`pwsh tools/package.ps1` gera, em `dist/`:

| Arquivo | Para quem |
|---|---|
| `MariPDF-<v>-portable.exe` | Executável único, com PDFium embutido. Roda de qualquer pasta, sem instalar. |
| `MariPDF-<v>-setup.exe` | Instalador **por usuário**: sem admin, sem UAC, em `%LOCALAPPDATA%\Programs\MariPDF`. |
| `MariPDF-<v>-sbom.json` | SBOM CycloneDX das dependências (gerado no CI). |
| `SHA256SUMS.txt` + `.sig` | Hashes de todos os artefatos, com assinatura Ed25519. |

O instalador é por usuário de propósito: sem elevação, o Windows não mostra o
prompt de UAC. O desinstalador também não exige admin e remove tudo o que o
instalador criou. Configurações e histórico do usuário são preservados.

## Associação `.pdf` (BF-604)

O Windows 10/11 guarda o app padrão de cada extensão em `UserChoice`, protegido
por hash: nenhum programa pode se definir como padrão sozinho. O MariPDF
(app e instalador, com as mesmas chaves em `HKCU`):

1. registra ProgID, `OpenWithProgids`, `Applications\maripdf.exe`,
   `Capabilities` e `RegisteredApplications`;
2. abre `ms-settings:defaultapps?registeredAppUser=MariPDF` para o usuário
   confirmar com um clique;
3. mostra o estado real lido de `UserChoice`: padrão, registrado ou não
   registrado.

## Assinatura de código (BF-605)

**Estratégia inicial, sem certificado Authenticode:**

- **Unsigned:** EXE e instalador ainda não têm assinatura da Microsoft. Na
  primeira execução, o SmartScreen mostra "O Windows protegeu o computador" →
  **Mais informações → Executar assim mesmo**. O aviso diminui conforme o
  arquivo ganha reputação de download.
- **Verificação de integridade:** `SHA256SUMS.txt` com assinatura **Ed25519**.
  A chave pública está embutida no app (`src/updater.rs`), e a atualização
  automática só instala se a assinatura e o hash conferirem (BF-606). Isso
  protege contra troca de arquivos no servidor, mas não remove o aviso do
  SmartScreen.
- **Próximo passo recomendado:** assinatura Authenticode gratuita para open
  source via [SignPath Foundation](https://signpath.org), ou o Azure Trusted
  Signing (pago, mensal). Os dois se integram ao `release.yml` antes do passo
  de checksums.

### Chave de release

- **Privada:** `%USERPROFILE%\.maripdf\release-signing-key.pem`. **Nunca** vai
  para o repositório. Guarde um backup offline: sem ela, as próximas versões
  não podem ser verificadas pelos apps já instalados.
- **Pública:** `bb025300…890adf5e`, embutida em `RELEASE_PUBLIC_KEY`.
- **Rotação:** gerar um novo par, publicar uma versão assinada com a chave
  **antiga** que já embute a nova, e só então passar a assinar com a nova.

## Atualização (BF-606)

- **Manual:** só consulta a rede quando o usuário clica em *Sobre → Verificar
  atualizações*. Não há verificação em segundo plano.
- **Canais:** quem está numa versão estável só recebe estáveis; quem está numa
  pré-release (RC) recebe a mais nova, incluindo RCs. Rascunhos nunca aparecem.
- **Verificado:** o app baixa o instalador, `SHA256SUMS.txt` e `.sig`. Confere
  a assinatura e o hash antes de executar. Se algo falhar, descarta o arquivo
  e explica o motivo.
- **Rollback:** o Inno Setup desfaz a instalação se ela falhar no meio. A
  versão anterior continua funcional, e o downgrade é bloqueado pelo
  instalador.
- **Portátil:** recebe só o link da release; a troca do arquivo é manual.

## Release (BF-607)

1. Atualizar a `version` no `Cargo.toml` e o `CHANGELOG.md`, e fazer o merge
   no `master`.
2. `git tag v<versão> && git push origin v<versão>`.
3. O workflow **Release** roda os gates, o smoke test, o empacotamento, o SBOM
   e os checksums, e cria um **rascunho** de release.
4. Sem o secret `MARIPDF_SIGNING_KEY`, assine localmente:
   `pwsh tools/release_sign.ps1 -Tag v<versão>`.
5. Revisar as notas e **publicar** o rascunho manualmente.

## Testes de instalação (BF-608)

`pwsh tools/installer_test.ps1` cobre instalação limpa, upgrade, downgrade
bloqueado, desinstalação e `HKCU` restaurado ao estado anterior.

- **Windows 11 build 26200:** 13/13 ✅.
- **Windows 10:** ainda não executado. Pendente na matriz da RC (BF-707).

## Winget (BF-609)

O `tools/package.ps1` gera `dist/winget/manifests/...` para a versão (Inno, escopo
de usuário). Para publicar: `winget validate` e um PR manual em
`microsoft/winget-pkgs`, depois da primeira release estável.

## Doação (BF-610)

O link "Apoiar o MariPDF" no *Sobre* já existe, mas só aparece quando
`DONATION_URL` (`src/app_about.rs`) recebe um canal oficial. Não há modal nem
lembrete.
