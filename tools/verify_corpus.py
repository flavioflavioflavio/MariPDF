"""Confere o corpus de benchmark contra o manifesto antes de medir.

Uso: python tools/verify_corpus.py bench/corpus bench/corpus.toml [arquivo ...]

Cada PDF precisa bater em bytes e SHA-256 com o manifesto; entradas com
sha256 = "variable" (dependem de fontes do sistema) só precisam existir.
Sem isso, uma medição sobre PDFs diferentes seria comparada com a baseline.
"""
import hashlib
import sys
import tomllib
from pathlib import Path

corpus = Path(sys.argv[1])
manifest = Path(sys.argv[2])
only = set(sys.argv[3:])

with manifest.open("rb") as fh:
    entries = tomllib.load(fh)["pdf"]

problems = []
for entry in entries:
    name = entry["file"]
    if only and name not in only:
        continue
    path = corpus / name
    if not path.exists():
        problems.append(f"{name}: ausente")
        continue
    if entry.get("sha256") == "variable":
        continue
    data = path.read_bytes()
    if len(data) != entry["bytes"]:
        problems.append(f"{name}: {len(data)} bytes, manifesto diz {entry['bytes']}")
    elif hashlib.sha256(data).hexdigest() != entry["sha256"]:
        problems.append(f"{name}: SHA-256 diferente do manifesto")

if problems:
    sys.exit("corpus divergente (regenere com tools/make_corpus.py):\n  " + "\n  ".join(problems))
print("corpus confere com o manifesto")
