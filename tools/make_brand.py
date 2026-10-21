"""Gera os assets de marca do MariPDF a partir dos originais do Illustrator.

Uso:
    python tools/make_brand.py ../ID

Entrada (pasta ID): ICON.svg, Logo_MariPDF.svg e Logo_MariPDF_HORIZONTAL.ai.
Saída:
    assets/brand/maripdf-icon.svg      ícone (só a camada visível, recortado)
    assets/brand/maripdf-logo.svg      logo vertical (ícone + nome)
    assets/icon.png / assets/icon.ico  ícone do app, do .exe e do instalador
    assets/logo-horizontal.png         nome com o M do ícone (Home e Sobre)

Requer: Pillow, resvg-py e pypdfium2 (o .ai é compatível com PDF).
"""
import io
import re
import sys
from pathlib import Path

import pypdfium2 as pdfium
import resvg_py
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"


def visible_svg(source: Path) -> str:
    """Remove as camadas ocultas (class st4 = display:none) e recorta a arte."""
    text = source.read_text(encoding="utf-8")
    text = re.sub(r'<g id="Layer_\d+" class="st4">.*?\n  </g>\n', "", text, flags=re.S)
    text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
    return text


def render_svg(svg: str, size: int) -> Image.Image:
    png = resvg_py.svg_to_bytes(svg_string=svg, width=size, height=size)
    return Image.open(io.BytesIO(bytes(png))).convert("RGBA")


def crop_to_art(image: Image.Image, pad_ratio: float = 0.0, square: bool = False) -> Image.Image:
    box = image.getchannel("A").getbbox()
    art = image.crop(box)
    w, h = art.size
    side_w = w if not square else max(w, h)
    side_h = h if not square else max(w, h)
    pad = round(max(side_w, side_h) * pad_ratio)
    canvas = Image.new("RGBA", (side_w + 2 * pad, side_h + 2 * pad), (0, 0, 0, 0))
    canvas.paste(art, ((canvas.width - w) // 2, (canvas.height - h) // 2), art)
    return canvas


def main() -> None:
    id_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "../ID").resolve()
    brand = ASSETS / "brand"
    brand.mkdir(exist_ok=True)

    icon_svg = visible_svg(id_dir / "ICON.svg")
    logo_svg = visible_svg(id_dir / "Logo_MariPDF.svg")
    (brand / "maripdf-icon.svg").write_text(icon_svg, encoding="utf-8")
    (brand / "maripdf-logo.svg").write_text(logo_svg, encoding="utf-8")

    # Ícone do app: M em fundo transparente, com respiro para não encostar
    # nas bordas da barra de tarefas.
    icon = crop_to_art(render_svg(icon_svg, 2048), pad_ratio=0.06, square=True)
    icon = icon.resize((512, 512), Image.LANCZOS)
    icon.save(ASSETS / "icon.png", optimize=True)
    sizes = [(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)]
    icon.save(ASSETS / "icon.ico", sizes=sizes)

    # Horizontal: só existe no .ai; o PDFium renderiza com fundo transparente.
    pdf = pdfium.PdfDocument(str(id_dir / "Logo_MariPDF_HORIZONTAL.ai"))
    page = pdf[0]
    wordmark = page.render(scale=6.0, fill_color=(0, 0, 0, 0)).to_pil().convert("RGBA")
    wordmark = crop_to_art(wordmark)
    height = 132  # 3× a altura exibida (44 px) para telas de alta densidade
    width = round(wordmark.width * height / wordmark.height)
    wordmark.resize((width, height), Image.LANCZOS).save(ASSETS / "logo-horizontal.png", optimize=True)

    for name in ["icon.png", "icon.ico", "logo-horizontal.png"]:
        print(f"assets/{name}: {(ASSETS / name).stat().st_size} bytes")


if __name__ == "__main__":
    main()
