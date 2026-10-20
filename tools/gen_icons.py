import json
import sys

mapping = json.load(open(sys.argv[1]))
out_path = sys.argv[2]

# (constante, nome Phosphor, comentário)
ICONS = [
    ("Navegação", [
        ("ARROW_LEFT", "arrow-left", "voltar"),
        ("ARROW_RIGHT", "arrow-right", ""),
        ("ARROW_UP", "arrow-up", ""),
        ("CARET_LEFT", "caret-left", "página anterior / expandir"),
        ("CARET_RIGHT", "caret-right", "próxima página / recolher"),
        ("CARET_DOWN", "caret-down", "abre menu"),
    ]),
    ("Ações", [
        ("MAGNIFY_GLASS", "magnifying-glass", "busca"),
        ("X_CIRCLE", "x-circle", "fechar/dispensar"),
        ("X", "x", "fechar barra/overlay"),
        ("CHECK", "check", ""),
        ("FILE_PDF", "file-pdf", ""),
        ("FOLDER_OPEN", "folder-open", ""),
        ("CLOCK", "clock", "recentes"),
        ("UPLOAD_SIMPLE", "upload-simple", "drop zone"),
        ("ARROW_COUNTER_CW", "arrow-counter-clockwise", "reset"),
    ]),
    ("Tema", [
        ("MOON", "moon", "Dark"),
        ("SUN", "sun", "Light"),
        ("LAMP_PENDANT", "lamp-pendant", "Sépia"),
        ("BOOK_OPEN", "book-open", ""),
    ]),
    ("Zoom e visualização", [
        ("MINUS", "minus", "afastar"),
        ("PLUS", "plus", "aproximar"),
        ("ARROWS_OUT", "arrows-out", ""),
        ("ARROWS_OUT_LINE_H", "arrows-out-line-horizontal", "ajustar largura"),
        ("FRAME_CORNERS", "frame-corners", "ajustar página"),
        ("FILE", "file", "modo página única"),
        ("ROWS", "rows", "modo contínuo"),
        ("SQUARE_HALF", "square-half", ""),
        ("PICTURE_IN_PICTURE", "picture-in-picture", "modo Flutuante"),
        ("HAND", "hand", "ferramenta mão (pan)"),
        ("PUSH_PIN", "push-pin", "sempre no topo"),
    ]),
    ("Estado", [
        ("WARNING", "warning", ""),
        ("SPINNER_GAP", "spinner-gap", "carregando"),
    ]),
    ("Painéis e configurações", [
        ("LIST", "list", "sumário/TOC"),
        ("SQUARES_FOUR", "squares-four", "miniaturas"),
        ("SLIDERS", "sliders-horizontal", ""),
        ("GEAR", "gear", "configurações"),
        ("PAINT_BRUSH", "paint-brush", "aba Aparência"),
        ("BOOK", "book", "aba Leitura"),
        ("SHIELD", "shield", "aba Privacidade"),
        ("INFO", "info", "sobre"),
    ]),
]

lines = [
    "// icons.rs — Mapeamento de ícones Phosphor para uso no egui",
    "//",
    "// Fonte: Phosphor Regular 2.1 (assets/phosphor.ttf) embutida no binário.",
    "// Uso: `ph(ARROW_LEFT, 16.0, color)` → RichText com FontFamily::Name(\"Icons\").",
    "//",
    "// Codepoints conferidos contra o CSS oficial de @phosphor-icons/web@2.1.1",
    "// (src/regular/style.css); todos existem no cmap da fonte embutida.",
    "// Ao trocar a fonte, regenere a tabela a partir do CSS da mesma versão.",
    "",
    "#![allow(dead_code)]",
    "",
    "use eframe::egui;",
    "",
]
for section, icons in ICONS:
    lines.append(f"// ── {section} " + "─" * max(4, 74 - len(section)))
    lines.append("")
    for const, name, note in icons:
        code = mapping[name]
        comment = f"// {name}" + (f" — {note}" if note else "")
        lines.append(f'pub const {const}: &str = "\\u{{{code:x}}}"; {comment}')
    lines.append("")

lines += [
    "// ── Helper " + "─" * 68,
    "",
    "/// Cria um `RichText` com o ícone Phosphor solicitado.",
    "/// `glyph` = constante `icons::*`, `size` = tamanho em pts, `color` = cor RGBA.",
    "pub fn ph(glyph: &str, size: f32, color: egui::Color32) -> egui::RichText {",
    "    egui::RichText::new(glyph)",
    "        .size(size)",
    "        .color(color)",
    "        .font(egui::FontId::new(",
    "            size,",
    '            egui::FontFamily::Name("Icons".into()),',
    "        ))",
    "}",
    "",
]
open(out_path, "w", encoding="utf-8", newline="\n").write("\n".join(lines))
print("wrote", out_path)
