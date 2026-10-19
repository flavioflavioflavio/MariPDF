// platform/window_placement.rs — Posição/tamanho/maximização da janela (BF-412).
//
// Usa GetWindowPlacement/SetWindowPlacement: o Windows já guarda o retângulo
// "normal" de uma janela maximizada e trabalha em pixels físicos, evitando
// confusão de DPI entre monitores. Coordenadas são de workspace (origem no
// canto da área útil do monitor principal), as mesmas do placement.
//
// A decisão de onde reabrir é pura (`resolve`) e testada sem janela.

use serde::{Deserialize, Serialize};

/// Retângulo em pixels físicos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PxRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PxRect {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    fn overlap_area(&self, other: &PxRect) -> i64 {
        let w = (self.right.min(other.right) - self.left.max(other.left)).max(0);
        let h = (self.bottom.min(other.bottom) - self.top.max(other.top)).max(0);
        i64::from(w) * i64::from(h)
    }
}

/// Estado persistido da janela.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    /// Retângulo da janela quando não está maximizada.
    pub normal: PxRect,
    pub maximized: bool,
}

/// Faixa superior (barra de título) que precisa continuar alcançável.
const TITLE_BAND: i32 = 32;
/// Largura mínima dessa faixa visível para a janela ainda ser arrastável.
const MIN_VISIBLE_TITLE: i32 = 96;

fn title_visible(window: &PxRect, work: &PxRect) -> bool {
    let visible_width = window.right.min(work.right) - window.left.max(work.left);
    window.top >= work.top
        && window.top + TITLE_BAND <= work.bottom
        && visible_width >= MIN_VISIBLE_TITLE
}

/// Decide onde reabrir a janela dada a topologia atual de monitores.
///
/// `work_areas` são as áreas úteis (sem taskbar) de cada monitor, com o
/// principal primeiro. A janela termina inteira dentro de um monitor: no que
/// mais a contém, se a barra de título ainda está visível nele; senão
/// centralizada no principal. O tamanho é limitado à área útil e ao mínimo.
pub fn resolve(saved: PxRect, work_areas: &[PxRect], min_size: (i32, i32)) -> PxRect {
    let Some(primary) = work_areas.first() else {
        return saved;
    };
    let reachable = work_areas
        .iter()
        .filter(|work| title_visible(&saved, work))
        .max_by_key(|work| saved.overlap_area(work));
    let target = reachable.unwrap_or(primary);

    let fit = |wanted: i32, min: i32, available: i32| wanted.max(min.min(available)).min(available);
    let width = fit(saved.width(), min_size.0, target.width());
    let height = fit(saved.height(), min_size.1, target.height());

    let (left, top) = if reachable.is_some() {
        (
            saved.left.clamp(target.left, target.right - width),
            saved.top.clamp(target.top, target.bottom - height),
        )
    } else {
        (
            target.left + (target.width() - width) / 2,
            target.top + (target.height() - height) / 2,
        )
    };
    PxRect::new(left, top, width, height)
}

// ── Win32 ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win32 {
    use super::{Placement, PxRect};

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct WindowPlacement {
        length: u32,
        flags: u32,
        show_cmd: u32,
        min_position: Point,
        max_position: Point,
        normal_position: Rect,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfo {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }

    type MonitorEnumProc = unsafe extern "system" fn(isize, isize, *mut Rect, isize) -> i32;

    #[link(name = "user32")]
    extern "system" {
        fn GetWindowPlacement(hwnd: isize, placement: *mut WindowPlacement) -> i32;
        fn SetWindowPlacement(hwnd: isize, placement: *const WindowPlacement) -> i32;
        fn EnumDisplayMonitors(
            hdc: isize,
            clip: *const Rect,
            callback: MonitorEnumProc,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }

    const SW_HIDE: u32 = 0;
    const SW_SHOWMINIMIZED: u32 = 2;
    const SW_SHOWMAXIMIZED: u32 = 3;
    const WPF_RESTORETOMAXIMIZED: u32 = 2;
    const MONITORINFOF_PRIMARY: u32 = 1;

    impl From<Rect> for PxRect {
        fn from(r: Rect) -> Self {
            PxRect {
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
            }
        }
    }

    pub fn read(hwnd: isize) -> Option<Placement> {
        let mut placement = WindowPlacement {
            length: std::mem::size_of::<WindowPlacement>() as u32,
            ..Default::default()
        };
        // SAFETY: hwnd vem do eframe e a struct tem o layout de WINDOWPLACEMENT.
        if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
            return None;
        }
        let maximized = placement.show_cmd == SW_SHOWMAXIMIZED
            || (placement.show_cmd == SW_SHOWMINIMIZED
                && placement.flags & WPF_RESTORETOMAXIMIZED != 0);
        Some(Placement {
            normal: placement.normal_position.into(),
            maximized,
        })
    }

    /// Define o retângulo normal sem mostrar a janela (o eframe a exibe
    /// depois do primeiro frame).
    pub fn apply_normal(hwnd: isize, rect: PxRect) -> bool {
        let mut placement = WindowPlacement {
            length: std::mem::size_of::<WindowPlacement>() as u32,
            ..Default::default()
        };
        // SAFETY: idem `read`.
        if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
            return false;
        }
        placement.flags = 0;
        placement.show_cmd = SW_HIDE;
        placement.normal_position = Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };
        // SAFETY: idem `read`.
        unsafe { SetWindowPlacement(hwnd, &placement) != 0 }
    }

    unsafe extern "system" fn collect(monitor: isize, _: isize, _: *mut Rect, data: isize) -> i32 {
        let out = &mut *(data as *mut Vec<MonitorInfo>);
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            out.push(info);
        }
        1
    }

    /// Áreas úteis dos monitores em coordenadas de workspace, principal primeiro.
    pub fn work_areas() -> Vec<PxRect> {
        let mut monitors: Vec<MonitorInfo> = Vec::new();
        // SAFETY: o callback só escreve no Vec apontado por `data` durante a chamada.
        unsafe {
            EnumDisplayMonitors(
                0,
                std::ptr::null(),
                collect,
                &mut monitors as *mut _ as isize,
            );
        }
        monitors.sort_by_key(|m| m.flags & MONITORINFOF_PRIMARY == 0);
        let Some(primary) = monitors.first().copied() else {
            return Vec::new();
        };
        // Workspace: origem no canto da área útil do monitor principal.
        let (dx, dy) = (
            primary.work.left - primary.monitor.left,
            primary.work.top - primary.monitor.top,
        );
        monitors
            .iter()
            .map(|m| PxRect {
                left: m.work.left - dx,
                top: m.work.top - dy,
                right: m.work.right - dx,
                bottom: m.work.bottom - dy,
            })
            .collect()
    }
}

