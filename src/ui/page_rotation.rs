// ui/page_rotation.rs — Rotação de visualização da página
//
// A rotação é só de exibição (o arquivo não muda) e fica salva por documento.
// O PDFium já entrega o bitmap girado; aqui mora o único mapeamento entre
// coordenadas do PDF (pontos, y para cima, página sem rotação) e a tela, usado
// por seleção de texto, links e destaque da busca.

use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Rotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    pub fn clockwise(self) -> Self {
        match self {
            Self::R0 => Self::R90,
            Self::R90 => Self::R180,
            Self::R180 => Self::R270,
            Self::R270 => Self::R0,
        }
    }

    pub fn counter_clockwise(self) -> Self {
        match self {
            Self::R0 => Self::R270,
            Self::R90 => Self::R0,
            Self::R180 => Self::R90,
            Self::R270 => Self::R180,
        }
    }

    /// 90° e 270° trocam largura e altura.
    pub fn swaps_axes(self) -> bool {
        matches!(self, Self::R90 | Self::R270)
    }

    pub fn quarter_turns(self) -> u8 {
        match self {
            Self::R0 => 0,
            Self::R90 => 1,
            Self::R180 => 2,
            Self::R270 => 3,
        }
    }

    pub fn from_quarter_turns(turns: u8) -> Self {
        match turns % 4 {
            0 => Self::R0,
            1 => Self::R90,
            2 => Self::R180,
            _ => Self::R270,
        }
    }

    /// (largura, altura) exibidas de uma página com o tamanho `size`.
    pub fn display_size(self, (width, height): (f32, f32)) -> (f32, f32) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// Coordenada normalizada da página sem rotação (u à direita, v para
    /// baixo) → coordenada normalizada na página girada no sentido horário.
    fn rotate_unit(self, (u, v): (f32, f32)) -> (f32, f32) {
        match self {
            Self::R0 => (u, v),
            Self::R90 => (1.0 - v, u),
            Self::R180 => (1.0 - u, 1.0 - v),
            Self::R270 => (v, 1.0 - u),
        }
    }

    fn unrotate_unit(self, (u, v): (f32, f32)) -> (f32, f32) {
        match self {
            Self::R0 => (u, v),
            Self::R90 => (v, 1.0 - u),
            Self::R180 => (1.0 - u, 1.0 - v),
            Self::R270 => (1.0 - v, u),
        }
    }
}

/// Onde e como uma página está desenhada na tela.
#[derive(Clone, Copy, Debug)]
pub struct PageMapping {
    /// Retângulo da página na tela (já com as dimensões giradas).
    pub rect: egui::Rect,
    /// Tamanho da página em pontos PDF, sem rotação.
    pub pdf_size: (f32, f32),
    pub rotation: Rotation,
}

impl PageMapping {
    pub fn new(rect: egui::Rect, pdf_size: (f32, f32), rotation: Rotation) -> Self {
        Self {
            rect,
            pdf_size,
            rotation,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.rect.is_positive() && self.pdf_size.0 > 0.0 && self.pdf_size.1 > 0.0
    }

    /// Ponto do PDF (x à direita, y para cima) → tela.
    pub fn screen_point(&self, x: f32, y: f32) -> egui::Pos2 {
        let (w, h) = self.pdf_size;
        let (u, v) = self.rotation.rotate_unit((x / w, 1.0 - y / h));
        self.rect.min + egui::vec2(u * self.rect.width(), v * self.rect.height())
    }

    /// Tela → ponto do PDF.
    pub fn pdf_point(&self, pos: egui::Pos2) -> (f32, f32) {
        let (w, h) = self.pdf_size;
        let u = (pos.x - self.rect.left()) / self.rect.width().max(1.0);
        let v = (pos.y - self.rect.top()) / self.rect.height().max(1.0);
        let (u, v) = self.rotation.unrotate_unit((u, v));
        (u * w, (1.0 - v) * h)
    }

    /// `[left, bottom, right, top]` em pontos PDF → retângulo na tela.
    pub fn rect_to_screen(&self, [left, bottom, right, top]: [f32; 4]) -> egui::Rect {
        egui::Rect::from_two_pos(
            self.screen_point(left, top),
            self.screen_point(right, bottom),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(rotation: Rotation) -> PageMapping {
        // Página 100×200 pt desenhada em escala 2.
        let (w, h) = rotation.display_size((200.0, 400.0));
        PageMapping::new(
            egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(w, h)),
            (100.0, 200.0),
            rotation,
        )
    }

    #[test]
    fn screen_and_pdf_round_trip_in_every_rotation() {
        for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
            let m = mapping(rotation);
            for (x, y) in [(0.0, 0.0), (25.0, 150.0), (100.0, 200.0), (60.0, 10.0)] {
                let (bx, by) = m.pdf_point(m.screen_point(x, y));
                assert!(
                    (bx - x).abs() < 1e-3 && (by - y).abs() < 1e-3,
                    "{rotation:?} {x},{y}"
                );
            }
        }
    }

    #[test]
    fn top_left_of_the_page_follows_a_clockwise_turn() {
        // Canto superior esquerdo do PDF = (0, altura).
        assert_eq!(
            mapping(Rotation::R0).screen_point(0.0, 200.0),
            egui::pos2(10.0, 20.0)
        );
        // Girando 90° no sentido horário ele vai para o canto superior direito.
        let r90 = mapping(Rotation::R90);
        assert_eq!(r90.screen_point(0.0, 200.0), egui::pos2(10.0 + 400.0, 20.0));
        assert_eq!(r90.rect.size(), egui::vec2(400.0, 200.0));
    }

    #[test]
    fn rect_mapping_matches_the_unrotated_formula() {
        let m = mapping(Rotation::R0);
        let r = m.rect_to_screen([10.0, 150.0, 30.0, 180.0]);
        let close = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-3;
        assert!(close(r.min, egui::pos2(10.0 + 20.0, 20.0 + 40.0)), "{r:?}");
        assert!(close(r.max, egui::pos2(10.0 + 60.0, 20.0 + 100.0)), "{r:?}");
    }

    #[test]
    fn turns_cycle() {
        assert_eq!(Rotation::R270.clockwise(), Rotation::R0);
        assert_eq!(Rotation::R0.counter_clockwise(), Rotation::R270);
        assert_eq!(
            Rotation::from_quarter_turns(Rotation::R180.quarter_turns()),
            Rotation::R180
        );
    }
}
