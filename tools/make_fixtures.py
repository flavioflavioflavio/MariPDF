"""Gera fixtures pequenas e determinísticas para os testes do MariPDF."""
import io
import sys
from pathlib import Path

from pypdf import PdfReader, PdfWriter

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)


def page_stream(lines):
    body = "BT /F1 14 Tf\n" + "".join(
        f"1 0 0 1 72 {y} Tm ({text}) Tj\n" for text, y in lines
    ) + "ET\n"
    return body


def build(pages):
    """PDF mínimo, uma fonte Helvetica, uma página por item de `pages`."""
    objs = ["<< /Type /Catalog /Pages 2 0 R >>", None]
    kids = []
    font_id = 3
    objs.append("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
    for lines in pages:
        content = page_stream(lines)
        content_id = len(objs) + 1
        objs.append(f"<< /Length {len(content)} >>\nstream\n{content}endstream")
        page_id = len(objs) + 1
        objs.append(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            f"/Contents {content_id} 0 R /Resources << /Font << /F1 {font_id} 0 R >> >> >>"
        )
        kids.append(f"{page_id} 0 R")
    objs[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {len(kids)} >>"
    data = b"%PDF-1.4\n"
    offsets = []
    for i, body in enumerate(objs, 1):
        offsets.append(len(data))
        data += f"{i} 0 obj\n{body}\nendobj\n".encode("latin-1")
    xref = len(data)
    data += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
    for off in offsets:
        data += f"{off:010d} 00000 n \n".encode()
    data += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return data


sample = build([
    [("Leitor de PDF MariPDF", 700), ("Busca com destaque na pagina", 680),
     ("segunda linha com MariPDF de novo", 660)],
    [("Pagina dois do documento de teste", 700), ("fim", 680)],
])
(out / "sample.pdf").write_bytes(sample)

writer = PdfWriter(clone_from=PdfReader(io.BytesIO(sample)))
writer.encrypt(user_password="segredo", owner_password="dono", algorithm="AES-128")
buf = io.BytesIO()
writer.write(buf)
(out / "encrypted.pdf").write_bytes(buf.getvalue())

# Cabeçalho válido seguido de lixo: o PDFium deve recusar sem derrubar o app.
(out / "corrupt.pdf").write_bytes(sample[:180] + b"\x00garbage\xff" * 20)

for name in ["sample.pdf", "encrypted.pdf", "corrupt.pdf"]:
    print(name, (out / name).stat().st_size, "bytes")