#[cfg(target_os = "windows")]
pub use win32::{apply_normal, read, work_areas};

#[cfg(not(target_os = "windows"))]
pub fn read(_hwnd: isize) -> Option<Placement> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_normal(_hwnd: isize, _rect: PxRect) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn work_areas() -> Vec<PxRect> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: (i32, i32) = (800, 600);

    fn primary() -> PxRect {
        PxRect::new(0, 0, 1920, 1040)
    }

    #[test]
    fn visible_window_is_kept_as_is() {
        let saved = PxRect::new(100, 80, 1200, 900);
        assert_eq!(resolve(saved, &[primary()], MIN), saved);
    }

    #[test]
    fn window_on_removed_monitor_is_centered_on_primary() {
        // Estava num monitor à direita que foi desconectado.
        let saved = PxRect::new(2200, 100, 1000, 700);
        let resolved = resolve(saved, &[primary()], MIN);
        assert_eq!(resolved, PxRect::new(460, 170, 1000, 700));
    }

    #[test]
    fn left_monitor_with_negative_coordinates_is_valid() {
        let left = PxRect::new(-2560, 0, 2560, 1400);
        let saved = PxRect::new(-2000, 50, 1200, 900);
        assert_eq!(resolve(saved, &[primary(), left], MIN), saved);
    }

    #[test]
    fn partially_offscreen_window_is_pulled_inside() {
        let saved = PxRect::new(1500, 300, 1000, 900);
        let resolved = resolve(saved, &[primary()], MIN);
        assert_eq!(resolved, PxRect::new(920, 140, 1000, 900));
    }

    #[test]
    fn title_bar_above_the_screen_is_recovered() {
        let saved = PxRect::new(200, -400, 1000, 900);
        let resolved = resolve(saved, &[primary()], MIN);
        assert!(resolved.top >= 0);
        assert_eq!(resolved.width(), 1000);
    }

    #[test]
    fn size_is_clamped_to_work_area_and_minimum() {
        let small_screen = PxRect::new(0, 0, 1366, 728);
        let huge = PxRect::new(0, 0, 3000, 2000);
        assert_eq!(resolve(huge, &[small_screen], MIN), small_screen);

        let tiny = PxRect::new(10, 10, 100, 100);
        let resolved = resolve(tiny, &[primary()], MIN);
        assert_eq!((resolved.width(), resolved.height()), MIN);
    }

    #[test]
    fn prefers_the_monitor_that_holds_most_of_the_window() {
        let right = PxRect::new(1920, 0, 1920, 1040);
        let saved = PxRect::new(1800, 100, 1000, 700);
        let resolved = resolve(saved, &[primary(), right], MIN);
        assert!(resolved.left >= right.left, "{resolved:?}");
    }

    #[test]
    fn no_monitor_information_keeps_the_saved_rect() {
        let saved = PxRect::new(5, 5, 900, 700);
        assert_eq!(resolve(saved, &[], MIN), saved);
    }

    #[test]
    fn placement_round_trips_through_toml() {
        let placement = Placement {
            normal: PxRect::new(-1200, 40, 1100, 800),
            maximized: true,
        };
        let text = toml::to_string(&placement).unwrap();
        assert_eq!(toml::from_str::<Placement>(&text).unwrap(), placement);
    }
}
