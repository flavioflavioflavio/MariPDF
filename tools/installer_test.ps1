# tools/installer_test.ps1 — Matriz de instalação do MariPDF (BF-608).
#
# Instala, atualiza, tenta downgrade e desinstala o instalador por usuário,
# conferindo arquivos, atalhos, registro e a entrada de desinstalação. Ao
# final confere que o HKCU voltou ao estado anterior. Usa o ISCC para gerar
# versões vizinhas (anterior/posterior) a partir do mesmo executável.
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
$keys = @(
    "HKCU:\Software\Classes\MariPDF.Document",
    "HKCU:\Software\Classes\Applications\maripdf.exe",
    "HKCU:\Software\MariPDF",
    $uninstallKey
)
$results = [System.Collections.Generic.List[string]]::new()
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
function Run([string]$setup, [string]$extra = "") {
    $log = "$setup.log"
    $args = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /TASKS=""associate"" /LOG=""$log"" $extra"
    $p = Start-Process $setup -ArgumentList $args -PassThru -Wait
    $p.ExitCode
}

if (Test-Path $uninstallKey) { throw "MariPDF já está instalado neste usuário; desinstale antes do teste." }
$before = Snapshot
$failed = $false
try {
    $v1 = Build "0.9.0"; $v2 = Build "0.9.1"; $old = Build "0.8.0"

    $code = Run $v1
    Check "instalação limpa sem admin (exit $code)" ($code -eq 0)
    Check "executável em %LOCALAPPDATA%\Programs\MariPDF" (Test-Path "$installDir\maripdf.exe")
    Check "atalho no Menu Iniciar" (Test-Path "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\MariPDF.lnk")
    Check "entrada de desinstalação 0.9.0" ((Get-ItemProperty $uninstallKey).DisplayVersion -eq "0.9.0")
    $cmd = (Get-ItemProperty "HKCU:\Software\Classes\MariPDF.Document\shell\open\command").'(default)'
    Check "ProgID abre o executável instalado" ($cmd -like "*Programs\MariPDF\maripdf.exe*%1*")
    Check "Capabilities + RegisteredApplications" ((Get-ItemProperty "HKCU:\Software\RegisteredApplications").MariPDF -eq "Software\MariPDF\Capabilities")
    $legacy = (Get-ItemProperty "HKCU:\Software\Classes\.pdf" -ErrorAction SilentlyContinue).'(default)'
    Check "não sobrescreve o padrão legado de .pdf" ($legacy -ne "MariPDF.Document")

    $code = Run $v2
    Check "upgrade 0.9.0 → 0.9.1 (exit $code)" ($code -eq 0 -and (Get-ItemProperty $uninstallKey).DisplayVersion -eq "0.9.1")

    $code = Run $old
    Check "downgrade 0.9.1 → 0.8.0 bloqueado (exit $code)" ($code -ne 0 -and (Get-ItemProperty $uninstallKey).DisplayVersion -eq "0.9.1")

    $uninstaller = (Get-ItemProperty $uninstallKey).UninstallString.Trim('"')
    $p = Start-Process $uninstaller -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -PassThru -Wait
    Start-Sleep -Seconds 2
    Check "desinstalação (exit $($p.ExitCode))" ($p.ExitCode -eq 0)
    Check "pasta do programa removida" (-not (Test-Path "$installDir\maripdf.exe"))
    Check "atalho removido" (-not (Test-Path "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\MariPDF.lnk"))
    $after = Snapshot
    Check "HKCU igual ao estado anterior" ($after -eq $before)
    if ($after -ne $before) { $results.Add("   antes: $before"); $results.Add("   depois: $after") }
}
finally {
    $results | ForEach-Object { Write-Host $_ }
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
if ($failed) { exit 1 }
