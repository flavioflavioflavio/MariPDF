"""Gera as imagens do pacote MSIX (Microsoft Store) a partir da marca.

Uso:
    python tools/make_msix_assets.py <pasta Assets de saída>

Fonte: assets/icon.png (M vermelho, fundo transparente) e
assets/logo-horizontal.png. Gera os tamanhos de bloco, barra de tarefas
(targetsize, com e sem placa) e o logo da Store. Requer Pillow.
"""
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ICON = Image.open(ROOT / "assets" / "icon.png").convert("RGBA")
WORDMARK = Image.open(ROOT / "assets" / "logo-horizontal.png").convert("RGBA")


def fit(source: Image.Image, box: tuple[int, int], fill: float) -> Image.Image:
    """Centraliza `source` num quadro transparente ocupando `fill` do lado menor."""
    width, height = box
    scale = min(width * fill / source.width, height * fill / source.height)
    art = source.resize(
        (max(1, round(source.width * scale)), max(1, round(source.height * scale))),
        Image.LANCZOS,
    )
    canvas = Image.new("RGBA", box, (0, 0, 0, 0))
    canvas.paste(art, ((width - art.width) // 2, (height - art.height) // 2), art)
    return canvas


def main() -> None:
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "Assets")
    out.mkdir(parents=True, exist_ok=True)
    images = {}
    for scale in (100, 200):
        k = scale / 100
        images[f"Square150x150Logo.scale-{scale}.png"] = fit(ICON, (round(150 * k), round(150 * k)), 0.62)
        images[f"Square44x44Logo.scale-{scale}.png"] = fit(ICON, (round(44 * k), round(44 * k)), 0.86)
        images[f"Wide310x150Logo.scale-{scale}.png"] = fit(WORDMARK, (round(310 * k), round(150 * k)), 0.70)
        images[f"StoreLogo.scale-{scale}.png"] = fit(ICON, (round(50 * k), round(50 * k)), 0.86)
        images[f"FileLogo.scale-{scale}.png"] = fit(ICON, (round(44 * k), round(44 * k)), 0.86)
    # Barra de tarefas e menu Iniciar: tamanhos exatos, com e sem placa.
    for size in (16, 24, 32, 48, 256):
        icon = fit(ICON, (size, size), 0.92)
        images[f"Square44x44Logo.targetsize-{size}.png"] = icon
        images[f"Square44x44Logo.targetsize-{size}_altform-unplated.png"] = icon
        images[f"Square44x44Logo.altform-lightunplated_targetsize-{size}.png"] = icon
    for name, image in images.items():
        image.save(out / name, optimize=True)
    print(f"{len(images)} imagens em {out}")


if __name__ == "__main__":
    main()
