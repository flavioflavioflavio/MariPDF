# tools/smoke.ps1 — Smoke test do app real (BF-007).
#
# Abre o executável com um PDF conhecido em modo medição, exige que o roteiro
# inteiro passe, que o processo saia sozinho e que o log não tenha panic.
# Usa estado isolado (MARIPDF_DATA_DIR) e pode rodar no CI.
#
#   pwsh tools/smoke.ps1                        # usa tests/fixtures/sample.pdf
#   pwsh tools/smoke.ps1 -Exe dist\MariPDF.exe -Pdf bench\corpus\text-100.pdf

param(
    [string]$Exe = "target\release\maripdf.exe",
    [string]$Pdf = "tests\fixtures\sample.pdf",
    [int]$TimeoutSec = 90
)

$ErrorActionPreference = "Stop"
$data = Join-Path $env:TEMP "maripdf-smoke-$([guid]::NewGuid())"
New-Item -ItemType Directory $data | Out-Null
$report = Join-Path $data "report.toml"

try {
    $env:MARIPDF_DATA_DIR = $data
    $env:MARIPDF_BENCH = $report
    $pdfPath = Resolve-Path $Pdf
    $proc = Start-Process -FilePath (Resolve-Path $Exe) -ArgumentList "`"$pdfPath`"" -PassThru
    if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
        $proc.Kill()
        throw "Smoke: o app não terminou o roteiro em $TimeoutSec s"
    }
    if (-not (Test-Path $report)) { throw "Smoke: relatório não gerado ($report)" }

    $text = Get-Content $report -Raw
    $topLevel = ($text -split "\[\[steps\]\]")[0]
    if ($topLevel -notmatch "(?m)^ok = true") {
        Write-Host $text
        throw "Smoke: roteiro com etapa falha"
    }
    $panics = Select-String -Path (Join-Path $data "*.log*") -Pattern "panicked"
    if ($panics) { $panics | ForEach-Object { Write-Host $_ }; throw "Smoke: panic no log" }

    $first = [regex]::Match($text, "startup_to_first_page_ms = ([\d.]+)").Groups[1].Value
    Write-Host "Smoke OK — primeira página em $([math]::Round([double]$first)) ms desde o início do processo"
}
finally {
    Remove-Item Env:\MARIPDF_DATA_DIR, Env:\MARIPDF_BENCH -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue
}
