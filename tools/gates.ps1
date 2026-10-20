# tools/gates.ps1 — Os mesmos gates do CI, falhando no primeiro erro.
#
#   pwsh tools/gates.ps1                 # toolchain padrão
#   pwsh tools/gates.ps1 -Toolchain 1.99.1

param([string]$Toolchain = "")

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$tc = if ($Toolchain) { "+$Toolchain" } else { $null }

function Step([string]$name, [string[]]$cargoArgs) {
    Write-Host "── $name"
    $all = @($tc) + $cargoArgs | Where-Object { $_ }
    & cargo @all
    if ($LASTEXITCODE -ne 0) { throw "gate falhou: $name" }
}

Step "fmt" @("fmt", "--check")
Step "clippy" @("clippy", "--all-targets", "--all-features", "--", "-D", "warnings")
Step "test" @("test", "--all-targets")
Write-Host "Gates OK"
