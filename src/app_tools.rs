// app_tools.rs — Funções de leitura do dia a dia
//
// Voltar/avançar na navegação, girar a página, recarregar quando o arquivo
// muda no disco, copiar a página como imagem, mostrar na pasta, marcadores do
// usuário, propriedades do documento e abertura de PDF com senha.

use crate::app::{MariApp, Toast};
use crate::app_constants::{BG_SURFACE, TEXT_MUTED, TEXT_PRIMARY};
use crate::pdf::document_worker::{DocumentJobOutput, JobOutcome, JobPriority};
use crate::pdf::engine::DocumentInfo;
use crate::t;
use crate::ui::icons;
use crate::ui::viewer::ViewMode;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ── Histórico de navegação ────────────────────────────────────────────────────

const HISTORY_LIMIT: usize = 64;

/// Pilhas de páginas para Alt+← / Alt+→. Só saltos (link, sumário, busca,
/// ir para página) entram: quem vira página usa `turn_page`, que não registra.
/// A intenção vem de quem chama, não da distância (um salto do sumário para a
/// página vizinha também conta).
#[derive(Default, Debug)]
pub struct NavHistory {
    back: Vec<usize>,
    forward: Vec<usize>,
}

impl NavHistory {
    pub fn record(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        if self.back.last() != Some(&from) {
            self.back.push(from);
            if self.back.len() > HISTORY_LIMIT {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    pub fn back(&mut self, current: usize) -> Option<usize> {
        let target = self.back.pop()?;
        self.forward.push(current);
        Some(target)
    }

    pub fn forward(&mut self, current: usize) -> Option<usize> {
        let target = self.forward.pop()?;
        self.back.push(current);
        Some(target)
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn clear(&mut self) {
        self.back.clear();
        self.forward.clear();
    }
}

// ── Arquivo no disco ──────────────────────────────────────────────────────────

/// Intervalo entre verificações de mudança no arquivo aberto.
const WATCH_INTERVAL_SECS: f64 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl FileStamp {
    pub fn read(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    }
}

/// Estado do monitor: o carimbo do arquivo aberto e uma mudança observada que
/// só vira recarga quando se repete (o gerador pode estar no meio da escrita).
#[derive(Default, Debug)]
pub struct FileWatch {
    pub stamp: Option<FileStamp>,
    pending: Option<FileStamp>,
    last_check: f64,
}

impl FileWatch {
    pub fn reset(&mut self, path: &Path) {
        self.stamp = FileStamp::read(path);
        self.pending = None;
    }

    /// true quando o arquivo mudou e o novo carimbo ficou estável.
    fn changed(&mut self, path: &Path, now: f64) -> bool {
        if now - self.last_check < WATCH_INTERVAL_SECS {
            return false;
        }
        self.last_check = now;
        let Some(current) = FileStamp::read(path) else {
            // Sumiu temporariamente (salvamento atômico); espera voltar.
            self.pending = None;
            return false;
        };
        if Some(current) == self.stamp {
            self.pending = None;
            return false;
        }
        if self.pending == Some(current) {
            // Marca como tratada antes de recarregar: se a abertura falhar
            // (PDF corrompido, senha nova cancelada), só uma nova mudança no
            // disco dispara outra tentativa.
            self.pending = None;
            self.stamp = Some(current);
            return true;
        }
        self.pending = Some(current);
        false
    }
}

/// Escala do render copiado: 2 px por ponto = 144 dpi.
const COPY_SCALE: f32 = 2.0;

// ── Senha ─────────────────────────────────────────────────────────────────────

/// Recarga automática aguardando a abertura feita numa thread.
pub struct PendingReload {
    path: PathBuf,
    password: Option<String>,
    rx: std::sync::mpsc::Receiver<anyhow::Result<crate::pdf::engine::PdfEngine>>,
}

pub struct PasswordPrompt {
    pub path: PathBuf,
    pub input: String,
    /// A senha anterior foi recusada.
    pub wrong: bool,
    /// Pedido nasceu de uma recarga automática: a nova tentativa continua
    /// sendo recarga (mantém modo, painel, histórico e estado ao vivo).
    pub reload: bool,
    focus_requested: bool,
}

impl PasswordPrompt {
    pub fn new(path: PathBuf, wrong: bool, reload: bool) -> Self {
        Self {
            path,
            input: String::new(),
            wrong,
            reload,
            focus_requested: false,
        }
    }
}

// ── Propriedades ──────────────────────────────────────────────────────────────

pub struct DocProperties {
    pub info: DocumentInfo,
    pub file_size: u64,
    pub page_size_pts: (f32, f32),
    /// Página cujo tamanho foi lido (descarta se o usuário já saiu dela).
    pub page: usize,
}

impl MariApp {
    // ── Navegação ────────────────────────────────────────────────────────────

    pub(crate) fn nav_back(&mut self, ctx: &egui::Context) {
        if let Some(page) = self.nav.back(self.current_page) {
            self.go_to_page_untracked(ctx, page);
        }
    }

    pub(crate) fn nav_forward(&mut self, ctx: &egui::Context) {
        if let Some(page) = self.nav.forward(self.current_page) {
            self.go_to_page_untracked(ctx, page);
        }
    }

    // ── Rotação ──────────────────────────────────────────────────────────────

    pub(crate) fn rotate_view(&mut self, ctx: &egui::Context, clockwise: bool) {
        let Some(engine) = self.engine.clone() else {
            return;
        };
        let rotation = if clockwise {
            self.viewer.rotation.clockwise()
        } else {
            self.viewer.rotation.counter_clockwise()
        };
        self.apply_rotation(ctx, &engine, rotation);
        self.save_session();
    }

    pub(crate) fn apply_rotation(
        &mut self,
        ctx: &egui::Context,
        engine: &crate::pdf::engine::PdfEngine,
        rotation: crate::ui::page_rotation::Rotation,
    ) {
        self.viewer.rotation = rotation;
        engine.set_rotation(rotation);
        // Dimensões aprendidas trocam de eixo junto com a rotação.
        self.viewer
            .page_layout
            .set_axes_swapped(rotation.swaps_axes());
        // 4) Uma cópia pedida antes de girar sairia na orientação errada.
        self.page_copy = None;
        self.invalidate_render(ctx);
        if let (Some(worker), Some(_)) = (&self.document_worker, &self.thumb_cache) {
            self.thumb_cache = Some(crate::pdf::thumb_cache::ThumbCache::new(
                self.page_count,
                worker.handle(),
            ));
        }
        if self.viewer.mode == ViewMode::Continuous {
            self.viewer.request_continuous_page(self.current_page);
        }
        ctx.request_repaint();
    }

    // ── Recarregar ───────────────────────────────────────────────────────────

    /// Recarrega o documento quando o arquivo muda no disco (LaTeX,
    /// exportações), mantendo página, zoom e rotação.
    pub(crate) fn poll_file_changes(&mut self, ctx: &egui::Context) {
        self.poll_pending_reload(ctx);
        if self.engine.is_none() || self.password_prompt.is_some() || self.pending_reload.is_some()
        {
            return;
        }
        let Some(path) = self.current_file.clone() else {
            return;
        };
        let now = ctx.input(|i| i.time);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(WATCH_INTERVAL_SECS));
        if self.file_watch.changed(&path, now) {
            tracing::info!(path = %path.display(), "arquivo mudou no disco: recarregando");
            self.start_reload(ctx, path);
        }
    }

    /// Abre a nova versão numa thread: o PDFium é serializado por um mutex
    /// global e um job pesado do worker travaria a janela durante a recarga.
    fn start_reload(&mut self, ctx: &egui::Context, path: PathBuf) {
        let password = self.doc_password.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        let (thread_path, thread_password) = (path.clone(), password.clone());
        let spawned = std::thread::Builder::new()
            .name("maripdf-reload".into())
            .spawn(move || {
                let opened = crate::pdf::engine::get_pdfium().and_then(|pdfium| {
                    crate::pdf::engine::PdfEngine::open_with_password(
                        &thread_path,
                        pdfium,
                        thread_password.as_deref(),
                    )
                });
                let _ = tx.send(opened);
                repaint.request_repaint();
            });
        match spawned {
            Ok(_) => {
                self.pending_reload = Some(PendingReload { path, password, rx });
            }
            Err(error) => tracing::warn!("recarga: {error}"),
        }
    }

    fn poll_pending_reload(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.pending_reload else {
            return;
        };
        let opened = match pending.rx.try_recv() {
            Ok(opened) => opened,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending_reload = None;
                return;
            }
        };
        let Some(pending) = self.pending_reload.take() else {
            return;
        };
        // O usuário pode ter aberto outro arquivo enquanto a recarga rodava.
        if self.current_file.as_deref() != Some(pending.path.as_path()) {
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.install_document(ctx, pending.path, pending.password, true, opened) {
            self.toast = Some(Toast::info(t!("toast-reloaded"), 2.5, now));
        }
    }

    // ── Copiar / pasta ───────────────────────────────────────────────────────

    /// Copia a página atual como imagem (144 dpi, cores originais). O render
    /// vai para o worker; a imagem chega em `poll_page_copy`.
    pub(crate) fn copy_page_image(&mut self, ctx: &egui::Context) {
        let Some(worker) = &self.document_worker else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let page = self.current_page;
        if worker
            .handle()
            .request_render(page, COPY_SCALE, JobPriority::Immediate, tx)
        {
            self.page_copy = Some(rx);
        } else {
            let now = ctx.input(|i| i.time);
            self.toast = Some(Toast::error(t!("toast-page-copy-failed"), 4.0, now));
        }
    }

    /// Entrega a cópia da página quando o render termina.
    pub(crate) fn poll_page_copy(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.page_copy else {
            return;
        };
        let completion = match rx.try_recv() {
            Ok(completion) => completion,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.page_copy = None;
                return;
            }
        };
        self.page_copy = None;
        let now = ctx.input(|i| i.time);
        match completion.outcome {
            JobOutcome::Ready(DocumentJobOutput::Render {
                page_index, page, ..
            }) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [page.width as usize, page.height as usize],
                    &page.rgba,
                );
                ctx.copy_image(image);
                self.toast = Some(Toast::info(
                    t!("toast-page-copied", page => page_index + 1),
                    2.5,
                    now,
                ));
            }
            JobOutcome::Cancelled => {}
            JobOutcome::Failed(error) => {
                tracing::warn!("copiar página: {error}");
                self.toast = Some(Toast::error(t!("toast-page-copy-failed"), 4.0, now));
            }
            JobOutcome::Ready(_) => {
                tracing::warn!("copiar página: resultado de tipo inesperado");
                self.toast = Some(Toast::error(t!("toast-page-copy-failed"), 4.0, now));
            }
        }
    }

