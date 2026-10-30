# tools/package_msix.ps1 — Pacote MSIX da Microsoft Store.
#
#   pwsh tools/package_msix.ps1                # pacote para enviar à Store
#   pwsh tools/package_msix.ps1 -Register      # teste local (Modo de Desenvolvedor)
#
# Identidade (padrões abaixo): Partner Center → MariPDF (9NBRBHQTB8SP) →
# Gerenciamento do produto → Identidade do produto.
# A Store re-assina o pacote; nenhum certificado próprio é necessário.
# Saída: dist/MariPDF-<versão>.msix (e dist/msix-stage com o layout).

param(
    [string]$IdentityName = "Takemoto.MariPDF",
    [string]$Publisher = "CN=CDCC5894-95FC-42E2-9B2C-C3A22F788BE9",
    [string]$PublisherDisplayName = "Takemoto",
    # Versão do pacote (4 números, último = 0). Padrão: derivada do Cargo.toml.
    [string]$Version = "",
    [string]$Python = "python",
    [switch]$SkipBuild,
    # Registra o layout desempacotado para testar localmente (Modo de Desenvolvedor).
    [switch]$Register
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$cargoVersion = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
if (-not $Version) {
    # X.Y.Z-rc.N → X.Y.(Z*100+N).0 ; X.Y.Z → X.Y.(Z*100+99).0 (RCs < estável, sempre crescente)
    if ($cargoVersion -notmatch '^(\d+)\.(\d+)\.(\d+)(?:-rc\.(\d+))?$') { throw "Versão não suportada: $cargoVersion" }
    $patch = [int]$Matches[3] * 100 + $(if ($Matches[4]) { [int]$Matches[4] } else { 99 })
    $Version = "$($Matches[1]).$($Matches[2]).$patch.0"
}
Write-Host "MariPDF $cargoVersion → pacote $Version ($IdentityName)"

$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.*\x64\makeappx.exe" |
    Sort-Object FullName | Select-Object -Last 1
if (-not $sdk) { throw "makeappx.exe não encontrado (instale o Windows SDK)" }

if (-not $SkipBuild) {
    # Diretório próprio para não misturar com o build normal (sem a feature store).
    cargo build --release --locked --features store --target-dir target/store
    if ($LASTEXITCODE -ne 0) { throw "cargo build --features store falhou" }
}
$exe = "target\store\release\maripdf.exe"
if (-not (Test-Path $exe)) { throw "Executável não encontrado: $exe" }

$stage = "dist\msix-stage"
if (Test-Path $stage) {
    if ($Register) { Get-AppxPackage -Name $IdentityName | Remove-AppxPackage -ErrorAction SilentlyContinue }
    Remove-Item -Recurse -Force $stage
}
New-Item -ItemType Directory -Force "$stage\Assets" | Out-Null
Copy-Item $exe "$stage\MariPDF.exe"
Copy-Item LICENSE "$stage\LICENSE.txt"

& $Python tools/make_msix_assets.py "$stage\Assets"
if ($LASTEXITCODE -ne 0) { throw "geração dos ícones falhou" }

$manifest = Get-Content installer\msix\AppxManifest.xml.in -Raw -Encoding utf8
$manifest = $manifest.Replace("{{IDENTITY_NAME}}", $IdentityName).
    Replace("{{PUBLISHER}}", [Security.SecurityElement]::Escape($Publisher)).
    Replace("{{PUBLISHER_DISPLAY_NAME}}", [Security.SecurityElement]::Escape($PublisherDisplayName)).
    Replace("{{VERSION}}", $Version)
[IO.File]::WriteAllText((Resolve-Path $stage).Path + "\AppxManifest.xml", $manifest, [Text.UTF8Encoding]::new($false))

# Índice de recursos (PRI): faz o Windows escolher o ícone certo para cada
# escala e tamanho (.scale-200, .targetsize-48 etc.).
$makepri = Join-Path $sdk.DirectoryName "makepri.exe"
$priConfig = Join-Path $env:TEMP "maripdf-priconfig.xml"
& $makepri createconfig /cf $priConfig /dq pt-BR /pv 10.0.0 /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri createconfig falhou" }
# Sem <packaging>: tudo num resources.pri só (o MSIX não é dividido em pacotes de recursos).
$config = Get-Content $priConfig -Raw
[IO.File]::WriteAllText($priConfig, ($config -replace '(?s)\s*<packaging>.*?</packaging>', ''))
& $makepri new /pr $stage /cf $priConfig /mn "$stage\AppxManifest.xml" /of "$stage\resources.pri" /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri new falhou" }

if ($Register) {
    Add-AppxPackage -Register "$stage\AppxManifest.xml"
    Write-Host "Registrado: procure 'MariPDF' no menu Iniciar."
    return
}

$msix = "dist\MariPDF-$cargoVersion.msix"
& $sdk.FullName pack /o /d $stage /p $msix | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makeappx pack falhou" }
Write-Host ("{0} ({1:N1} MB)" -f $msix, ((Get-Item $msix).Length / 1MB))
