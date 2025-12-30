use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, HeaderMap},
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use cookie::Cookie;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentError, FluentResource};
use tracing::warn;
use unic_langid::LanguageIdentifier;

use crate::interface::error::AppError;
use crate::interface::state::AppState;

pub const DEFAULT_LOCALE: &str = "en";
pub const LOCALE_COOKIE_NAME: &str = "locale";
pub const REQUIRED_LOCALES: &[&str] = &["en", "es", "nl", "de", "fr"];

#[derive(Clone)]
pub struct Translator {
    bundles: Arc<HashMap<LanguageIdentifier, FluentBundle<FluentResource>>>,
    default_locale: LanguageIdentifier,
    supported: Arc<Vec<LanguageIdentifier>>,
}

impl Translator {
    pub fn load_from_disk<P: AsRef<Path>>(
        root: P,
        default_locale: &str,
        required_locales: &[&str],
    ) -> Result<Self, anyhow::Error> {
        let root = root.as_ref();
        let mut bundles = HashMap::new();

        for entry in fs::read_dir(root).context("reading locales directory")? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let locale_code = entry.file_name().to_string_lossy().to_string();
            let Some(langid) = parse_locale(&locale_code) else {
                warn!(code = %locale_code, "skipping invalid locale directory name");
                continue;
            };
            let path = entry.path().join("main.ftl");
            if !path.exists() {
                continue;
            }
            let bundle = load_bundle(&langid, &path)?;
            bundles.insert(langid, bundle);
        }

        for code in required_locales {
            let langid =
                parse_locale(code).context(format!("invalid required locale tag: {code}"))?;
            if !bundles.contains_key(&langid) {
                let path = root.join(code).join("main.ftl");
                return Err(anyhow::anyhow!(
                    "missing required locale file: {}",
                    path.display()
                ));
            }
        }

        let default_locale = parse_locale(default_locale).context("invalid default locale tag")?;
        let mut supported: Vec<_> = bundles.keys().cloned().collect();
        supported.sort();
        Ok(Self {
            bundles: Arc::new(bundles),
            default_locale,
            supported: Arc::new(supported),
        })
    }

    pub fn supported_locales(&self) -> &[LanguageIdentifier] {
        &self.supported
    }

    pub fn default_locale(&self) -> &LanguageIdentifier {
        &self.default_locale
    }

    pub fn is_supported(&self, locale: &LanguageIdentifier) -> bool {
        self.bundles.contains_key(locale)
    }

    pub fn normalize_locale(&self, candidate: &LanguageIdentifier) -> LanguageIdentifier {
        if self.is_supported(candidate) {
            candidate.clone()
        } else {
            self.default_locale.clone()
        }
    }

    pub fn text(
        &self,
        locale: &LanguageIdentifier,
        key: &str,
        args: Option<&FluentArgs>,
    ) -> String {
        self.lookup(locale, key, args)
            .or_else(|| {
                if locale != &self.default_locale {
                    self.lookup(&self.default_locale, key, args)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                warn!(%locale, key, "missing translation key");
                key.to_string()
            })
    }

    fn lookup(
        &self,
        locale: &LanguageIdentifier,
        key: &str,
        args: Option<&FluentArgs>,
    ) -> Option<String> {
        let bundle = self.bundles.get(locale)?;
        let msg = bundle.get_message(key)?;
        let value = msg.value()?;
        let mut errors: Vec<FluentError> = Vec::new();
        let formatted = bundle.format_pattern(value, args, &mut errors);
        if !errors.is_empty() {
            warn!(?errors, key, "formatting errors for translation key");
        }
        Some(formatted.to_string())
    }
}

fn load_bundle(
    locale: &LanguageIdentifier,
    path: &PathBuf,
) -> Result<FluentBundle<FluentResource>, anyhow::Error> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("reading translation file {}", path.display()))?;
    let resource = FluentResource::try_new(source)
        .map_err(|errs| anyhow::anyhow!("failed to parse {}: {:?}", path.display(), errs))?;
    let mut bundle = FluentBundle::new_concurrent(vec![locale.clone()]);
    bundle
        .add_resource(resource)
        .map_err(|errs| anyhow::anyhow!("failed to add resource {}: {:?}", path.display(), errs))?;
    Ok(bundle)
}

fn parse_locale(raw: &str) -> Option<LanguageIdentifier> {
    raw.parse::<LanguageIdentifier>().ok()
}

#[derive(Clone, Debug)]
pub struct Locale(pub LanguageIdentifier);

#[async_trait]
impl<S> FromRequestParts<S> for Locale
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let locale = resolve_locale_from_headers(&app_state, &parts.headers);
        Ok(Locale(locale))
    }
}

fn resolve_locale_from_headers(state: &AppState, headers: &HeaderMap) -> LanguageIdentifier {
    let translator = &state.i18n.translator;
    if let Some(raw) = read_cookie(headers, LOCALE_COOKIE_NAME) {
        if let Some(tag) = parse_locale(&raw) {
            let normalized = translator.normalize_locale(&tag);
            return normalized;
        }
    }

    if let Some(header_locale) = accept_language_locale(headers, translator) {
        return translator.normalize_locale(&header_locale);
    }

    translator.default_locale().clone()
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|part| Cookie::parse(part.trim()).ok())
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_string())
}

fn accept_language_locale(
    headers: &HeaderMap,
    translator: &Translator,
) -> Option<LanguageIdentifier> {
    let header = headers.get(header::ACCEPT_LANGUAGE)?.to_str().ok()?;
    for part in header.split(',') {
        let tag = part.split(';').next()?.trim();
        if let Some(loc) = parse_locale(tag) {
            if translator.is_supported(&loc) {
                return Some(loc);
            }
        }
    }
    None
}

