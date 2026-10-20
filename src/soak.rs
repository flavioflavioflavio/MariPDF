// soak.rs — Teste de longa duração dentro do app real (BF-708).
//
// Ativado com MARIPDF_SOAK_SECS=<duração> e MARIPDF_SOAK_FILES=<pdf;pdf;...>.
// Executa, em ordem pseudoaleatória reproduzível, o que um usuário faz:
// navegar, saltar, dar zoom, alternar modo, buscar e trocar de documento.
// Amostra a memória a cada poucos segundos e grava um relatório TOML em
// MARIPDF_SOAK_OUT, incluindo a tendência de crescimento (vazamento).

use crate::app::MariApp;
use crate::ui::viewer::ViewMode;
use eframe::egui;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

const ACTION_INTERVAL_SECS: f64 = 0.35;
const SAMPLE_INTERVAL_SECS: f64 = 5.0;
const SWITCH_DOCUMENT_SECS: f64 = 30.0;
/// Crescimento estável aceito do working set (caches limitados por bytes
/// devem estabilizar; acima disso é tratado como vazamento).
pub(crate) const MAX_GROWTH_MB_PER_HOUR: f64 = 50.0;

#[derive(Debug, Default, Serialize)]
pub struct SoakReport {
    pub duration_secs: f64,
    pub actions: u64,
    pub documents_opened: u64,
    pub open_failures: u64,
    pub render_errors: u64,
    pub samples_mb: Vec<f64>,
    pub first_quarter_mb: f64,
    pub last_quarter_mb: f64,
    pub peak_mb: f64,
    pub growth_mb_per_hour: f64,
    pub ok: bool,
}

pub struct SoakRun {
    files: Vec<PathBuf>,
    output: PathBuf,
    duration: f64,
    started: Instant,
    last_action: f64,
    last_sample: f64,
    last_switch: f64,
    next_file: usize,
    rng: u64,
    report: SoakReport,
    finished: bool,
}

impl SoakRun {
    pub fn from_env() -> Option<Self> {
        let duration: f64 = std::env::var("MARIPDF_SOAK_SECS").ok()?.parse().ok()?;
        let files: Vec<PathBuf> = std::env::var("MARIPDF_SOAK_FILES")
            .ok()?
            .split(';')
            .filter(|p| !p.trim().is_empty())
            .map(PathBuf::from)
            .collect();
        let output = std::env::var_os("MARIPDF_SOAK_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::storage::local_dir().join("soak.toml"));
        if files.is_empty() {
            return None;
        }
        tracing::info!(duration, files = files.len(), "soak: iniciado");
        Some(Self {
            files,
            output,
            duration,
            started: Instant::now(),
            last_action: 0.0,
            last_sample: 0.0,
            last_switch: 0.0,
            next_file: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
            report: SoakReport::default(),
            finished: false,
        })
    }

    /// xorshift64*: determinístico, sem dependências.
    fn next(&mut self, bound: u64) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound.max(1)
    }

    pub(crate) fn record_open(&mut self, opened: bool) {
        if opened {
            self.report.documents_opened += 1;
        } else {
            self.report.open_failures += 1;
        }
    }

