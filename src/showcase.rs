// showcase.rs — Capturas de tela do fluxo real para docs e README (BF-706).
//
// Ativado com MARIPDF_SHOWCASE=<pasta> e MARIPDF_SHOWCASE_FILE=<pdf>.
// O app percorre um roteiro fixo e fotografa a PRÓPRIA janela via
// ViewportCommand::Screenshot (nunca a tela do usuário). Também grava uma
// sequência de quadros de navegação para montar um GIF curto.

use crate::app::MariApp;
use crate::app_reader::SidePanelKind;
use crate::reader_profile::{DarkPage, PageChoice};
use crate::ui::viewer::ViewMode;
use eframe::egui;
use std::path::PathBuf;
use std::time::Instant;

/// Tempo para a UI assentar (render, fade da toolbar) antes de cada foto.
const SETTLE_SECS: f64 = 1.2;
const GIF_FRAMES: usize = 36;
const GIF_FRAME_SECS: f64 = 0.08;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shot {
    Home,
    Reader,
    Search,
    Continuous,
    Settings,
    About,
    Float,
    MoreMenu,
    Rotated,
    Marks,
    Properties,
    PageWarm,
    PageDeep,
    PageNight,
    Appearance,
    Password,
    Gif(usize),
    Done,
}

impl Shot {
    fn file_name(self) -> String {
        match self {
            Self::Gif(i) => format!("gif/frame_{i:03}.png"),
            other => format!("{}.png", format!("{other:?}").to_lowercase()),
        }
    }
}

pub struct ShowcaseRun {
    dir: PathBuf,
    file: PathBuf,
    shot: Shot,
    stage_started: Instant,
    requested: bool,
}

impl ShowcaseRun {
    pub fn from_env() -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("MARIPDF_SHOWCASE")?);
        let file = PathBuf::from(std::env::var_os("MARIPDF_SHOWCASE_FILE")?);
        let _ = std::fs::create_dir_all(dir.join("gif"));
        Some(Self {
            dir,
            file,
            shot: Shot::Home,
            stage_started: Instant::now(),
            requested: false,
        })
    }

    fn settled(&self, secs: f64) -> bool {
        self.stage_started.elapsed().as_secs_f64() >= secs
    }

    fn advance(&mut self, next: Shot) {
        self.shot = next;
        self.stage_started = Instant::now();
        self.requested = false;
    }
}

impl MariApp {
    pub(crate) fn showcase_step(&mut self, ctx: &egui::Context) {
        let Some(mut run) = self.showcase.take() else {
            return;
        };
        ctx.request_repaint();

        // Foto pedida no frame anterior chegou?
        let captured = ctx.input(|i| {
            i.raw.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = captured {
            save_png(&run.dir.join(run.shot.file_name()), &image);
            let next = self.prepare_next(ctx, run.shot, &run.file);
            run.advance(next);
        } else if run.shot == Shot::Done {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !run.requested
            && run.settled(self.settle_for(run.shot))
            && self.ready_for(run.shot)
        {
            run.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        self.showcase = Some(run);
    }

    fn settle_for(&self, shot: Shot) -> f64 {
        match shot {
            Shot::Gif(_) => GIF_FRAME_SECS,
            _ => SETTLE_SECS,
        }
    }

    fn ready_for(&self, shot: Shot) -> bool {
        let page_ready = self.viewer.texture.is_some()
            || self.viewer.mode == ViewMode::Continuous
            || shot == Shot::Home;
        match shot {
            Shot::Home => true,
            Shot::Search => !self.search.hits.is_empty() && page_ready,
            _ => page_ready && self.engine.is_some(),
        }
    }

    /// Prepara a tela da próxima foto e devolve qual é.
    fn prepare_next(&mut self, ctx: &egui::Context, done: Shot, file: &std::path::Path) -> Shot {
        match done {
            Shot::Home => {
                self.pending_open = Some(file.to_path_buf());
                Shot::Reader
            }
            Shot::Reader => {
                self.search_open = true;
                self.search.query = "MariPDF".into();
                self.search.start("MariPDF");
                Shot::Search
            }
            Shot::Search => {
                self.search_open = false;
                self.search.clear();
                self.do_toggle_mode(ctx);
                Shot::Continuous
            }
            Shot::Continuous => {
                self.settings_open = true;
                Shot::Settings
            }
            Shot::Settings => {
                crate::ui::settings_panel::select_about_tab(ctx);
                Shot::About
            }
            Shot::About => {
                self.settings_open = false;
                self.enter_frame_mode(ctx);
                Shot::Float
            }
            Shot::Float => {
                self.exit_frame_mode(ctx);
                ctx.memory_mut(|m| m.open_popup(crate::app_toolbar::more_menu_id()));
                Shot::MoreMenu
            }
            Shot::MoreMenu => {
                ctx.memory_mut(|m| m.close_popup());
                self.rotate_view(ctx, true);
                Shot::Rotated
            }
            Shot::Rotated => {
                self.rotate_view(ctx, false);
                self.toggle_mark(ctx);
                self.side_panel = SidePanelKind::Toc;
                Shot::Marks
            }
            Shot::Marks => {
                self.side_panel = SidePanelKind::None;
                self.open_properties(ctx);
                Shot::Properties
            }
            Shot::Properties => {
                self.properties = None;
                self.set_page(ctx, PageChoice::Dark(DarkPage::WarmNight));
                Shot::PageWarm
            }
            Shot::PageWarm => {
                self.set_page(ctx, PageChoice::Dark(DarkPage::Deep));
                Shot::PageDeep
            }
            Shot::PageDeep => {
                self.set_page(ctx, PageChoice::Dark(DarkPage::Night));
                Shot::PageNight
            }
            Shot::PageNight => {
                self.settings_open = true;
                crate::ui::settings_panel::select_appearance_tab(ctx);
                Shot::Appearance
            }
            Shot::Appearance => {
                self.settings_open = false;
                // PDF com senha opcional: MARIPDF_SHOWCASE_LOCKED=<pdf>.
                match std::env::var_os("MARIPDF_SHOWCASE_LOCKED") {
                    Some(locked) => {
                        self.open_document(ctx, locked.into(), None, false);
                        Shot::Password
                    }
                    None => self.start_gif(ctx),
                }
            }
            Shot::Password => {
                self.password_prompt = None;
                self.start_gif(ctx)
            }
            Shot::Gif(i) if i + 1 < GIF_FRAMES => {
                // Rolagem contínua suave por algumas páginas.
                self.viewer.smooth_scroll = true;
                if i % 6 == 0 {
                    let page = (i / 6 + 1).min(self.page_count.saturating_sub(1));
                    self.viewer.request_continuous_page(page);
                }
                Shot::Gif(i + 1)
            }
            Shot::Gif(_) | Shot::Done => Shot::Done,
        }
    }
}

impl MariApp {
    fn set_page(&mut self, ctx: &egui::Context, choice: PageChoice) {
        choice.apply(&mut self.settings.page_styles);
        self.invalidate_render(ctx);
    }

    fn start_gif(&mut self, ctx: &egui::Context) -> Shot {
        if self.viewer.mode == ViewMode::SinglePage {
            self.do_toggle_mode(ctx);
        }
        Shot::Gif(0)
    }
}

fn save_png(path: &std::path::Path, image: &egui::ColorImage) {
    let [w, h] = image.size;
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    match image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
        Some(buffer) => {
            if let Err(error) = buffer.save(path) {
                tracing::warn!("showcase: falha ao salvar {}: {error}", path.display());
            }
        }
        None => tracing::warn!("showcase: buffer inválido para {}", path.display()),
    }
}
