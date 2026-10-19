# tools/installer_test.ps1 — Matriz de instalação do MariPDF (BF-608).
#
# Instala, atualiza, tenta downgrade (inclusive entre RCs) e desinstala o
# instalador por usuário, conferindo arquivos, atalhos, registro e a entrada
# de desinstalação. Também cobre: associação recusada no setup e registrada
# depois pelo app, e pasta escolhida com arquivos alheios. Ao final confere
# que o HKCU voltou ao estado anterior.
#
#   pwsh tools/installer_test.ps1

param(
    [string]$Iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    [string]$Exe = "target\release\maripdf.exe"
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$work = Join-Path $env:TEMP "maripdf-installer-test-$([guid]::NewGuid())"
New-Item -ItemType Directory $work | Out-Null

$appId = "73F72B7B-8E4F-4704-8CBA-9FB568284922"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{$appId}_is1"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\MariPDF"
$startMenuLink = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\MariPDF.lnk"
$progIdKey = "HKCU:\Software\Classes\MariPDF.Document"
$keys = @(
    $progIdKey,
    "HKCU:\Software\Classes\Applications\maripdf.exe",
    "HKCU:\Software\MariPDF",
    $uninstallKey
)
$results = [System.Collections.Generic.List[string]]::new()
$failed = $false

function Check([string]$name, [bool]$ok) {
    $results.Add(("{0} {1}" -f $(if ($ok) { "✅" } else { "❌" }), $name))
    if (-not $ok) { $script:failed = $true }
}
function Snapshot {
    $state = foreach ($k in $keys) { "{0}={1}" -f $k, (Test-Path $k) }
    $owp = (Get-ItemProperty "HKCU:\Software\Classes\.pdf\OpenWithProgids" -ErrorAction SilentlyContinue)."MariPDF.Document"
    $reg = (Get-ItemProperty "HKCU:\Software\RegisteredApplications" -ErrorAction SilentlyContinue).MariPDF
    ($state + "OpenWithProgids=$($null -ne $owp)" + "RegisteredApplications=$($null -ne $reg)") -join "; "
}
function Build([string]$version) {
    & $Iscc /Q "/DAppVersion=$version" "/DSourceExe=..\$Exe" "/DOutputDir=$work" installer\maripdf.iss
    if ($LASTEXITCODE -ne 0) { throw "ISCC $version falhou" }
    Join-Path $work "MariPDF-$version-setup.exe"
}
function Run([string]$setup, [string]$tasks = "associate", [string]$extra = "") {
    $arguments = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /TASKS=`"$tasks`" /LOG=`"$setup.log`" $extra"
    (Start-Process $setup -ArgumentList $arguments -PassThru -Wait).ExitCode
}
function Uninstall {
    $uninstaller = (Get-ItemProperty $uninstallKey).UninstallString.Trim('"')
    $code = (Start-Process $uninstaller -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -PassThru -Wait).ExitCode
    Start-Sleep -Seconds 2
    $code
}
function InstalledVersion { (Get-ItemProperty $uninstallKey).DisplayVersion }
function CompareState([string]$label, [string]$before) {
    $after = Snapshot
    Check $label ($after -eq $before)
    if ($after -ne $before) { $results.Add("   antes: $before"); $results.Add("   depois: $after") }
}

if (Test-Path $uninstallKey) { throw "MariPDF já está instalado neste usuário; desinstale antes do teste." }
$before = Snapshot
try {
    $v1 = Build "0.9.0"; $v2 = Build "0.9.1"; $old = Build "0.8.0"
    $rc2 = Build "1.0.0-rc.2"; $rc10 = Build "1.0.0-rc.10"

    # ── instalação limpa, upgrade e downgrade ─────────────────────────────
    $code = Run $v1
    Check "instalação limpa sem admin (exit $code)" ($code -eq 0)
    Check "executável em %LOCALAPPDATA%\Programs\MariPDF" (Test-Path "$installDir\maripdf.exe")
    Check "atalho no Menu Iniciar" (Test-Path $startMenuLink)
    Check "entrada de desinstalação 0.9.0" ((InstalledVersion) -eq "0.9.0")
    $cmd = (Get-ItemProperty "$progIdKey\shell\open\command").'(default)'
    Check "ProgID abre o executável instalado" ($cmd -like "*Programs\MariPDF\maripdf.exe*%1*")
    Check "Capabilities + RegisteredApplications" ((Get-ItemProperty "HKCU:\Software\RegisteredApplications").MariPDF -eq "Software\MariPDF\Capabilities")
    $legacy = (Get-ItemProperty "HKCU:\Software\Classes\.pdf" -ErrorAction SilentlyContinue).'(default)'
    Check "não sobrescreve o padrão legado de .pdf" ($legacy -ne "MariPDF.Document")

    $code = Run $v2
    Check "upgrade 0.9.0 → 0.9.1 (exit $code)" ($code -eq 0 -and (InstalledVersion) -eq "0.9.1")
    $code = Run $old
    Check "downgrade 0.9.1 → 0.8.0 bloqueado (exit $code)" ($code -ne 0 -and (InstalledVersion) -eq "0.9.1")
    $code = Run $rc10
    Check "upgrade para 1.0.0-rc.10 (versão RC compila e instala, exit $code)" ($code -eq 0 -and (InstalledVersion) -eq "1.0.0-rc.10")
    $code = Run $rc2
    Check "downgrade rc.10 → rc.2 bloqueado (exit $code)" ($code -ne 0 -and (InstalledVersion) -eq "1.0.0-rc.10")

    $code = Uninstall
    Check "desinstalação (exit $code)" ($code -eq 0)
    Check "executável removido" (-not (Test-Path "$installDir\maripdf.exe"))
    Check "atalho removido" (-not (Test-Path $startMenuLink))
    CompareState "HKCU igual ao estado anterior" $before

    # ── associação recusada no setup e feita depois pelo app ──────────────
    $code = Run $v1 ""
    Check "instalação sem a tarefa de associação (exit $code)" ($code -eq 0 -and -not (Test-Path $progIdKey))
    $command = '"' + "$installDir\maripdf.exe" + '" "%1"'
    New-Item -Force "$progIdKey\shell\open\command" | Set-ItemProperty -Name "(default)" -Value $command
    New-Item -Force "HKCU:\Software\Classes\Applications\maripdf.exe\SupportedTypes" | Out-Null
    New-Item -Force "HKCU:\Software\MariPDF\Capabilities\FileAssociations" | Out-Null
    New-Item -Force "HKCU:\Software\Classes\.pdf\OpenWithProgids" -ErrorAction SilentlyContinue | Out-Null
    Set-ItemProperty "HKCU:\Software\Classes\.pdf\OpenWithProgids" "MariPDF.Document" ""
    Set-ItemProperty "HKCU:\Software\RegisteredApplications" "MariPDF" "Software\MariPDF\Capabilities"
    $code = Uninstall
    CompareState "associação feita pelo app é removida na desinstalação" $before

    # ── pasta escolhida com arquivos alheios ──────────────────────────────
    $custom = Join-Path $work "pasta-do-usuario"
    New-Item -ItemType Directory $custom | Out-Null
    Set-Content (Join-Path $custom "documento-importante.txt") "não apagar"
    $code = Run $v1 "" "/DIR=`"$custom`""
    Check "instalação em pasta escolhida (exit $code)" ($code -eq 0 -and (Test-Path (Join-Path $custom "maripdf.exe")))
    $code = Uninstall
    Check "arquivos alheios na pasta escolhida preservados" (Test-Path (Join-Path $custom "documento-importante.txt"))
    Check "executável removido da pasta escolhida" (-not (Test-Path (Join-Path $custom "maripdf.exe")))
    CompareState "HKCU limpo ao final" $before
}
finally {
    $results | ForEach-Object { Write-Host $_ }
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
if ($failed) { exit 1 }
