"""Assina SHA256SUMS.txt com a chave Ed25519 de release (BF-606/607).

A chave privada nunca entra no repositório. Ordem de busca:
  1. variável MARIPDF_SIGNING_KEY (conteúdo PEM, usado se configurado no CI);
  2. arquivo --key (padrão: ~/.maripdf/release-signing-key.pem).

Saída: SHA256SUMS.txt.sig (assinatura de 64 bytes em base64), verificada pelo
app com a chave pública embutida em src/updater.rs antes de instalar.

    python tools/sign_release.py dist/SHA256SUMS.txt
"""
import argparse
import base64
import os
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

EMBEDDED_PUBLIC_KEY = bytes.fromhex(
    "bb025300b157d03b37f297c59dfc9c783d2d1295cce4bf8f099cdc39890adf5e"
)

parser = argparse.ArgumentParser()
parser.add_argument("sums", type=Path)
parser.add_argument("--key", type=Path, default=Path.home() / ".maripdf" / "release-signing-key.pem")
args = parser.parse_args()

pem = os.environ.get("MARIPDF_SIGNING_KEY", "").encode() or args.key.read_bytes()
key = serialization.load_pem_private_key(pem, password=None)
if not isinstance(key, Ed25519PrivateKey):
    raise SystemExit("a chave precisa ser Ed25519")
public = key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
if public != EMBEDDED_PUBLIC_KEY:
    raise SystemExit("chave não corresponde à chave pública embutida no app — abortado")

data = args.sums.read_bytes()
signature = key.sign(data)
key.public_key().verify(signature, data)
out = args.sums.with_name(args.sums.name + ".sig")
out.write_text(base64.b64encode(signature).decode() + "\n", encoding="ascii")
print(f"assinado: {out}")
