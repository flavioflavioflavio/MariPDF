# tools/package.ps1 — Gera os artefatos de distribuição (BF-603/607).
#
#   pwsh tools/package.ps1                 # build release + portátil + instalador + checksums
#   pwsh tools/package.ps1 -SkipBuild      # reaproveita target/release/maripdf.exe
#
# Saída em dist/:
#   MariPDF-<versão>-portable.exe   executável único (PDFium embutido)
#   MariPDF-<versão>-setup.exe      instalador por usuário (sem admin)
#   SHA256SUMS.txt                  hashes dos dois artefatos

param(
    [switch]$SkipBuild,
    [string]$Iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "MariPDF $version"

if (-not $SkipBuild) {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release falhou" }
}
$exe = "target\release\maripdf.exe"
if (-not (Test-Path $exe)) { throw "Executável não encontrado: $exe" }

$dist = "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
Get-ChildItem $dist -Filter "MariPDF-*" -ErrorAction SilentlyContinue | Remove-Item -Force

# Smoke test no binário que será publicado
pwsh -NoProfile -File tools/smoke.ps1 -Exe $exe
if ($LASTEXITCODE -ne 0) { throw "smoke test falhou" }

$portable = Join-Path $dist "MariPDF-$version-portable.exe"
Copy-Item $exe $portable

if (-not (Test-Path $Iscc)) { throw "Inno Setup não encontrado em $Iscc (choco install innosetup)" }
& $Iscc /Q "/DAppVersion=$version" "/DSourceExe=..\$exe" "/DOutputDir=..\$dist" installer\maripdf.iss
if ($LASTEXITCODE -ne 0) { throw "ISCC falhou" }
$setup = Join-Path $dist "MariPDF-$version-setup.exe"

$sums = Get-ChildItem $dist -Filter "MariPDF-$version-*.exe" | Sort-Object Name | ForEach-Object {
    "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.Name
}
# LF e sem BOM: formato do sha256sum, verificável em qualquer sistema.
[IO.File]::WriteAllText((Join-Path (Resolve-Path $dist) "SHA256SUMS.txt"), ($sums -join "`n") + "`n")

Get-ChildItem $dist | Format-Table Name, @{n = "MB"; e = { [math]::Round($_.Length / 1MB, 2) } } -AutoSize
Get-Content (Join-Path $dist "SHA256SUMS.txt")
