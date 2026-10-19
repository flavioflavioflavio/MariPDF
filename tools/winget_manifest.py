"""Gera os manifestos Winget de uma versão a partir do SHA256SUMS (BF-609).

    python tools/winget_manifest.py 1.0.0 dist/SHA256SUMS.txt

Saída: dist/winget/manifests/f/FlavioTakemoto/MariPDF/<versão>/*.yaml, prontos
para `winget validate` e para um PR em microsoft/winget-pkgs (envio manual).
"""
import sys
from pathlib import Path

version, sums_path = sys.argv[1], Path(sys.argv[2])
setup = f"MariPDF-{version}-setup.exe"
sha = next(
    line.split()[0].upper()
    for line in sums_path.read_text(encoding="utf-8").splitlines()
    if line.strip().endswith(setup)
)
package = "FlavioTakemoto.MariPDF"
schema = "1.6.0"
url = f"https://github.com/flavioflavioflavio/MariPDF/releases/download/v{version}/{setup}"
out = sums_path.parent / "winget" / "manifests" / "f" / "FlavioTakemoto" / "MariPDF" / version
out.mkdir(parents=True, exist_ok=True)

files = {
    f"{package}.yaml": f"""PackageIdentifier: {package}
PackageVersion: {version}
DefaultLocale: pt-BR
ManifestType: version
ManifestVersion: {schema}
""",
    f"{package}.installer.yaml": f"""PackageIdentifier: {package}
PackageVersion: {version}
MinimumOSVersion: 10.0.0.0
InstallerType: inno
Scope: user
InstallModes:
  - interactive
  - silent
  - silentWithProgress
InstallerSwitches:
  Silent: /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
  SilentWithProgress: /SILENT /SUPPRESSMSGBOXES /NORESTART
UpgradeBehavior: install
ProductCode: '{{73F72B7B-8E4F-4704-8CBA-9FB568284922}}_is1'
FileExtensions:
  - pdf
Installers:
  - Architecture: x64
    InstallerUrl: {url}
    InstallerSha256: {sha}
ManifestType: installer
ManifestVersion: {schema}
""",
    f"{package}.locale.pt-BR.yaml": f"""PackageIdentifier: {package}
PackageVersion: {version}
PackageLocale: pt-BR
Publisher: Flávio Takemoto
PublisherUrl: https://www.linkedin.com/in/takemotoflavio/
PackageName: MariPDF
PackageUrl: https://github.com/flavioflavioflavio/MariPDF
License: GPL-3.0-or-later
LicenseUrl: https://github.com/flavioflavioflavio/MariPDF/blob/master/LICENSE
ShortDescription: Leitor de PDF rápido e focado para Windows.
Description: Leitor de PDF open source, sem conta, sem telemetria e sem edição. Interface que não trava em documentos grandes, busca com destaque, modo Flutuante e tema claro/escuro/sépia.
Tags:
  - pdf
  - leitor
  - viewer
ReleaseNotesUrl: https://github.com/flavioflavioflavio/MariPDF/releases/tag/v{version}
ManifestType: defaultLocale
ManifestVersion: {schema}
""",
    f"{package}.locale.en-US.yaml": f"""PackageIdentifier: {package}
PackageVersion: {version}
PackageLocale: en-US
Publisher: Flávio Takemoto
PackageName: MariPDF
License: GPL-3.0-or-later
ShortDescription: A fast, focused PDF reader for Windows.
ManifestType: locale
ManifestVersion: {schema}
""",
}
for name, body in files.items():
    (out / name).write_text(body, encoding="utf-8")
print(f"manifestos Winget em {out}")
