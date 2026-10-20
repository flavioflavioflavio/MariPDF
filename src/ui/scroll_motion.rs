// ui/scroll_motion.rs — Contrato do "Scroll Suave" no modo contínuo (BF-404).
//
// Ligado: a roda usa a suavização do egui e saltos programáticos (ir para
// página, setas, busca, miniaturas) animam com ease-out.
// Desligado: a roda aplica o delta bruto no mesmo frame e saltos são
// instantâneos.

/// Duração da animação de salto.
pub(crate) const JUMP_SECS: f64 = 0.25;

/// Animação de um salto de rolagem entre dois offsets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollTween {
    from: f32,
    to: f32,
    started: f64,
}

impl ScrollTween {
    pub(crate) fn new(from: f32, to: f32, now: f64) -> Self {
        Self {
            from,
            to,
            started: now,
        }
    }

    /// Mantém o andamento e troca o destino: usado quando o layout muda no
    /// meio do salto (páginas que terminam de renderizar mudam de altura).
    pub(crate) fn retarget(&mut self, to: f32) {
        self.to = to;
    }

    /// Offset no instante `now` e se a animação terminou.
    pub(crate) fn sample(&self, now: f64) -> (f32, bool) {
        let t = ((now - self.started) / JUMP_SECS).clamp(0.0, 1.0) as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        (self.from + (self.to - self.from) * eased, t >= 1.0)
    }
}

/// Destino de uma navegação programática, guardado para ser recalculado se
/// o layout mudar antes de a animação terminar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NavGoal {
    Page(usize),
    Anchor(crate::ui::page_layout::LayoutAnchor),
}

/// Offset após um passo de roda sem suavização (delta positivo sobe a página).
pub(crate) fn instant_wheel_offset(current: f32, raw_delta_y: f32) -> f32 {
    (current - raw_delta_y).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_starts_at_origin_and_lands_on_target() {
        let tween = ScrollTween::new(100.0, 900.0, 10.0);
        assert_eq!(tween.sample(10.0), (100.0, false));
        assert_eq!(tween.sample(10.0 + JUMP_SECS), (900.0, true));
        assert_eq!(tween.sample(99.0), (900.0, true));
    }

    #[test]
    fn tween_eases_out_monotonically() {
        let tween = ScrollTween::new(0.0, 1000.0, 0.0);
        let samples: Vec<f32> = (0..=10)
            .map(|i| tween.sample(JUMP_SECS * f64::from(i) / 10.0).0)
            .collect();
        assert!(samples.windows(2).all(|w| w[0] <= w[1]));
        // Ease-out: mais da metade do caminho já na primeira metade do tempo.
        assert!(samples[5] > 500.0);
    }

    #[test]
    fn tween_works_upwards() {
        let tween = ScrollTween::new(800.0, 200.0, 0.0);
        let (mid, done) = tween.sample(JUMP_SECS / 2.0);
        assert!(mid < 800.0 && mid > 200.0 && !done);
    }

    #[test]
    fn retarget_keeps_progress_and_lands_on_the_new_target() {
        let mut tween = ScrollTween::new(0.0, 1000.0, 0.0);
        let (before, _) = tween.sample(JUMP_SECS / 2.0);
        tween.retarget(1200.0);
        let (after, _) = tween.sample(JUMP_SECS / 2.0);
        assert!(after > before, "mesmo andamento, destino mais distante");
        assert_eq!(tween.sample(JUMP_SECS), (1200.0, true));
    }

    #[test]
    fn instant_wheel_moves_by_the_raw_delta_and_never_above_top() {
        assert_eq!(instant_wheel_offset(500.0, -120.0), 620.0);
        assert_eq!(instant_wheel_offset(500.0, 120.0), 380.0);
        assert_eq!(instant_wheel_offset(50.0, 120.0), 0.0);
    }
}
