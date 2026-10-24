"""Gera os PDFs de demonstração usados nas capturas de tela do README.

Uso:
    python tools/make_demo_pdfs.py <pasta>

Textos originais (sem direitos de terceiros), com sumário navegável, títulos,
citações, tabelas e gráficos, para os prints mostrarem um documento real.
Requer reportlab.
"""
import random
import sys
from pathlib import Path

from reportlab.graphics.charts.barcharts import VerticalBarChart
from reportlab.graphics.charts.linecharts import HorizontalLineChart
from reportlab.graphics.shapes import Drawing, Rect, String
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import mm
from reportlab.platypus import (
    BaseDocTemplate,
    Frame,
    NextPageTemplate,
    PageBreak,
    PageTemplate,
    Paragraph,
    Spacer,
    Table,
    TableStyle,
)

RED = colors.HexColor("#AF0C0C")
CORAL = colors.HexColor("#F15855")
INK = colors.HexColor("#1F1D24")
MUTED = colors.HexColor("#6B6874")

BODY = ParagraphStyle("body", fontName="Times-Roman", fontSize=11.5, leading=17,
                      alignment=TA_JUSTIFY, textColor=INK, spaceAfter=9)
LEAD = ParagraphStyle("lead", parent=BODY, fontSize=13, leading=19, textColor=MUTED)
H1 = ParagraphStyle("h1", fontName="Helvetica-Bold", fontSize=24, leading=29,
                    textColor=INK, spaceBefore=6, spaceAfter=14)
H2 = ParagraphStyle("h2", fontName="Helvetica-Bold", fontSize=14, leading=19,
                    textColor=RED, spaceBefore=12, spaceAfter=6)
KICKER = ParagraphStyle("kicker", fontName="Helvetica-Bold", fontSize=9,
                        leading=12, textColor=RED, spaceAfter=4)
QUOTE = ParagraphStyle("quote", fontName="Times-Italic", fontSize=15, leading=22,
                       textColor=INK, leftIndent=18, borderPadding=(6, 0, 6, 12),
                       spaceBefore=10, spaceAfter=14)
TOC = ParagraphStyle("toc", fontName="Times-Roman", fontSize=12.5, leading=24, textColor=INK)
TITLE = ParagraphStyle("title", fontName="Helvetica-Bold", fontSize=40, leading=46,
                       alignment=TA_CENTER, textColor=INK)
SUBTITLE = ParagraphStyle("subtitle", fontName="Times-Italic", fontSize=16, leading=22,
                          alignment=TA_CENTER, textColor=MUTED)

# Frases originais para compor parágrafos variados sobre leitura e documentos.
SENTENCES = [
    "Ler bem começa por enxergar bem: a página precisa aparecer inteira, nítida e sem demora.",
    "Um bom leitor de PDF some da frente e deixa o texto ocupar o centro da tela.",
    "Cada segundo de espera entre abrir um arquivo e ver a primeira página quebra a concentração.",
    "Por isso o MariPDF trata o desempenho como um recurso, e não como um detalhe técnico.",
    "Documentos longos pedem navegação rápida: sumário, miniaturas, busca e marcadores.",
    "A rolagem contínua mantém o ritmo da leitura, enquanto a página única favorece o estudo.",
    "Os temas escuro, claro e sépia acompanham o ambiente e cansam menos os olhos.",
    "No tema escuro, a página inverte as cores com suavidade, sem o branco agressivo da tela.",
    "Marcar uma página é tão simples quanto dobrar o canto de um livro de papel.",
    "A busca encontra frases mesmo quando elas quebram de linha no meio do parágrafo.",
    "Selecionar e copiar um trecho deve funcionar do jeito que a mão já espera.",
    "Atalhos de teclado deixam as tarefas frequentes a um gesto de distância.",
    "O modo flutuante transforma o PDF em uma janela de referência, sempre à vista.",
    "Nada é enviado para a internet: o arquivo é aberto, lido e fechado no seu computador.",
    "A interface responde enquanto as páginas ficam prontas em segundo plano.",
    "Ler é um ato de atenção, e a ferramenta certa protege essa atenção.",
    "Um índice bem organizado vale tanto quanto o próprio conteúdo do livro.",
    "Quem estuda volta muitas vezes ao mesmo trecho, por isso voltar precisa ser imediato.",
    "Relatórios, contratos, manuais e livros têm ritmos diferentes de leitura.",
    "A margem generosa dá respiro ao texto e ajuda o olho a encontrar a próxima linha.",
    "Cada recurso existe para tirar uma pequena fricção do caminho de quem lê.",
    "Um programa leve abre em instantes, ocupa pouco espaço e respeita a máquina.",
    "Arquivos protegidos por senha abrem com a senha certa, que nunca fica gravada.",
    "Quando o arquivo muda no disco, a nova versão aparece sem perder a página atual.",
]

