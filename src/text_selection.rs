// text_selection.rs — Regras puras de seleção de texto (BF-411).
//
// Opera sobre os chars com geometria extraídos do PDFium (coordenadas PDF,
// origem no canto inferior esquerdo). Mantém o contrato fora da UI para que
// palavras, linhas e o texto copiado sejam testáveis sem janela.

use crate::pdf::engine::TextCharInfo;

/// Fração da altura da linha a partir da qual um vão horizontal vira espaço.
const WORD_GAP_RATIO: f32 = 0.25;

/// Índices inclusivos em ordem, qualquer que seja a direção do arraste.
pub fn ordered(anchor: usize, focus: usize) -> (usize, usize) {
    (anchor.min(focus), anchor.max(focus))
}

fn height(ch: &TextCharInfo) -> f32 {
    (ch.top - ch.bottom).max(1.0)
}

fn center_y(ch: &TextCharInfo) -> f32 {
    (ch.top + ch.bottom) * 0.5
}

fn is_break(ch: &TextCharInfo) -> bool {
    matches!(ch.ch, '\n' | '\r')
}

/// Dois chars estão na mesma linha quando os centros verticais ficam a menos
/// de meia altura de linha um do outro.
pub fn same_line(a: &TextCharInfo, b: &TextCharInfo) -> bool {
    (center_y(a) - center_y(b)).abs() < height(a).max(height(b)) * 0.5
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'' | '’' | '-')
}

/// Palavra que contém `index` (inclusiva). Fora de palavra, só o próprio char.
pub fn word_range(chars: &[TextCharInfo], index: usize) -> (usize, usize) {
    let Some(origin) = chars.get(index) else {
        return (index, index);
    };
    if !is_word_char(origin.ch) {
        return (index, index);
    }
    // Vizinhos só se juntam se forem de palavra, na mesma linha e sem vão.
    let joins = |left: &TextCharInfo, right: &TextCharInfo| {
        is_word_char(left.ch)
            && is_word_char(right.ch)
            && same_line(left, right)
            && !has_word_gap(left, right)
    };
    let mut lo = index;
    while lo > 0 && joins(&chars[lo - 1], &chars[lo]) {
        lo -= 1;
    }
    let mut hi = index;
    while hi + 1 < chars.len() && joins(&chars[hi], &chars[hi + 1]) {
        hi += 1;
    }
    (lo, hi)
}

fn has_word_gap(left: &TextCharInfo, right: &TextCharInfo) -> bool {
    right.left - left.right > height(left).max(height(right)) * WORD_GAP_RATIO
}

/// Linha visual que contém `index` (inclusiva), sem as quebras.
pub fn line_range(chars: &[TextCharInfo], index: usize) -> (usize, usize) {
    let Some(origin) = chars.get(index) else {
        return (index, index);
    };
    let joins = |ch: &TextCharInfo| !is_break(ch) && same_line(ch, origin);
    let mut lo = index;
    while lo > 0 && joins(&chars[lo - 1]) {
        lo -= 1;
    }
    let mut hi = index;
    while hi + 1 < chars.len() && joins(&chars[hi + 1]) {
        hi += 1;
    }
    (lo, hi)
}