pub fn locale_cookie(tag: &LanguageIdentifier) -> Cookie<'static> {
    Cookie::build((LOCALE_COOKIE_NAME, tag.to_string()))
        .path("/")
        .secure(true)
        .http_only(false)
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::days(365))
        .build()
}

const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const ES_MONTHS: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

const NL_MONTHS: [&str; 12] = [
    "januari",
    "februari",
    "maart",
    "april",
    "mei",
    "juni",
    "juli",
    "augustus",
    "september",
    "oktober",
    "november",
    "december",
];

const DE_MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

const FR_MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

pub fn format_date(date: &DateTime<Utc>, locale: &LanguageIdentifier) -> String {
    let month_idx = (date.month0() as usize).min(11);
    let month = match locale.language.as_str() {
        "es" => ES_MONTHS[month_idx],
        "nl" => NL_MONTHS[month_idx],
        "de" => DE_MONTHS[month_idx],
        "fr" => FR_MONTHS[month_idx],
        _ => EN_MONTHS[month_idx],
    };
    let day = date.day();
    let year = date.year();
    match locale.language.as_str() {
        "es" => format!("{day} de {month} de {year}"),
        "nl" | "fr" => format!("{day} {month} {year}"),
        "de" => format!("{day}. {month} {year}"),
        _ => format!("{month} {day}, {year}"),
    }
}

pub fn format_datetime(date: &DateTime<Utc>, locale: &LanguageIdentifier) -> String {
    let date_part = format_date(date, locale);
    format!("{} {:02}:{:02}", date_part, date.hour(), date.minute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::state::{AppState, AuthSettings, I18nState};
    use axum::http::{header, HeaderMap};
    use sqlx::PgPool;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;
    use uuid::Uuid;

    struct TempLocales {
        root: PathBuf,
    }

    impl TempLocales {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("tee_i18n_test_{}", Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write_locale(&self, code: &str, ftl: &str) {
            let dir = self.root.join(code);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("main.ftl"), ftl).unwrap();
        }
    }

    impl Drop for TempLocales {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn resolves_locale_fallback() {
        let translator =
            Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES).unwrap();
        let unknown = parse_locale("zz").unwrap();
        assert_eq!(
            translator.default_locale(),
            &translator.normalize_locale(&unknown)
        );
    }

    #[test]
    fn formats_date_in_spanish() {
        let date = DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let es = parse_locale("es").unwrap();
        assert_eq!(format_date(&date, &es), "15 de marzo de 2024");
    }

    fn dummy_state(translator: Translator) -> AppState {
        let pool =
            PgPool::connect_lazy("postgres://postgres@localhost/postgres").expect("lazy pool");
        let auth = AuthSettings::new(
            Duration::from_secs(3600),
            Duration::from_secs(1800),
            Duration::from_secs(7200),
        );
        AppState {
            pool,
            auth,
            i18n: I18nState { translator },
        }
    }

    #[tokio::test]
    async fn resolves_locale_from_cookie_before_header() {
        let translator =
            Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES).unwrap();
        let state = dummy_state(translator);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("{LOCALE_COOKIE_NAME}=es; other=ignored")
                .parse()
                .unwrap(),
        );
        headers.insert(header::ACCEPT_LANGUAGE, "fr, en;q=0.9".parse().unwrap());
        let resolved = resolve_locale_from_headers(&state, &headers);
        assert_eq!(resolved, parse_locale("es").unwrap());
    }

    #[tokio::test]
    async fn resolves_locale_from_accept_language() {
        let translator =
            Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES).unwrap();
        let state = dummy_state(translator);
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_LANGUAGE, "fr, en;q=0.9".parse().unwrap());
        let resolved = resolve_locale_from_headers(&state, &headers);
        assert_eq!(resolved, parse_locale("fr").unwrap());
    }

    #[tokio::test]
    async fn resolves_default_when_locale_is_unsupported() {
        let translator =
            Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES).unwrap();
        let state = dummy_state(translator);
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_LANGUAGE, "zz".parse().unwrap());
        let resolved = resolve_locale_from_headers(&state, &headers);
        assert_eq!(resolved, parse_locale(DEFAULT_LOCALE).unwrap());
    }

    #[test]
    fn falls_back_when_message_missing() {
        let temp = TempLocales::new();
        temp.write_locale("en", "greeting = Hello\n");
        temp.write_locale("es", "other = Hola\n");
        let translator = Translator::load_from_disk(&temp.root, "en", &["en", "es"]).unwrap();
        let es = parse_locale("es").unwrap();
        assert_eq!(translator.text(&es, "greeting", None), "Hello");
    }

    #[test]
    fn falls_back_when_message_has_no_value() {
        let temp = TempLocales::new();
        temp.write_locale("en", "greeting = Hello\n");
        temp.write_locale("es", "greeting =\n    .attr = Hola\n");
        let translator = Translator::load_from_disk(&temp.root, "en", &["en", "es"]).unwrap();
        let es = parse_locale("es").unwrap();
        assert_eq!(translator.text(&es, "greeting", None), "Hello");
    }

    #[test]
    fn returns_key_when_missing_in_all_locales() {
        let temp = TempLocales::new();
        temp.write_locale("en", "greeting = Hello\n");
        temp.write_locale("es", "saludo = Hola\n");
        let translator = Translator::load_from_disk(&temp.root, "en", &["en", "es"]).unwrap();
        let en = parse_locale("en").unwrap();
        assert_eq!(translator.text(&en, "unknown-key", None), "unknown-key");
    }
}