CHAPTERS = [
    ("Por que um leitor rápido", "Antes de qualquer recurso, a velocidade"),
    ("A primeira página", "Do clique ao texto na tela"),
    ("Navegar sem se perder", "Sumário, miniaturas e marcadores"),
    ("Encontrar o que importa", "Busca, seleção e cópia"),
    ("Ler com conforto", "Temas, páginas e margens"),
    ("Referência sempre à vista", "O modo flutuante"),
    ("Privacidade por padrão", "Nada sai do seu computador"),
    ("Pequenos hábitos de leitura", "Atalhos que viram reflexo"),
]

SUBHEADS = [
    "O custo da espera", "Leve por dentro", "Abrir e começar", "O que acontece no clique",
    "Páginas sob demanda", "O sumário como mapa", "Miniaturas e atalhos visuais",
    "Marcar e voltar", "Buscar no documento inteiro", "Selecionar sem esforço",
    "Copiar o que importa", "Escuro, claro ou sépia", "A página certa para cada tema",
    "Margens e respiro", "Uma janela de referência", "Sempre no topo",
    "Arquivos que ficam com você", "Sem conta, sem rastreio", "Teclas que poupam tempo",
    "Voltar e avançar", "Girar quando preciso", "O ritmo de cada documento",
    "Ler em telas grandes", "Pequenos cuidados",
]

QUOTES = [
    "A melhor interface é aquela que você esquece que está usando.",
    "Ler é conversar com alguém que pensou com calma.",
    "Rapidez não é pressa: é tirar a espera do caminho.",
    "Uma página bem posta convida a continuar.",
]


def paragraph(rng: random.Random, sentences: int) -> Paragraph:
    return Paragraph(" ".join(rng.sample(SENTENCES, sentences)), BODY)


class Document(BaseDocTemplate):
    """Documento com cabeçalho, rodapé e sumário navegável (outline do PDF)."""

    def __init__(self, path: Path, title: str, author: str, running_title: str):
        super().__init__(
            str(path), pagesize=A4, title=title, author=author, subject=running_title,
            creator="MariPDF demo", leftMargin=26 * mm, rightMargin=26 * mm,
            topMargin=26 * mm, bottomMargin=24 * mm,
        )
        self.running_title = running_title
        frame = Frame(self.leftMargin, self.bottomMargin, self.width, self.height, id="f")
        self.addPageTemplates([
            PageTemplate(id="cover", frames=[frame]),
            PageTemplate(id="body", frames=[frame], onPage=self.decorate),
        ])

    def decorate(self, canvas, doc):
        canvas.saveState()
        canvas.setFont("Helvetica", 8.5)
        canvas.setFillColor(MUTED)
        canvas.drawString(self.leftMargin, A4[1] - 15 * mm, self.running_title.upper())
        canvas.drawRightString(A4[0] - self.rightMargin, 13 * mm, str(doc.page))
        canvas.setStrokeColor(colors.HexColor("#E2E0E6"))
        canvas.line(self.leftMargin, A4[1] - 17 * mm, A4[0] - self.rightMargin, A4[1] - 17 * mm)
        canvas.restoreState()

    def afterFlowable(self, flowable):
        # Títulos viram entradas do sumário do PDF (painel Sumário do app).
        if isinstance(flowable, Paragraph) and flowable.style.name in ("h1", "h2"):
            level = 0 if flowable.style.name == "h1" else 1
            key = f"k{id(flowable)}"
            self.canv.bookmarkPage(key)
            self.canv.addOutlineEntry(flowable.getPlainText(), key, level=level, closed=False)