    pub(crate) fn show_in_folder(&mut self) {
        let Some(path) = self.current_file.clone() else {
            return;
        };
        if let Err(error) = reveal_in_explorer(&path) {
            tracing::warn!("mostrar na pasta: {error}");
            crate::notices::error(t!("toast-reveal-failed"));
        }
    }

    // ── Marcadores ───────────────────────────────────────────────────────────

    pub(crate) fn is_marked(&self, page: usize) -> bool {
        self.marks.binary_search(&page).is_ok()
    }

    pub(crate) fn toggle_mark(&mut self, ctx: &egui::Context) {
        let page = self.current_page;
        self.toggle_mark_page(ctx, page);
    }

    /// Marca ou desmarca `page` (fita na página, Ctrl+D, menu).
    pub(crate) fn toggle_mark_page(&mut self, ctx: &egui::Context, page: usize) {
        if self.engine.is_none() || page >= self.page_count {
            return;
        }
        let now = ctx.input(|i| i.time);
        let message = match self.marks.binary_search(&page) {
            Ok(index) => {
                self.marks.remove(index);
                t!("toast-mark-removed", page => page + 1)
            }
            Err(index) => {
                self.marks.insert(index, page);
                t!("toast-mark-added", page => page + 1)
            }
        };
        self.toast = Some(Toast::info(message, 2.0, now));
        self.persist_marks();
    }

