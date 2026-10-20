// i18n/mod.rs — Internacionalização via Mozilla Fluent
//
// Usa FluentBundle::new_concurrent() que é Send + Sync via IntlLangMemoizer<RwLock>.
// Bundle global em OnceLock — troca de idioma em runtime com recontrução do bundle.

use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource, FluentValue};
use std::sync::{Mutex, OnceLock};
use unic_langid::langid;

// ── Enum de idioma ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
pub enum Language {
    #[default]
    PtBr,
    En,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PtBr => "PT-BR",
            Self::En => "EN",
        }
    }
    pub fn all() -> [Self; 2] {
        [Self::PtBr, Self::En]
    }
}

// ── Bundle global ──────────────────────────────────────────────────────────────

// concurrent::FluentBundle usa IntlLangMemoizer<RwLock> — é Send + Sync.
// Mutex é necessário porque init() re-atribui o conteúdo após a primeira chamada.
static BUNDLE: OnceLock<Mutex<FluentBundle<FluentResource>>> = OnceLock::new();

/// Testes que trocam o idioma global seguram este lock para não se atropelarem.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

const FTL_PT_BR: &str = include_str!("../../locales/pt-BR/main.ftl");
const FTL_EN: &str = include_str!("../../locales/en/main.ftl");

fn build_bundle(lang: Language) -> FluentBundle<FluentResource> {
    let (source, locale) = match lang {
        Language::PtBr => (FTL_PT_BR, langid!("pt-BR")),
        Language::En => (FTL_EN, langid!("en-US")),
    };
    let resource =
        FluentResource::try_new(source.to_owned()).expect("FTL inválido — verifique locales/");
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    // Sem FSI/PDI ao redor dos argumentos: o egui os desenharia como caixas.
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .expect("Falha ao adicionar recurso FTL");
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
    get_args(key, &[(arg_name, arg_val.to_owned())])
}

/// Retorna mensagem traduzida com vários argumentos de string.
pub fn get_args(key: &str, values: &[(&str, String)]) -> String {
    if let Some(mtx) = BUNDLE.get() {
        let b = mtx.lock().unwrap();
        if let Some(msg) = b.get_message(key) {
            if let Some(pat) = msg.value() {
                let mut args = FluentArgs::new();
                for (name, value) in values {
                    args.set(*name, FluentValue::from(value.clone()));
                }
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
    ($key:expr) => {
        $crate::i18n::get($key)
    };
    ($key:expr, $arg:ident => $val:expr) => {
        $crate::i18n::get1($key, stringify!($arg), &($val).to_string())
    };
    ($key:expr, $($arg:ident => $val:expr),+ $(,)?) => {
        $crate::i18n::get_args($key, &[$((stringify!($arg), ($val).to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::{FTL_EN, FTL_PT_BR};
    use std::collections::BTreeSet;

    fn keys(ftl: &str) -> BTreeSet<&str> {
        ftl.lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_lowercase()))
            .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim()))
            .collect()
    }

    #[test]
    fn locales_expose_the_same_keys() {
        let pt = keys(FTL_PT_BR);
        let en = keys(FTL_EN);
        let only_pt: Vec<_> = pt.difference(&en).collect();
        let only_en: Vec<_> = en.difference(&pt).collect();
        assert!(
            only_pt.is_empty() && only_en.is_empty(),
            "missing in EN: {only_pt:?}; missing in PT-BR: {only_en:?}"
        );
    }

    #[test]
    fn every_key_used_in_code_exists() {
        let defined = keys(FTL_PT_BR);
        let mut missing = Vec::new();
        // O próprio módulo só cita a macro em exemplos de documentação.
        let sources = rust_sources()
            .into_iter()
            .filter(|(path, _)| !path.replace('\\', "/").ends_with("i18n/mod.rs"));
        for (path, source) in sources {
            for key in literal_keys(&source) {
                if !defined.contains(key.as_str()) {
                    missing.push(format!("{path}: {key}"));
                }
            }
        }
        assert!(missing.is_empty(), "chaves sem tradução: {missing:#?}");
    }

    /// Texto visível deve passar pelo Fluent; esta guarda pega regressões
    /// óbvias (literais com palavras em widgets, tooltips e toasts).
    #[test]
    fn ui_text_is_not_hardcoded() {
        const SINKS: [&str; 7] = [
            "RichText::new(\"",
            "on_hover_text(\"",
            "Toast::info(\"",
            "Toast::error(\"",
            "hint_text(\"",
            "set_title(\"",
            "ui.label(\"",
        ];
        const ALLOWED: [&str; 1] = ["MariPDF"];
        let mut offenders = Vec::new();
        for (path, source) in rust_sources() {
            for (number, line) in source.lines().enumerate() {
                for sink in SINKS {
                    let Some(start) = line.find(sink) else {
                        continue;
                    };
                    let rest = &line[start + sink.len()..];
                    let literal = rest.split('"').next().unwrap_or_default();
                    let has_word = literal
                        .split(|c: char| !c.is_alphabetic())
                        .any(|word| word.chars().count() >= 3);
                    if has_word && !ALLOWED.contains(&literal) {
                        offenders.push(format!("{path}:{}: {literal}", number + 1));
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "texto fixo na UI: {offenders:#?}");
    }

    fn rust_sources() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).expect("ler src/").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let source = std::fs::read_to_string(&path).expect("ler fonte");
                    out.push((path.display().to_string(), source));
                }
            }
        }
        let mut out = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut out,
        );
        out
    }

    /// Chaves literais em `t!("...")`.
    fn literal_keys(source: &str) -> Vec<String> {
        source
            .match_indices("t!(\"")
            .filter_map(|(index, mat)| {
                let rest = &source[index + mat.len()..];
                let key = rest.split('"').next()?;
                key.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                    .then(|| key.to_string())
            })
            .collect()
    }
}