def cover(story, kicker, title, subtitle, note):
    story += [
        Spacer(1, 62 * mm),
        Paragraph(kicker, ParagraphStyle("ck", parent=KICKER, alignment=TA_CENTER, fontSize=10)),
        Spacer(1, 6 * mm),
        Paragraph(title, TITLE),
        Spacer(1, 8 * mm),
        Paragraph(subtitle, SUBTITLE),
        Spacer(1, 70 * mm),
        Paragraph(note, ParagraphStyle("cn", parent=SUBTITLE, fontSize=11, fontName="Helvetica")),
        NextPageTemplate("body"),
        PageBreak(),
    ]


def guide(path: Path):
    rng = random.Random(7)
    doc = Document(path, "Guia do Leitor", "Equipe MariPDF", "Guia do Leitor")
    story = []
    cover(story, "LEITURA", "Guia do Leitor", "Ler rápido, ler bem, ler com calma", "Edição de demonstração")
    story.append(Paragraph("Sumário", H1))
    for number, (name, _) in enumerate(CHAPTERS, 1):
        story.append(Paragraph(f"{number}.&nbsp;&nbsp;{name}", TOC))
    story.append(PageBreak())
    for number, (name, lead) in enumerate(CHAPTERS, 1):
        story += [
            Paragraph(f"CAPÍTULO {number}", KICKER),
            Paragraph(name, H1),
            Paragraph(lead, LEAD),
        ]
        for section in range(3):
            subhead = SUBHEADS[((number - 1) * 3 + section) % len(SUBHEADS)]
            story.append(Paragraph(f"{number}.{section + 1}&nbsp;&nbsp;{subhead}", H2))
            for _ in range(rng.randint(3, 4)):
                story.append(paragraph(rng, rng.randint(4, 6)))
            if section == 1:
                story.append(Paragraph(f"“{QUOTES[number % len(QUOTES)]}”", QUOTE))
        story.append(PageBreak())
    doc.build(story)


def bar_chart(values, labels, title):
    drawing = Drawing(440, 210)
    chart = VerticalBarChart()
    chart.x, chart.y, chart.width, chart.height = 40, 30, 380, 150
    chart.data = [values]
    chart.categoryAxis.categoryNames = labels
    chart.valueAxis.valueMin = 0
    chart.bars[0].fillColor = RED
    chart.bars[0].strokeColor = None
    chart.categoryAxis.labels.fontName = "Helvetica"
    chart.valueAxis.labels.fontName = "Helvetica"
    drawing.add(chart)
    drawing.add(String(40, 195, title, fontName="Helvetica-Bold", fontSize=10, fillColor=INK))
    return drawing


def line_chart(series, labels, title):
    drawing = Drawing(440, 210)
    chart = HorizontalLineChart()
    chart.x, chart.y, chart.width, chart.height = 40, 30, 380, 150
    chart.data = series
    chart.categoryAxis.categoryNames = labels
    chart.lines[0].strokeColor = RED
    chart.lines[0].strokeWidth = 2.2
    chart.lines[1].strokeColor = CORAL
    chart.lines[1].strokeWidth = 2.2
    chart.categoryAxis.labels.fontName = "Helvetica"
    chart.valueAxis.labels.fontName = "Helvetica"
    drawing.add(chart)
    drawing.add(String(40, 195, title, fontName="Helvetica-Bold", fontSize=10, fillColor=INK))
    return drawing


