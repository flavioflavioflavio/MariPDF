"""Gera o corpus de benchmark do MariPDF (BF-001).

Nenhum PDF de terceiros: tudo é sintetizado aqui, de forma determinística
(reportlab com invariant=1 e sementes fixas). O manifesto `bench/corpus.toml`
lista cada arquivo, a categoria, o número de páginas e o SHA-256 esperado.

Uso:
    python tools/make_corpus.py bench/corpus            # gera os PDFs
    python tools/make_corpus.py bench/corpus --manifest bench/corpus.toml

Requer: reportlab, pypdf, pillow, cryptography. As fontes de "complex-fonts" vêm do
Windows (Segoe UI) e de fontes CID embutidas no reportlab (japonês).
"""
import hashlib
import io
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw
from pypdf import PdfReader, PdfWriter
from reportlab.lib.pagesizes import A3, A4, LETTER, landscape
from reportlab.lib.utils import ImageReader
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.cidfonts import UnicodeCIDFont
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

LOREM = (
    "MariPDF le documentos grandes sem travar a interface. "
    "Busca, selecao e navegacao precisam continuar fluidas em qualquer pagina. "
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod "
    "tempor incididunt ut labore et dolore magna aliqua."
)


def new_canvas(buffer, size=A4):
    return canvas.Canvas(buffer, pagesize=size, invariant=1)


def text_page(c, page_no, total, size=A4, font="Helvetica"):
    width, height = size
    c.setFont(font, 11)
    y = height - 72
    c.drawString(72, y, f"Pagina {page_no} de {total} - MariPDF benchmark")
    y -= 24
    rng = random.Random(page_no)
    while y > 72:
        words = LOREM.split()
        rng.shuffle(words)
        c.drawString(72, y, " ".join(words)[: int((width - 144) / 5.6)])
        y -= 15
    c.showPage()


def text_doc(pages):
    buf = io.BytesIO()
    c = new_canvas(buf)
    for i in range(1, pages + 1):
        text_page(c, i, pages)
    c.save()
    return buf.getvalue()


def scan_doc(pages=10):
    """Páginas inteiras como JPEG A4 a 300 dpi, como um documento escaneado."""
    buf = io.BytesIO()
    c = new_canvas(buf)
    for i in range(pages):
        rng = random.Random(1000 + i)
        img = Image.new("L", (2480, 3508), 236)
        draw = ImageDraw.Draw(img)
        for line in range(60):
            y = 200 + line * 52
            x = 180
            while x < 2300:
                w = rng.randint(30, 160)
                draw.rectangle([x, y, x + w, y + 24], fill=rng.randint(20, 70))
                x += w + rng.randint(18, 40)
        noise = Image.effect_noise((2480, 3508), 18)
        img = Image.blend(img, noise, 0.15)
        jpeg = io.BytesIO()
        img.save(jpeg, "JPEG", quality=70)
        jpeg.seek(0)
        c.drawImage(ImageReader(jpeg), 0, 0, *A4)
        c.showPage()
    c.save()
    return buf.getvalue()


def bookmarks_doc(pages=60):
    base = PdfReader(io.BytesIO(text_doc(pages)))
    writer = PdfWriter(clone_from=base)
    for chapter in range(6):
        first = chapter * 10
        parent = writer.add_outline_item(f"Capitulo {chapter + 1}", first)
        for section in range(3):
            child = writer.add_outline_item(
                f"Secao {chapter + 1}.{section + 1}", first + section * 3, parent=parent
            )
            writer.add_outline_item(
                f"Detalhe {chapter + 1}.{section + 1}.1", first + section * 3 + 1, parent=child
            )
    out = io.BytesIO()
    writer.write(out)
    return out.getvalue()


def links_doc(pages=20, per_page=50):
    buf = io.BytesIO()
    c = new_canvas(buf)
    for page in range(pages):
        c.setFont("Helvetica", 9)
        for i in range(per_page):
            col, row = divmod(i, 25)
            x = 72 + col * 240
            y = A4[1] - 90 - row * 28
            label = f"link {page + 1}.{i + 1}"
            c.drawString(x, y, label)
            c.linkURL(
                f"https://example.org/maripdf/{page + 1}/{i + 1}",
                (x, y - 2, x + 80, y + 10),
                relative=0,
            )
        c.showPage()
    c.save()
    return buf.getvalue()