fn push_newline(out: &mut String) {
    let trimmed = out.trim_end_matches([' ', '\t']).len();
    out.truncate(trimmed);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

/// Texto copiado para o intervalo inclusivo `lo..=hi`: espaço onde há vão
/// entre palavras, quebra de linha onde a linha visual muda e sem `\r`.
pub fn selection_text(chars: &[TextCharInfo], lo: usize, hi: usize) -> String {
    if chars.is_empty() || lo > hi {
        return String::new();
    }
    let hi = hi.min(chars.len() - 1);
    let mut out = String::new();
    let mut previous: Option<&TextCharInfo> = None;

    for ch in &chars[lo..=hi] {
        if is_break(ch) {
            if ch.ch == '\n' {
                push_newline(&mut out);
            }
            previous = None;
            continue;
        }
        if let Some(prev) = previous {
            if !same_line(prev, ch) {
                push_newline(&mut out);
            } else if has_word_gap(prev, ch)
                && !ch.ch.is_whitespace()
                && !out.ends_with(char::is_whitespace)
            {
                out.push(' ');
            }
        }
        out.push(ch.ch);
        previous = Some(ch);
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Linhas de 10pt de altura; cada char tem 6pt de largura. `|` marca um
    /// vão de palavra (sem char de espaço, como o PDFium costuma entregar).
    fn layout(lines: &[&str]) -> Vec<TextCharInfo> {
        let mut chars = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let bottom = 700.0 - row as f32 * 14.0;
            let mut x = 72.0;
            for c in line.chars() {
                if c == '|' {
                    x += 4.0;
                    continue;
                }
                chars.push(TextCharInfo {
                    ch: c,
                    left: x,
                    bottom,
                    right: x + 6.0,
                    top: bottom + 10.0,
                });
                x += 6.0;
            }
        }
        chars
    }

    #[test]
    fn gaps_become_spaces_and_lines_become_newlines() {
        let chars = layout(&["Leitor|de|PDF", "rápido"]);
        let text = selection_text(&chars, 0, chars.len() - 1);
        assert_eq!(text, "Leitor de PDF\nrápido");
    }

    #[test]
    fn explicit_line_breaks_are_normalized() {
        let mut chars = layout(&["ab"]);
        let mut cr = chars[1].clone();
        cr.ch = '\r';
        let mut lf = chars[1].clone();
        lf.ch = '\n';
        let mut c = chars[1].clone();
        c.ch = 'c';
        c.left += 6.0;
        c.right += 6.0;
        chars.extend([cr, lf, c]);
        assert_eq!(selection_text(&chars, 0, chars.len() - 1), "ab\nc");
    }

    #[test]
    fn existing_spaces_are_not_doubled() {
        let chars = layout(&["a| |b"]);
        assert_eq!(selection_text(&chars, 0, chars.len() - 1), "a b");
    }

    #[test]
    fn word_range_stops_at_gaps_punctuation_and_line_end() {
        let chars = layout(&["olá,|mundo", "novo"]);
        // "mundo" começa no índice 4.
        assert_eq!(word_range(&chars, 6), (4, 8));
        assert_eq!(word_range(&chars, 1), (0, 2));
        // Vírgula não pertence a palavra.
        assert_eq!(word_range(&chars, 3), (3, 3));
        // "mundo" e "novo" não se juntam através da quebra de linha.
        assert_eq!(word_range(&chars, 9), (9, 12));
    }

    #[test]
    fn word_range_respects_visual_gaps_without_space_chars() {
        let chars = layout(&["Leitor|de|PDF"]);
        assert_eq!(word_range(&chars, 2), (0, 5));
        assert_eq!(word_range(&chars, 6), (6, 7));
        assert_eq!(word_range(&chars, 9), (8, 10));
    }

    #[test]
    fn line_range_covers_the_visual_line() {
        let chars = layout(&["uma|linha", "outra"]);
        assert_eq!(line_range(&chars, 2), (0, 7));
        assert_eq!(line_range(&chars, 9), (8, 12));
    }

    #[test]
    fn ordered_handles_backwards_drags() {
        assert_eq!(ordered(9, 3), (3, 9));
        assert_eq!(ordered(3, 9), (3, 9));
    }

    #[test]
    fn out_of_range_requests_are_safe() {
        let chars = layout(&["ab"]);
        assert_eq!(selection_text(&chars, 0, 99), "ab");
        assert_eq!(selection_text(&chars, 2, 1), "");
        assert_eq!(selection_text(&[], 0, 0), "");
        assert_eq!(word_range(&chars, 99), (99, 99));
    }
}