def report(path: Path):
    rng = random.Random(11)
    doc = Document(path, "Relatório Anual 2025", "Estúdio Aurora", "Relatório Anual 2025")
    story = []
    cover(story, "ESTÚDIO AURORA", "Relatório Anual 2025", "Resultados, aprendizados e próximos passos", "Documento fictício de demonstração")
    months = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"]
    story += [
        Paragraph("RESUMO", KICKER),
        Paragraph("Um ano de crescimento consistente", H1),
        Paragraph("Os números abaixo são fictícios e existem apenas para demonstrar a leitura de relatórios com gráficos e tabelas.", LEAD),
        bar_chart([42, 48, 51, 49, 58, 63, 61, 70, 74, 79, 83, 91], months, "Projetos entregues por mês"),
        Spacer(1, 6 * mm),
    ]
    data = [["Indicador", "2024", "2025", "Variação"],
            ["Projetos entregues", "612", "769", "+25,7%"],
            ["Clientes ativos", "148", "187", "+26,4%"],
            ["Satisfação (NPS)", "61", "72", "+11 pts"],
            ["Prazo médio (dias)", "18", "14", "−22,2%"],
            ["Equipe", "34", "41", "+20,6%"]]
    table = Table(data, colWidths=[62 * mm, 30 * mm, 30 * mm, 32 * mm])
    table.setStyle(TableStyle([
        ("FONT", (0, 0), (-1, 0), "Helvetica-Bold", 10),
        ("FONT", (0, 1), (-1, -1), "Helvetica", 10),
        ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
        ("BACKGROUND", (0, 0), (-1, 0), RED),
        ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#F6F3F3")]),
        ("ALIGN", (1, 0), (-1, -1), "RIGHT"),
        ("LINEBELOW", (0, -1), (-1, -1), 0.6, colors.HexColor("#D9D4D4")),
        ("TOPPADDING", (0, 0), (-1, -1), 6),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
    ]))
    story += [table, PageBreak()]
    sections = ["Visão geral", "Pessoas e cultura", "Clientes", "Operação", "Produto", "Próximos passos"]
    for number, name in enumerate(sections, 1):
        story += [Paragraph(f"SEÇÃO {number}", KICKER), Paragraph(name, H1)]
        for _ in range(3):
            story.append(paragraph(rng, rng.randint(4, 6)))
        if number == 3:
            story.append(line_chart([[30, 34, 39, 41, 47, 52], [22, 25, 27, 33, 35, 41]],
                                    ["T1/24", "T2/24", "T3/24", "T4/24", "T1/25", "T2/25"],
                                    "Clientes novos × recorrentes"))
        for _ in range(2):
            story.append(paragraph(rng, rng.randint(4, 6)))
        story.append(PageBreak())
    doc.build(story)


def short_doc(path: Path, title: str, kicker: str, pages: int, seed: int):
    rng = random.Random(seed)
    doc = Document(path, title, "Equipe MariPDF", title)
    story = []
    cover(story, kicker, title, "Documento de demonstração", "MariPDF")
    for page in range(pages):
        story += [Paragraph(f"Parte {page + 1}", H1)]
        for _ in range(5):
            story.append(paragraph(rng, rng.randint(4, 6)))
        story.append(PageBreak())
    doc.build(story)


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "demo")
    out.mkdir(parents=True, exist_ok=True)
    guide(out / "Guia do Leitor.pdf")
    report(out / "Relatório Anual 2025.pdf")
    short_doc(out / "Manual de Estilo Editorial.pdf", "Manual de Estilo Editorial", "EDITORIAL", 6, 3)
    short_doc(out / "Notas de Leitura.pdf", "Notas de Leitura", "CADERNO", 4, 5)
    for pdf in sorted(out.glob("*.pdf")):
        print(f"{pdf.name}: {pdf.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