    fn persist_marks(&mut self) {
        if let Some(path) = self.current_file.clone() {
            self.persistence.set_marks(&path, &self.marks);
        }
    }

    /// Próxima marca depois da página atual (dá a volta no fim).
    pub(crate) fn next_mark(&mut self, ctx: &egui::Context) {
        let current = self.current_page;
        let target = self
            .marks
            .iter()
            .copied()
            .find(|&p| p > current)
            .or_else(|| self.marks.first().copied());
        if let Some(page) = target.filter(|&p| p != current) {
            self.go_to_page(ctx, page);
        }
    }

    /// Marca anterior à página atual (dá a volta no começo).
    pub(crate) fn prev_mark(&mut self, ctx: &egui::Context) {
        let current = self.current_page;
        let target = self
            .marks
            .iter()
            .rev()
            .copied()
            .find(|&p| p < current)
            .or_else(|| self.marks.last().copied());
        if let Some(page) = target.filter(|&p| p != current) {
            self.go_to_page(ctx, page);
        }
    }

    pub(crate) fn remove_mark(&mut self, page: usize) {
        if let Ok(index) = self.marks.binary_search(&page) {
            self.marks.remove(index);
            self.persist_marks();
        }
    }

    /// Lista de marcadores no topo do painel de sumário. Devolve a página
    /// escolhida.
    pub(crate) fn ui_marks_section(
        &mut self,
        ui: &mut egui::Ui,
        text: egui::Color32,
        muted: egui::Color32,
        ink: egui::Color32,
    ) -> Option<usize> {
        if self.marks.is_empty() {
            return None;
        }
        let mut goto = None;
        let mut remove = None;
        ui.label(
            egui::RichText::new(t!("side-marks-title"))
                .size(10.5)
                .strong()
                .color(muted),
        );
        ui.add_space(2.0);
        // Rolagem própria e limitada: muitos marcadores não podem empurrar o
        // sumário (nem os próprios botões de remover) para fora do painel.
        let max_height = (ui.available_height() * 0.35).clamp(60.0, 220.0);
        let marks = self.marks.clone();
        let current_page = self.current_page;
        egui::ScrollArea::vertical()
            .id_salt("side_marks_scroll")
            .max_height(max_height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for &page in &marks {
                    let current = page == current_page;
                    ui.horizontal(|ui| {
                        ui.label(icons::ph(icons::BOOKMARK_SIMPLE, 13.0, ink));
                        let label = egui::RichText::new(t!("side-mark-page", page => page + 1))
                            .size(12.5)
                            .color(if current { ink } else { text });
                        if ui
                            .add(egui::Label::new(label).sense(egui::Sense::click()))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            goto = Some(page);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(icons::ph(icons::X, 11.0, muted))
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::NONE),
                                )
                                .on_hover_text(t!("side-mark-remove"))
                                .clicked()
                            {
                                remove = Some(page);
                            }
                        });
                    });
                }
            });
        if let Some(page) = remove {
            self.remove_mark(page);
        }
        ui.add(egui::Separator::default().spacing(8.0));
        goto
    }

    // ── Propriedades ─────────────────────────────────────────────────────────

    /// Lê os metadados numa thread à parte: o PDFium é serializado por um
    /// mutex global e um job pesado do worker travaria a janela.
    pub(crate) fn open_properties(&mut self, ctx: &egui::Context) {
        let (Some(engine), Some(path)) = (self.engine.clone(), self.current_file.clone()) else {
            return;
        };
        let page = self.current_page;
        let repaint = ctx.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("maripdf-properties".into())
            .spawn(move || {
                let _ = tx.send(DocProperties {
                    info: engine.info(),
                    file_size: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                    page_size_pts: engine.page_size_pts(page),
                    page,
                });
                repaint.request_repaint();
            });
        match spawned {
            Ok(_) => self.properties_rx = Some(rx),
            Err(error) => tracing::warn!("propriedades: {error}"),
        }
    }

    /// Mostra as propriedades quando a leitura termina.
    pub(crate) fn poll_properties(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.properties_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(props) if props.page == self.current_page => {
                self.properties = Some(props);
                self.properties_rx = None;
            }
            // Página mudou durante a leitura: lê de novo para a atual.
            Ok(_) => {
                self.properties_rx = None;
                self.open_properties(ctx);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.properties_rx = None,
        }
    }

    pub(crate) fn ui_properties(&mut self, ctx: &egui::Context) {
        let Some(props) = &self.properties else {
            return;
        };
        let file_name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folder = self
            .current_file
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let (w, h) = props.page_size_pts;
        let rows: Vec<(String, String)> = [
            (t!("prop-file"), file_name),
            (t!("prop-folder"), folder),
            (t!("prop-size"), format_bytes(props.file_size)),
            (t!("prop-pages"), self.page_count.to_string()),
            (
                t!("prop-page-size"),
                format!(
                    "{:.0} × {:.0} mm ({})",
                    w / 72.0 * 25.4,
                    h / 72.0 * 25.4,
                    paper_name(w, h)
                ),
            ),
            (t!("prop-title"), props.info.title.clone()),
            (t!("prop-author"), props.info.author.clone()),
            (t!("prop-subject"), props.info.subject.clone()),
            (t!("prop-keywords"), props.info.keywords.clone()),
            (t!("prop-creator"), props.info.creator.clone()),
            (t!("prop-producer"), props.info.producer.clone()),
            (t!("prop-created"), props.info.created.clone()),
            (t!("prop-modified"), props.info.modified.clone()),
            (t!("prop-version"), props.info.version.clone()),
        ]
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .collect();

        let mut close = false;
        // Modal de verdade: o fundo bloqueia barra, painéis e página até o
        // diálogo fechar (navegar por baixo dele deixaria o conteúdo velho).
        egui::Modal::new(egui::Id::new("document_properties"))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(BG_SURFACE)
                    .inner_margin(egui::Margin::symmetric(22, 18)),
            )
            .show(ctx, |ui| {
                ui.set_width(440.0);
                ui.horizontal(|ui| {
                    ui.label(icons::ph(icons::INFO, 18.0, TEXT_PRIMARY));
                    ui.label(
                        egui::RichText::new(t!("prop-title-window"))
                            .size(15.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                });
                ui.add_space(10.0);
                // Metadados longos rolam; os botões ficam sempre na tela.
                let max_height = (ctx.screen_rect().height() - 180.0).max(120.0);
                egui::ScrollArea::vertical()
                    .id_salt("props_scroll")
                    .max_height(max_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("props_grid")
                            .num_columns(2)
                            .spacing([16.0, 7.0])
                            .show(ui, |ui| {
                                for (label, value) in &rows {
                                    ui.label(
                                        egui::RichText::new(label).size(12.0).color(TEXT_MUTED),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(value)
                                                .size(12.5)
                                                .color(TEXT_PRIMARY),
                                        )
                                        .wrap(),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button(t!("menu-show-in-folder")).clicked() {
                        close = true;
                        self.show_in_folder();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("btn-close")).clicked() {
                            close = true;
                        }
                    });
                });
            });
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.properties = None;
        }
    }

    // ── Senha ────────────────────────────────────────────────────────────────

    pub(crate) fn ui_password_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.password_prompt.as_mut() else {
            return;
        };
        let file_name = prompt
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut submit = false;
        let mut cancel = false;
        // Modal de verdade: o fundo bloqueia barra, painéis e página até o
        // diálogo fechar (navegar por baixo dele deixaria o conteúdo velho).
        egui::Modal::new(egui::Id::new("password_prompt"))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(BG_SURFACE)
                    .inner_margin(egui::Margin::symmetric(22, 18)),
            )
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.horizontal(|ui| {
                    ui.label(icons::ph(icons::LOCK_KEY, 18.0, TEXT_PRIMARY));
                    ui.label(
                        egui::RichText::new(t!("password-title"))
                            .size(15.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(t!("password-desc", file => file_name))
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
                ui.add_space(10.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut prompt.input)
                        .password(true)
                        .hint_text(t!("password-hint"))
                        .desired_width(f32::INFINITY),
                );
                if !prompt.focus_requested {
                    edit.request_focus();
                    prompt.focus_requested = true;
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
                if prompt.wrong {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(t!("password-wrong"))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(239, 90, 80)),
                    );
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("password-open")).clicked() {
                            submit = true;
                        }
                        if ui.button(t!("btn-cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
            });
        if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.password_prompt = None;
        } else if submit {
            if let Some(prompt) = self.password_prompt.take() {
                let password = prompt.input;
                self.open_document(ctx, prompt.path, Some(password), prompt.reload);
            }
        }
    }
}

