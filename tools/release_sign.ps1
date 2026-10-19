# tools/release_sign.ps1 — Assina o SHA256SUMS de um rascunho de release (BF-607).
#
# Baixa o SHA256SUMS.txt do rascunho, assina com a chave local
# (~/.maripdf/release-signing-key.pem, nunca enviada ao GitHub), confere a
# assinatura e anexa SHA256SUMS.txt.sig ao rascunho.
#
#   pwsh tools/release_sign.ps1 -Tag v1.0.0-rc.1
#
# Requer: gh autenticado, Python com o pacote `cryptography`.

param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Repo = "flavioflavioflavio/MariPDF",
    [string]$Python = "python"
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$work = Join-Path $env:TEMP "maripdf-sign-$([guid]::NewGuid())"
New-Item -ItemType Directory $work | Out-Null
try {
    gh release download $Tag --repo $Repo --pattern "SHA256SUMS.txt" --dir $work
    if ($LASTEXITCODE -ne 0) { throw "não foi possível baixar SHA256SUMS.txt de $Tag" }
    & $Python tools/sign_release.py (Join-Path $work "SHA256SUMS.txt")
    if ($LASTEXITCODE -ne 0) { throw "assinatura falhou" }
    gh release upload $Tag --repo $Repo --clobber (Join-Path $work "SHA256SUMS.txt.sig")
    if ($LASTEXITCODE -ne 0) { throw "upload do .sig falhou" }
    Write-Host "SHA256SUMS.txt.sig anexado ao rascunho $Tag"
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
