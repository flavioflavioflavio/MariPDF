// app_window.rs — Restaura e acompanha a posição da janela (BF-412).
//
// O eframe cria a janela oculta e só a mostra depois do primeiro frame; o
// placement salvo é aplicado nesse primeiro `update`, então a janela já aparece
// no lugar certo. Depois, mudanças são gravadas quando ficam estáveis por um
// instante (sobrevive a crash) e uma última vez ao sair.

use crate::pdf::persistence::PersistenceManager;
use crate::platform::window_placement::{self, Placement};
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Intervalo mínimo entre leituras do placement.
const POLL_SECS: f64 = 0.25;
/// Tempo sem mudanças antes de gravar.
const SETTLE_SECS: f64 = 1.0;
/// Tamanho mínimo da janela normal, em pontos.
const MIN_SIZE_POINTS: egui::Vec2 = egui::vec2(640.0, 480.0);

#[derive(Default)]
pub struct WindowTracker {
    hwnd: Option<isize>,
    restored: bool,
    last_saved: Option<Placement>,
    pending: Option<(Placement, f64)>,
    last_poll: f64,
}

impl WindowTracker {
    /// Chamado a cada frame. `track` é falso no modo Flutuante e em tela cheia,
    /// estados que não devem virar a janela "normal" da próxima sessão.
    pub fn on_frame(&mut self, ctx: &egui::Context, frame: &eframe::Frame, track: bool) {
        if self.hwnd.is_none() {
            self.hwnd = native_hwnd(frame);
        }
        let Some(hwnd) = self.hwnd else {
            return;
        };
        if !self.restored {
            self.restored = true;
            self.restore(ctx, hwnd);
            return;
        }
        if !track {
            self.pending = None;
            return;
        }

        let now = ctx.input(|i| i.time);
        if now - self.last_poll >= POLL_SECS {
            self.last_poll = now;
            if let Some(current) = window_placement::read(hwnd) {
                self.observe(current, now);
            }
        }
        // Garante um frame quando a mudança pendente amadurecer, mesmo com o
        // app ocioso; nunca antes do próximo poll permitido.
        if let Some((_, since)) = self.pending {
            let wait = (SETTLE_SECS - (now - since)).max(POLL_SECS) + 0.02;
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
    }

    fn observe(&mut self, current: Placement, now: f64) {
        if Some(current) == self.last_saved {
            self.pending = None;
            return;
        }
        match self.pending {
            Some((pending, since)) if pending == current => {
                if now - since >= SETTLE_SECS {
                    self.save(current);
                }
            }
            _ => self.pending = Some((current, now)),
        }
    }

    /// Grava o estado final ao fechar o app.
    pub fn on_exit(&mut self, track: bool) {
        if !track {
            return;
        }
        if let Some(current) = self.hwnd.and_then(window_placement::read) {
            if Some(current) != self.last_saved {
                self.save(current);
            }
        }
    }

    fn restore(&mut self, ctx: &egui::Context, hwnd: isize) {
        let Some(saved) = PersistenceManager::load_window().placement else {
            return;
        };
        let scale = ctx
            .input(|i| i.viewport().native_pixels_per_point)
            .unwrap_or(1.0);
        let min = (
            (MIN_SIZE_POINTS.x * scale).round() as i32,
            (MIN_SIZE_POINTS.y * scale).round() as i32,
        );
        let normal = window_placement::resolve(saved.normal, &window_placement::work_areas(), min);
        if !window_placement::apply_normal(hwnd, normal) {
            tracing::warn!("janela: não foi possível aplicar a posição salva");
            return;
        }
        if saved.maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        if normal != saved.normal {
            tracing::info!(
                ?saved,
                ?normal,
                "janela: posição salva ajustada aos monitores atuais"
            );
        }
        self.last_saved = Some(Placement {
            normal,
            maximized: saved.maximized,
        });
    }

    fn save(&mut self, placement: Placement) {
        match PersistenceManager::save_window(placement) {
            Ok(()) => {
                self.last_saved = Some(placement);
                self.pending = None;
            }
            Err(error) => tracing::warn!("janela: falha ao salvar posição: {error}"),
        }
    }
}

fn native_hwnd(frame: &eframe::Frame) -> Option<isize> {
    match frame.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}
