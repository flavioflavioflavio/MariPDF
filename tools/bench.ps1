# tools/bench.ps1 — Roda o medidor embutido (MARIPDF_BENCH) sobre o corpus (BF-002/BF-003).
#
# Uso:
#   python tools/make_corpus.py bench/corpus --manifest bench/corpus.toml
#   cargo build --release
#   pwsh tools/bench.ps1                      # todo o corpus, 3 rodadas
#   pwsh tools/bench.ps1 -Runs 1 -Only text-1000.pdf
#
# Cada execução usa um MARIPDF_DATA_DIR temporário: nada toca o estado real.
# Saída: bench/results/<timestamp>/<arquivo>.<rodada>.toml + summary.md

param(
    [string]$Exe = "target\release\maripdf.exe",
    [string]$Corpus = "bench\corpus",
    [string]$Manifest = "bench\corpus.toml",
    [int]$Runs = 3,
    [int]$TimeoutSec = 120,
    [string[]]$Only = @()
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path $Exe)) { throw "Executável não encontrado: $Exe (rode cargo build --release)" }

$files = Select-String -Path $Manifest -Pattern '^file = "(.+)"' | ForEach-Object { $_.Matches[0].Groups[1].Value }
# Aceita -Only a.pdf,b.pdf tanto em chamada direta quanto via `pwsh -File`.
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
if ($Only.Count -gt 0) { $files = $files | Where-Object { $Only -contains $_ } }

# Medir PDFs diferentes dos do manifesto invalidaria a comparação com a baseline.
python tools/verify_corpus.py $Corpus $Manifest @files
if ($LASTEXITCODE -ne 0) { throw "Corpus não confere com $Manifest" }

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outDir = Join-Path "bench\results" $stamp
New-Item -ItemType Directory -Force $outDir | Out-Null

$missing = [System.Collections.Generic.List[string]]::new()
foreach ($file in $files) {
    $pdf = Resolve-Path (Join-Path $Corpus $file)
    for ($run = 1; $run -le $Runs; $run++) {
        $data = Join-Path $env:TEMP "maripdf-bench-$([guid]::NewGuid())"
        New-Item -ItemType Directory $data | Out-Null
        $report = Join-Path (Resolve-Path $outDir) "$file.$run.toml"
        $env:MARIPDF_DATA_DIR = $data
        $env:MARIPDF_BENCH = $report
        $proc = Start-Process -FilePath $Exe -ArgumentList "`"$pdf`"" -PassThru
        if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
            $proc.Kill()
            Write-Warning "$file (rodada $run): timeout"
        }
        $panics = Select-String -Path (Join-Path $data "*.log*") -Pattern "panicked" -ErrorAction SilentlyContinue
        if ($panics) { Write-Warning "$file (rodada $run): panic no log"; Copy-Item (Join-Path $data "*.log*") $outDir }
        Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue
        if (-not (Test-Path $report)) { $missing.Add("$file (rodada $run)") }
        Write-Host ("{0,-20} rodada {1}: {2}" -f $file, $run, $(if (Test-Path $report) { "ok" } else { "SEM RELATÓRIO" }))
    }
}
Remove-Item Env:\MARIPDF_DATA_DIR, Env:\MARIPDF_BENCH -ErrorAction SilentlyContinue

python tools/bench_report.py $outDir $Manifest $Runs
$reportExit = $LASTEXITCODE
Write-Host "Resultados em $outDir"
if ($missing.Count -gt 0) {
    throw "Rodadas sem relatório (travou, crashou ou excedeu $TimeoutSec s): $($missing -join ', ')"
}
if ($reportExit -ne 0) { throw "bench_report.py: rodadas faltando ou reprovadas" }
