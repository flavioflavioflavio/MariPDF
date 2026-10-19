// ui/prefetch.rs — Política de overscan orientada ao movimento do viewport.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollDirection {
    Backward,
    Stationary,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PrefetchPlan {
    pub first: usize,
    pub last: usize,
    first_visible: usize,
    last_visible: usize,
    direction: ScrollDirection,
}

impl PrefetchPlan {
    pub fn new(
        first_visible: usize,
        last_visible: usize,
        page_count: usize,
        scroll_delta: f32,
        viewport_height: f32,
    ) -> Self {
        let direction = if scroll_delta > 1.0 {
            ScrollDirection::Forward
        } else if scroll_delta < -1.0 {
            ScrollDirection::Backward
        } else {
            ScrollDirection::Stationary
        };
        let fast = scroll_delta.abs() > viewport_height.max(1.0) * 0.75;
        let favored = if fast { 6 } else { 4 };
        let (behind, ahead) = match direction {
            ScrollDirection::Backward => (favored, 1),
            ScrollDirection::Forward => (1, favored),
            ScrollDirection::Stationary => (1, 3),
        };

        Self {
            first: first_visible.saturating_sub(behind),
            last: (last_visible + ahead).min(page_count.saturating_sub(1)),
            first_visible,
            last_visible,
            direction,
        }
    }

    /// Agenda viewport primeiro; depois o lado favorecido pelo movimento.
    pub fn schedule(self, mut request: impl FnMut(usize)) {
        for page in self.first_visible..=self.last_visible {
            request(page);
        }
        match self.direction {
            ScrollDirection::Backward => {
                for page in (self.first..self.first_visible).rev() {
                    request(page);
                }
                for page in self.last_visible + 1..=self.last {
                    request(page);
                }
            }
            ScrollDirection::Stationary | ScrollDirection::Forward => {
                for page in self.last_visible + 1..=self.last {
                    request(page);
                }
                for page in (self.first..self.first_visible).rev() {
                    request(page);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(plan: PrefetchPlan) -> Vec<usize> {
        let mut pages = Vec::new();
        plan.schedule(|page| pages.push(page));
        pages
    }

    #[test]
    fn forward_scroll_prioritizes_visible_then_next_pages() {
        let plan = PrefetchPlan::new(10, 11, 100, 300.0, 800.0);
        assert_eq!(order(plan), vec![10, 11, 12, 13, 14, 15, 9]);
    }

    #[test]
    fn backward_scroll_prioritizes_previous_pages_in_reverse_order() {
        let plan = PrefetchPlan::new(10, 11, 100, -300.0, 800.0);
        assert_eq!(order(plan), vec![10, 11, 9, 8, 7, 6, 12]);
    }

    #[test]
    fn fast_scroll_expands_overscan_in_movement_direction() {
        let plan = PrefetchPlan::new(10, 11, 100, 700.0, 800.0);
        assert_eq!(plan.first, 9);
        assert_eq!(plan.last, 17);
    }

    #[test]
    fn document_edges_are_clamped() {
        let start = PrefetchPlan::new(0, 0, 3, -900.0, 800.0);
        let end = PrefetchPlan::new(2, 2, 3, 900.0, 800.0);
        assert_eq!(order(start), vec![0, 1]);
        assert_eq!(order(end), vec![2, 1]);
    }
}
