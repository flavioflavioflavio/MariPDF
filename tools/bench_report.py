"""Resume os relatórios do medidor (tools/bench.ps1) em summary.md.

Para cada PDF agrega as rodadas pela mediana; a primeira rodada costuma ser
fria (DLL e drivers fora do cache do sistema) e aparece separada.
"""
import statistics
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

out_dir = Path(sys.argv[1])
manifest = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("bench/corpus.toml")
expected_failure = set()
if manifest.exists():
    with manifest.open("rb") as fh:
        expected_failure = {p["file"] for p in tomllib.load(fh)["pdf"] if p["category"] == "error"}
by_file = defaultdict(list)
for path in sorted(out_dir.glob("*.toml")):
    name, run, _ = path.name.rsplit(".", 2)
    with path.open("rb") as fh:
        by_file[name].append((int(run), tomllib.load(fh)))


def step(report, name):
    for s in report.get("steps", []):
        if s["name"] == name:
            return s["ms"] if s["ok"] else None
    return None


def med(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def fmt(value, digits=0):
    return "—" if value is None else f"{value:.{digits}f}"


rows = []
for name, runs in sorted(by_file.items()):
    runs.sort()
    reports = [r for _, r in runs]
    warm = reports[1:] or reports
    ok = all(r.get("ok") for r in reports)
    if name in expected_failure:
        # Deve falhar ao abrir, com mensagem e sem travar: isso é o sucesso.
        refused = all(
            not r.get("ok") and r.get("steps", [{}])[0].get("name") == "WaitDocument"
            for r in reports
        )
        status = "✅ recusado" if refused else "❌"
    else:
        status = "✅" if ok else "❌"
    rows.append([
        name,
        str(reports[0].get("pages", 0)),
        status,
        fmt(reports[0].get("startup_to_first_frame_ms")),
        fmt(med(r.get("startup_to_first_frame_ms") for r in warm)),
        fmt(med(r.get("open_ms") for r in warm), 1),
        fmt(med(step(r, "FirstPage") for r in warm), 1),
        fmt(med(step(r, "FarPage") for r in warm), 1),
        fmt(med(step(r, "NeighborPage") for r in warm), 1),
        fmt(med(step(r, "Zoom") for r in warm), 1),
        fmt(med(step(r, "Search") for r in warm), 1),
        fmt(med(r["ui_frames"]["p95_ms"] for r in warm if "ui_frames" in r), 1),
        fmt(max((r["ui_frames"]["max_ms"] for r in warm if "ui_frames" in r), default=None), 1),
        fmt(med(r.get("working_set_mb") for r in warm)),
        fmt(max((r.get("peak_working_set_mb", 0) for r in warm), default=None)),
    ])

header = [
    "PDF", "Págs", "OK", "1º frame frio (ms)", "1º frame (ms)", "Abrir (ms)",
    "1ª página (ms)", "Salto sem cache (ms)", "Vizinha c/ prefetch (ms)",
    "Zoom (ms)", "Busca 1º hit (ms)", "Frame p95 (ms)", "Frame máx (ms)",
    "Working set (MB)", "Pico (MB)",
]
lines = [
    "| " + " | ".join(header) + " |",
    "|" + "|".join("---" for _ in header) + "|",
]
lines += ["| " + " | ".join(r) + " |" for r in rows]
summary = "\n".join(lines) + "\n"
(out_dir / "summary.md").write_text(summary, encoding="utf-8")
print(summary)
