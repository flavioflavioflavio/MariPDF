// toast.rs — Notificações temporárias de UI

use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind { Info, Error }

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub kind:    ToastKind,
    /// Instante em que o toast expira (segundos monotônicos do processo)
    pub expires: f64,
}

impl Toast {
    pub fn info(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Info, expires: now + duration_secs }
    }
    pub fn error(message: impl Into<String>, duration_secs: f64, now: f64) -> Self {
        Self { message: message.into(), kind: ToastKind::Error, expires: now + duration_secs }
    }
    pub fn is_alive(&self, now: f64) -> bool { now < self.expires }
    /// 0.0 = sumindo, 1.0 = totalmente visível
    pub fn alpha(&self, now: f64) -> f32 {
        let remaining = (self.expires - now).max(0.0);
        (remaining / 0.5).min(1.0) as f32
    }
}

/// Renderiza o toast flutuante no canto inferior da tela.
pub fn render_toast(ctx: &egui::Context, toast: &Toast) {
    let now    = ctx.input(|i| i.time);
    let alpha  = toast.alpha(now);
    let screen = ctx.screen_rect();

    let (bg, fg) = match toast.kind {
        ToastKind::Info  => (
            egui::Color32::from_rgba_unmultiplied(30, 30, 40, (230.0 * alpha) as u8),
            egui::Color32::from_rgba_unmultiplied(240, 240, 255, (240.0 * alpha) as u8),
        ),
        ToastKind::Error => (
            egui::Color32::from_rgba_unmultiplied(80, 20, 20, (230.0 * alpha) as u8),
            egui::Color32::from_rgba_unmultiplied(255, 180, 180, (240.0 * alpha) as u8),
        ),
    };

    let icon = match toast.kind {
        ToastKind::Info  => "ℹ",
        ToastKind::Error => "✖",
    };

    egui::Area::new(egui::Id::new("toast_area"))
        .fixed_pos(egui::pos2(screen.center().x - 200.0, screen.bottom() - 70.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(bg)
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(400.0);
                    ui.set_max_width(400.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).size(14.0).color(fg));
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(&toast.message).size(13.0).color(fg));
                    });
                });
        });
}