    fn elapsed(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl MariApp {
    pub(crate) fn soak_step(&mut self, ctx: &egui::Context) {
        let Some(mut run) = self.soak.take() else {
            return;
        };
        ctx.request_repaint();
        let now = run.elapsed();

        if !run.finished {
            if now - run.last_sample >= SAMPLE_INTERVAL_SECS || run.report.samples_mb.is_empty() {
                run.last_sample = now;
                if let Some(memory) = crate::platform::process_memory::current() {
                    run.report
                        .samples_mb
                        .push(memory.working_set as f64 / 1_048_576.0);
                    run.report.peak_mb = memory.peak_working_set as f64 / 1_048_576.0;
                }
            }

            if self.engine.is_none() || now - run.last_switch >= SWITCH_DOCUMENT_SECS {
                run.last_switch = now;
                let path = run.files[run.next_file % run.files.len()].clone();
                run.next_file += 1;
                // documents_opened/open_failures são contados em open_file,
                // depois do resultado real da abertura.
                self.pending_open = Some(path);
            } else if now - run.last_action >= ACTION_INTERVAL_SECS {
                run.last_action = now;
                run.report.actions += 1;
                if self.render_error.take().is_some() {
                    run.report.render_errors += 1;
                }
                // No modo contínuo a falha fica na página, não em render_error.
                run.report.render_errors += std::mem::take(&mut self.viewer.continuous_failures);
                let pages = self.page_count.max(1) as u64;
                match run.next(9) {
                    0..=2 => {
                        let page = (self.current_page + 1).min(self.page_count.saturating_sub(1));
                        self.go_to_page(ctx, page);
                    }
                    3 => {
                        let page = run.next(pages) as usize;
                        self.go_to_page(ctx, page);
                    }
                    4 => {
                        let zoom = 0.5 + run.next(30) as f32 / 10.0;
                        self.set_zoom(ctx, zoom);
                    }
                    5 => self.do_toggle_mode(ctx),
                    6 => {
                        self.search_open = true;
                        let query = ["MariPDF", "pagina", "ação", "zzz"][run.next(4) as usize];
                        self.search.start(query);
                    }
                    7 => {
                        self.search_open = false;
                        self.search.clear();
                    }
                    _ => {
                        if self.viewer.mode == ViewMode::Continuous {
                            let page = run.next(pages) as usize;
                            self.viewer.request_continuous_page(page);
                        }
                    }
                }
            }

            if now >= run.duration {
                run.finished = true;
                finish(&mut run);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.soak = Some(run);
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Inclinação (MB/h) da regressão linear das amostras igualmente espaçadas.
pub(crate) fn growth_per_hour(samples: &[f64], interval_secs: f64) -> f64 {
    let n = samples.len() as f64;
    if samples.len() < 2 {
        return 0.0;
    }
    let mean_x = (n - 1.0) / 2.0;
    let mean_y = mean(samples);
    let (num, den) = samples
        .iter()
        .enumerate()
        .fold((0.0, 0.0), |(num, den), (i, y)| {
            let dx = i as f64 - mean_x;
            (num + dx * (y - mean_y), den + dx * dx)
        });
    if den == 0.0 {
        0.0
    } else {
        num / den * (3600.0 / interval_secs)
    }
}

/// Passa só se exercitou o app de verdade, não houve falha de render e a
/// memória não cresce continuamente.
pub(crate) fn soak_passed(report: &SoakReport) -> bool {
    report.documents_opened > 1
        && report.open_failures == 0
        && report.actions > 0
        && report.render_errors == 0
        && report.growth_mb_per_hour <= MAX_GROWTH_MB_PER_HOUR
}

fn finish(run: &mut SoakRun) {
    let report = &mut run.report;
    report.duration_secs = run.started.elapsed().as_secs_f64();
    // Descarta o 1º quarto (aquecimento de caches) da comparação final.
    let quarter = (report.samples_mb.len() / 4).max(1);
    report.first_quarter_mb = mean(&report.samples_mb[..quarter.min(report.samples_mb.len())]);
    report.last_quarter_mb =
        mean(&report.samples_mb[report.samples_mb.len().saturating_sub(quarter)..]);
    let steady = &report.samples_mb[quarter.min(report.samples_mb.len())..];
    report.growth_mb_per_hour = growth_per_hour(steady, SAMPLE_INTERVAL_SECS);
    report.ok = soak_passed(report);
    match toml::to_string_pretty(&*report) {
        Ok(text) => {
            if let Err(error) = crate::storage::write_atomic(&run.output, text.as_bytes()) {
                tracing::error!("soak: falha ao gravar relatório: {error}");
            }
        }
        Err(error) => tracing::error!("soak: falha ao serializar: {error}"),
    }
    tracing::info!(
        actions = report.actions,
        docs = report.documents_opened,
        growth = report.growth_mb_per_hour,
        "soak: concluído"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soak_fails_on_render_errors_or_memory_growth() {
        let healthy = SoakReport {
            documents_opened: 8,
            actions: 3000,
            growth_mb_per_hour: 3.0,
            ..Default::default()
        };
        assert!(soak_passed(&healthy));
        assert!(!soak_passed(&SoakReport {
            render_errors: 1,
            ..healthy_clone(&healthy)
        }));
        assert!(!soak_passed(&SoakReport {
            growth_mb_per_hour: 400.0,
            ..healthy_clone(&healthy)
        }));
        assert!(!soak_passed(&SoakReport::default()));
    }

    fn healthy_clone(r: &SoakReport) -> SoakReport {
        SoakReport {
            documents_opened: r.documents_opened,
            actions: r.actions,
            growth_mb_per_hour: r.growth_mb_per_hour,
            ..Default::default()
        }
    }

    #[test]
    fn flat_memory_has_no_growth_and_a_ramp_is_detected() {
        assert_eq!(growth_per_hour(&[200.0; 20], 5.0), 0.0);
        // +1 MB a cada 5 s = 720 MB/h.
        let ramp: Vec<f64> = (0..20).map(|i| 200.0 + i as f64).collect();
        assert!((growth_per_hour(&ramp, 5.0) - 720.0).abs() < 1e-6);
        assert_eq!(growth_per_hour(&[1.0], 5.0), 0.0);
    }

    #[test]
    fn failed_opens_fail_the_soak() {
        let report = SoakReport {
            documents_opened: 8,
            open_failures: 1,
            actions: 3000,
            growth_mb_per_hour: 3.0,
            ..Default::default()
        };
        assert!(!soak_passed(&report));
    }
}
