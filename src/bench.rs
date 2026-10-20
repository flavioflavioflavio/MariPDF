// bench.rs — Medidor de produto e smoke test embutidos (BF-002 / BF-007).
//
// Ativado com `MARIPDF_BENCH=<relatório.toml>` e um PDF na linha de comando.
// O app executa um roteiro fixo sobre o documento, mede cada etapa e o custo
// de cada frame da UI, grava o relatório e fecha. Os scripts em tools/
// sempre usam `MARIPDF_DATA_DIR` para não tocar no estado do usuário.

use crate::app::MariApp;
use crate::ui::viewer::ViewMode;
use eframe::egui;
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Instant;

/// Tempo máximo por etapa antes de registrá-la como falha.
const STEP_TIMEOUT_SECS: f64 = 30.0;
/// Marca no relatório uma abertura recusada e tratada (não um timeout).
const OPEN_REFUSED: &str = "abertura recusada";
/// Saltos sequenciais no modo contínuo para medir custo de frame.
const CONTINUOUS_JUMPS: usize = 40;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

/// Marca o início do processo; chamado no topo de `main`.
pub fn mark_process_start() {
    let _ = PROCESS_START.set(Instant::now());
}

fn since_start_ms() -> f64 {
    PROCESS_START
        .get()
        .map_or(0.0, |start| start.elapsed().as_secs_f64() * 1000.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    WaitDocument,
    FirstPage,
    FarPage,
    NeighborPage,
    Zoom,
    Search,
    Continuous,
    Done,
}

#[derive(Debug, Default, Serialize)]
pub struct StepResult {
    pub name: String,
    pub ok: bool,
    pub ms: f64,
    pub detail: String,
}

#[derive(Debug, Default, Serialize)]
pub struct FrameStats {
    pub frames: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    pub over_50ms: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct BenchReport {
    pub file: String,
    pub pages: usize,
    pub ok: bool,
    pub startup_to_first_frame_ms: f64,
    pub open_ms: f64,
    pub startup_to_first_page_ms: f64,
    pub working_set_mb: f64,
    pub peak_working_set_mb: f64,
    pub steps: Vec<StepResult>,
    pub ui_frames: FrameStats,
    pub continuous_frames: FrameStats,
}

pub struct BenchRun {
    output: std::path::PathBuf,
    step: Step,
    step_started: Instant,
    target_page: usize,
    jumps_left: usize,
    report: BenchReport,
    frame_ms: Vec<f64>,
    continuous_ms: Vec<f64>,
    first_frame_seen: bool,
    written: bool,
    open_failure: Option<String>,
}

impl BenchRun {
    /// Liga o modo bench se `MARIPDF_BENCH` estiver definido.
    pub fn from_env() -> Option<Self> {
        let output = std::env::var_os("MARIPDF_BENCH")?;
        tracing::info!("bench: modo medição ativo → {}", output.to_string_lossy());
        Some(Self {
            output: output.into(),
            step: Step::WaitDocument,
            step_started: Instant::now(),
            target_page: 0,
            jumps_left: CONTINUOUS_JUMPS,
            report: BenchReport::default(),
            frame_ms: Vec::new(),
            continuous_ms: Vec::new(),
            first_frame_seen: false,
            written: false,
            open_failure: None,
        })
    }

    pub fn record_open(&mut self, open_ms: f64) {
        self.report.open_ms = open_ms;
    }

    /// Abertura recusada e já mostrada ao usuário (senha, corrompido...).
    pub fn record_open_failure(&mut self, message: String) {
        self.open_failure = Some(message);
    }

    fn finish_step(&mut self, ok: bool, detail: impl Into<String>, next: Step) {
        let result = StepResult {
            name: format!("{:?}", self.step),
            ok,
            ms: self.step_started.elapsed().as_secs_f64() * 1000.0,
            detail: detail.into(),
        };
        tracing::info!(step = %result.name, ok, ms = result.ms, "bench: etapa concluída");
        self.report.steps.push(result);
        self.step = next;
        self.step_started = Instant::now();
    }

    fn timed_out(&self) -> bool {
        self.step_started.elapsed().as_secs_f64() > STEP_TIMEOUT_SECS
    }
}

impl MariApp {
    /// Avança o roteiro do bench; chamado ao fim de cada `update`.
    pub(crate) fn bench_step(&mut self, ctx: &egui::Context, frame_ms: f64) {
        let Some(mut run) = self.bench.take() else {
            return;
        };
        if !run.first_frame_seen {
            run.first_frame_seen = true;
            run.report.startup_to_first_frame_ms = since_start_ms();
        }
        run.frame_ms.push(frame_ms);
        if run.step == Step::Continuous {
            run.continuous_ms.push(frame_ms);
        }
        ctx.request_repaint();

        let page_ready = |app: &MariApp, page: usize| {
            app.viewer.loaded_page == Some(page) && app.viewer.texture.is_some()
        };

        match run.step {
            Step::WaitDocument => {
                if self.engine.is_some() {
                    run.report.pages = self.page_count;
                    run.report.file = self
                        .current_file
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    run.finish_step(true, "documento aberto", Step::FirstPage);
                } else if let Some(message) = run.open_failure.take() {
                    // Prefixo que o bench_report.py exige para "recusa esperada".
                    run.finish_step(false, format!("{OPEN_REFUSED}: {message}"), Step::Done);
                } else if self.render_error.is_some() || run.timed_out() {
                    let detail = self
                        .render_error
                        .clone()
                        .unwrap_or_else(|| "timeout".into());
                    run.finish_step(false, detail, Step::Done);
                }
            }
            Step::FirstPage => {
                if page_ready(self, 0) {
                    run.report.startup_to_first_page_ms = since_start_ms();
                    run.target_page = (self.page_count / 2).max(1).min(self.page_count - 1);
                    let target = run.target_page;
                    run.finish_step(true, "página 1 apresentada", Step::FarPage);
                    self.go_to_page(ctx, target);
                } else if run.timed_out() {
                    run.finish_step(false, "timeout", Step::Done);
                }
            }
            Step::FarPage => {
                if self.page_count < 2 || page_ready(self, run.target_page) {
                    run.target_page = (run.target_page + 1).min(self.page_count - 1);
                    let target = run.target_page;
                    run.finish_step(true, "salto distante, sem cache", Step::NeighborPage);
                    self.go_to_page(ctx, target);
                } else if run.timed_out() {
                    run.finish_step(false, "timeout", Step::Done);
                }
            }
            Step::NeighborPage => {
                if page_ready(self, run.target_page) {
                    run.finish_step(true, "vizinha, com prefetch", Step::Zoom);
                    self.set_zoom(ctx, 1.6);
                } else if run.timed_out() {
                    run.finish_step(false, "timeout", Step::Done);
                }
            }
            Step::Zoom => {
                if page_ready(self, self.current_page) {
                    run.finish_step(true, "zoom 160%", Step::Search);
                    self.search_open = true;
                    self.search.start("MariPDF");
                } else if run.timed_out() {
                    run.finish_step(false, "timeout", Step::Done);
                }
            }
            Step::Search => {
                // Zero ocorrências é válido (PDF só de imagens); falha de
                // extração não: reprova em vez de virar "0 ocorrência(s)".
                if self.search.failed_pages > 0 {
                    let detail = format!(
                        "{} página(s) com falha na extração",
                        self.search.failed_pages
                    );
                    run.finish_step(false, detail, Step::Done);
                } else if !self.search.hits.is_empty() || !self.search.is_running {
                    let detail = format!("{} ocorrência(s)", self.search.hits.len());
                    run.finish_step(true, detail, Step::Continuous);
                    self.search_open = false;
                    self.search.clear();
                    self.set_zoom(ctx, 1.0);
                    if self.viewer.mode == ViewMode::SinglePage {
                        self.do_toggle_mode(ctx);
                    }
                    self.viewer.smooth_scroll = false;
                } else if run.timed_out() {
                    run.finish_step(false, "timeout", Step::Done);
                }
            }
            Step::Continuous => {
                // Cada salto só conta quando a página pedida virou a dominante
                // e tem textura na GPU; erro de render ou timeout reprova.
                let target = run.target_page;
                let landed = run.jumps_left == CONTINUOUS_JUMPS
                    || (self.viewer.dominant_continuous_page() == Some(target)
                        && self.viewer.tex_cache.contains(target));
                if let Some(error) = self.viewer.page_errors.get(&target).cloned() {
                    run.finish_step(false, format!("página {}: {error}", target + 1), Step::Done);
                } else if run.timed_out() {
                    let detail = format!(
                        "timeout no salto {} (página {})",
                        CONTINUOUS_JUMPS - run.jumps_left,
                        target + 1
                    );
                    run.finish_step(false, detail, Step::Done);
                } else if landed {
                    if run.jumps_left == 0 {
                        run.finish_step(
                            true,
                            format!("{CONTINUOUS_JUMPS} saltos renderizados"),
                            Step::Done,
                        );
                    } else {
                        run.jumps_left -= 1;
                        run.target_page =
                            (CONTINUOUS_JUMPS - run.jumps_left) * 7 % self.page_count.max(1);
                        self.viewer.request_continuous_page(run.target_page);
                    }
                }
            }
            Step::Done => {
                if !run.written {
                    run.written = true;
                    write_report(&mut run);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.bench = Some(run);
    }
}

fn stats(samples: &[f64]) -> FrameStats {
    if samples.is_empty() {
        return FrameStats::default();
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let pick = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    FrameStats {
        frames: sorted.len(),
        p50_ms: pick(0.5),
        p95_ms: pick(0.95),
        max_ms: *sorted.last().unwrap(),
        over_50ms: sorted.iter().filter(|ms| **ms > 50.0).count(),
    }
}

fn write_report(run: &mut BenchRun) {
    run.report.ok = run.report.steps.iter().all(|s| s.ok) && !run.report.steps.is_empty();
    run.report.ui_frames = stats(&run.frame_ms);
    run.report.continuous_frames = stats(&run.continuous_ms);
    if let Some(memory) = crate::platform::process_memory::current() {
        run.report.working_set_mb = memory.working_set as f64 / 1_048_576.0;
        run.report.peak_working_set_mb = memory.peak_working_set as f64 / 1_048_576.0;
    }
    match toml::to_string_pretty(&run.report) {
        Ok(text) => {
            if let Err(error) = crate::storage::write_atomic(&run.output, text.as_bytes()) {
                tracing::error!("bench: falha ao gravar relatório: {error}");
            }
        }
        Err(error) => tracing::error!("bench: falha ao serializar relatório: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_stats_pick_percentiles() {
        let samples: Vec<f64> = (1..=100).map(f64::from).collect();
        let s = stats(&samples);
        assert_eq!(s.frames, 100);
        assert_eq!(s.p50_ms, 51.0);
        assert_eq!(s.p95_ms, 95.0);
        assert_eq!(s.max_ms, 100.0);
        assert_eq!(s.over_50ms, 50);
        assert_eq!(stats(&[]).frames, 0);
    }
}