/// Abre o Explorer com o arquivo selecionado.
fn reveal_in_explorer(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // /select exige o caminho entre aspas logo após a vírgula.
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let folder = path.parent().unwrap_or(path);
        open::that(folder)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB * KB {
        format!("{:.2} GB", b / (KB * KB * KB))
    } else if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Nome do papel padrão mais próximo (tolerância de 2%), ou "personalizado".
fn paper_name(width_pts: f32, height_pts: f32) -> String {
    let (short, long) = if width_pts <= height_pts {
        (width_pts, height_pts)
    } else {
        (height_pts, width_pts)
    };
    let orientation = if width_pts > height_pts {
        t!("prop-landscape")
    } else {
        t!("prop-portrait")
    };
    const PAPERS: [(&str, f32, f32); 6] = [
        ("A3", 841.9, 1190.6),
        ("A4", 595.3, 841.9),
        ("A5", 419.5, 595.3),
        ("Letter", 612.0, 792.0),
        ("Legal", 612.0, 1008.0),
        ("Tabloid", 792.0, 1224.0),
    ];
    let near = |a: f32, b: f32| (a - b).abs() <= b * 0.02;
    match PAPERS
        .iter()
        .find(|(_, s, l)| near(short, *s) && near(long, *l))
    {
        Some((name, _, _)) => format!("{name}, {orientation}"),
        None => t!("prop-custom-size"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_records_only_jumps_and_walks_both_ways() {
        let mut nav = NavHistory::default();
        nav.record(4, 4); // mesma página não entra
        assert!(!nav.can_back());
        nav.record(3, 4); // salto para a vizinha (sumário, ir para) entra
        assert_eq!(nav.back(4), Some(3));
        nav.forward(3);
        nav.record(4, 40); // salto
        nav.record(40, 90);
        assert_eq!(nav.back(90), Some(40));
        assert_eq!(nav.back(40), Some(4));
        assert_eq!(nav.back(4), Some(3));
        assert_eq!(nav.back(3), None);
        assert_eq!(nav.forward(3), Some(4));
        assert_eq!(nav.forward(4), Some(40));
        assert_eq!(nav.forward(40), Some(90));
        // Um salto novo descarta o "avançar".
        nav.back(90);
        nav.record(40, 10);
        assert!(!nav.can_forward());
    }

    #[test]
    fn file_watch_reloads_only_after_a_stable_change() {
        let dir = std::env::temp_dir().join(format!("maripdf-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.pdf");
        std::fs::write(&path, b"v1").unwrap();
        let mut watch = FileWatch::default();
        watch.reset(&path);
        assert!(!watch.changed(&path, 10.0), "sem mudança");
        std::fs::write(&path, b"versao 2").unwrap();
        assert!(!watch.changed(&path, 12.0), "1ª observação só marca");
        assert!(!watch.changed(&path, 12.5), "antes do intervalo não checa");
        assert!(watch.changed(&path, 14.0), "estável: recarrega");
        // Mesmo que a recarga falhe, o mesmo arquivo não é tentado de novo.
        assert!(!watch.changed(&path, 16.0));
        assert!(!watch.changed(&path, 18.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paper_sizes_are_recognized() {
        let _guard = crate::i18n::TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        crate::i18n::init(crate::i18n::Language::PtBr);
        assert!(paper_name(595.0, 842.0).starts_with("A4"));
        assert!(paper_name(792.0, 612.0).starts_with("Letter"));
        assert_eq!(format_bytes(1536), "2 KB");
    }
}