def complex_fonts_doc():
    pdfmetrics.registerFont(TTFont("SegoeUI", r"C:\Windows\Fonts\segoeui.ttf"))
    pdfmetrics.registerFont(UnicodeCIDFont("HeiseiMin-W3"))
    buf = io.BytesIO()
    c = new_canvas(buf)
    samples = [
        ("SegoeUI", "Acentuação: ação, coração, pêssego, ÁÉÍÓÚ ãõ ç ü"),
        ("SegoeUI", "Ελληνικά: Αλφάβητο ελληνικό, καλημέρα κόσμε"),
        ("SegoeUI", "Кириллица: Съешь же ещё этих мягких французских булок"),
        ("SegoeUI", "Turco: İstanbul ılık şehir, ĞÜŞÖÇ"),
        ("HeiseiMin-W3", "日本語のテキスト：読みやすいPDFビューア"),
        ("HeiseiMin-W3", "東京 大阪 名古屋 札幌 福岡"),
    ]
    for page in range(5):
        y = A4[1] - 80
        for font, text in samples * 4:
            c.setFont(font, 13)
            c.drawString(72, y, text)
            y -= 26
            if y < 72:
                break
        c.showPage()
    c.save()
    return buf.getvalue()


def mixed_sizes_doc():
    sizes = [A4, LETTER, landscape(A3), (226, 600), (595, 3000), (300, 300)]
    buf = io.BytesIO()
    c = canvas.Canvas(buf, invariant=1)
    for i, size in enumerate(sizes * 2, 1):
        c.setPageSize(size)
        text_page(c, i, len(sizes) * 2, size=size)
    c.save()
    return buf.getvalue()


def encrypted_doc():
    writer = PdfWriter(clone_from=PdfReader(io.BytesIO(text_doc(3))))
    writer.encrypt(user_password="segredo", owner_password="dono", algorithm="AES-128")
    out = io.BytesIO()
    writer.write(out)
    return out.getvalue()


def corrupt_doc():
    good = text_doc(3)
    return good[:400] + bytes(random.Random(7).getrandbits(8) for _ in range(4000))


CORPUS = [
    # (arquivo, categoria, páginas esperadas ou None, propósito, gerador)
    ("text-small.pdf", "text", 2, "abertura e primeira página no caso comum", lambda: text_doc(2)),
    ("scan-10.pdf", "images", 10, "render pesado de imagens (scan 300 dpi)", scan_doc),
    ("text-100.pdf", "pages", 100, "navegação e prefetch em documento médio", lambda: text_doc(100)),
    ("text-1000.pdf", "pages", 1000, "documento grande: custo não linear de UI/layout", lambda: text_doc(1000)),
    ("bookmarks-60.pdf", "outline", 60, "sumário com 3 níveis", bookmarks_doc),
    ("links-20.pdf", "links", 20, "50 links URI por página", links_doc),
    ("complex-fonts.pdf", "fonts", 5, "acentos, grego, cirílico, turco e japonês", complex_fonts_doc),
    ("mixed-sizes.pdf", "layout", 12, "A4, Letter, A3 paisagem, recibo e página muito alta", mixed_sizes_doc),
    ("encrypted.pdf", "error", None, "protegido por senha (deve falhar com mensagem)", encrypted_doc),
    ("corrupt.pdf", "error", None, "estrutura corrompida (deve falhar sem crash)", corrupt_doc),
]


def main():
    out = Path(sys.argv[1])
    manifest = None
    if "--manifest" in sys.argv:
        manifest = Path(sys.argv[sys.argv.index("--manifest") + 1])
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    for name, category, pages, purpose, build in CORPUS:
        data = build()
        (out / name).write_bytes(data)
        # AES usa IV aleatório: o arquivo cifrado muda a cada geração.
        digest = "variable" if name == "encrypted.pdf" else hashlib.sha256(data).hexdigest()
        if pages is not None:
            actual = len(PdfReader(io.BytesIO(data)).pages)
            assert actual == pages, f"{name}: {actual} páginas, esperado {pages}"
        rows.append((name, category, pages, purpose, digest, len(data)))
        print(f"{name:20} {category:8} {str(pages):>5} pág  {len(data) / 1024:9.1f} KiB  {digest[:12]}")

    if manifest:
        lines = [
            "# Corpus de benchmark do MariPDF (BF-001) — gerado por tools/make_corpus.py.",
            "# Os PDFs não são versionados; regenere-os e confira os hashes.",
            "# Hashes de complex-fonts dependem da versão da Segoe UI do Windows.",
            "",
        ]
        for name, category, pages, purpose, digest, size in rows:
            lines += [
                "[[pdf]]",
                f'file = "{name}"',
                f'category = "{category}"',
                f"pages = {pages if pages is not None else -1}",
                f'purpose = "{purpose}"',
                f"bytes = {size}",
                f'sha256 = "{digest}"',
                "",
            ]
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text("\n".join(lines), encoding="utf-8")
        print("manifesto:", manifest)


if __name__ == "__main__":
    main()
