// i18n/mod.rs — Internacionalização via Mozilla Fluent
//
// Usa FluentBundle::new_concurrent() que é Send + Sync via IntlLangMemoizer<RwLock>.
// Bundle global em OnceLock — troca de idioma em runtime com recontrução do bundle.

pub mod loader;

use fluent_bundle::{concurrent::FluentBundle, FluentResource, FluentArgs, FluentValue};
use std::sync::{Mutex, OnceLock};
use unic_langid::langid;

// ── Enum de idioma ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Language {
    PtBr,
    En,
}

impl Default for Language { fn default() -> Self { Self::PtBr } }

impl Language {
    pub fn label(&self) -> &'static str {
        match self { Self::PtBr => "PT-BR", Self::En => "EN" }
    }
    pub fn all() -> [Self; 2] { [Self::PtBr, Self::En] }
}

// ── Bundle global ──────────────────────────────────────────────────────────────

// concurrent::FluentBundle usa IntlLangMemoizer<RwLock> — é Send + Sync.
// Mutex é necessário porque init() re-atribui o conteúdo após a primeira chamada.
static BUNDLE: OnceLock<Mutex<FluentBundle<FluentResource>>> = OnceLock::new();

const FTL_PT_BR: &str = include_str!("../../locales/pt-BR/main.ftl");
const FTL_EN:    &str = include_str!("../../locales/en/main.ftl");

fn build_bundle(lang: Language) -> FluentBundle<FluentResource> {
    let (source, locale) = match lang {
        Language::PtBr => (FTL_PT_BR, langid!("pt-BR")),
        Language::En   => (FTL_EN,    langid!("en-US")),
    };
    let resource = FluentResource::try_new(source.to_owned())
        .expect("FTL inválido — verifique locales/");
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    bundle.add_resource(resource).expect("Falha ao adicionar recurso FTL");
    bundle
}

/// Inicializa ou re-inicializa o bundle (troca de idioma em runtime).
pub fn init(lang: Language) {
    let bundle = build_bundle(lang);
    if let Some(mtx) = BUNDLE.get() {
        *mtx.lock().unwrap() = bundle;
    } else {
        let _ = BUNDLE.set(Mutex::new(bundle));
    }
}

/// Retorna mensagem traduzida sem argumentos.
pub fn get(key: &str) -> String {
    if let Some(mtx) = BUNDLE.get() {
        let b = mtx.lock().unwrap();
        if let Some(msg) = b.get_message(key) {
            if let Some(pat) = msg.value() {
                let mut errs = vec![];
                return b.format_pattern(pat, None, &mut errs).to_string();
            }
        }
    }
    key.to_string()
}

/// Retorna mensagem traduzida com um argumento de string.
pub fn get1(key: &str, arg_name: &str, arg_val: &str) -> String {
    if let Some(mtx) = BUNDLE.get() {
        let b = mtx.lock().unwrap();
        if let Some(msg) = b.get_message(key) {
            if let Some(pat) = msg.value() {
                let mut args = FluentArgs::new();
                args.set(arg_name, FluentValue::from(arg_val.to_owned()));
                let mut errs = vec![];
                return b.format_pattern(pat, Some(&args), &mut errs).to_string();
            }
        }
    }
    key.to_string()
}

// ── Macro de conveniência ──────────────────────────────────────────────────────

/// `t!("chave")` → String traduzida
/// `t!("chave", arg => "valor")` → String traduzida com argumento
#[macro_export]
macro_rules! t {
    ($key:expr) => { crate::i18n::get($key) };
    ($key:expr, $arg:ident => $val:expr) => {
        crate::i18n::get1($key, stringify!($arg), &($val).to_string())
    };
}
