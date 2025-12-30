# SPEC-04-iiI18N
## Internationalization (i18n) and Localization (l10n)
**Tee System Reference Extension**

**Status:** Draft  
**Version:** 1.0  
**Depends on:**
- Tee System Architecture Guidelines (v1.0)  (docs/Tee-Architecture-Guidelines.md)
- SPEC-01 — Task Registration & Lifecycle Management  (docs/SPECS/SPEC-01.md)
- SPEC-02 — Soft Delete, Scheduling, and Concurrency  (docs/SPECS/SPEC-02.md)
- SPEC-03-AUTH — Authentication & Session Management  (docs/SPECS/SPEC-03.md)

---

## 1. Purpose

This specification defines a pragmatic, production-ready approach to internationalization (i18n) and localization (l10n) for Tee System applications. It prescribes how to detect user locale, where to place translations, how to integrate translations into server-rendered templates and queries, and how to keep the solution consistent with the Tee constraints (single Rust service + Postgres, SQL-first, HTML-first UI, minimal JS).

The goal is to deliver fully localizable user-facing surfaces while keeping domain data language-neutral and maintainable.

---

## 2. Architectural Context

This SPEC follows the Tee System constraints:

- The Interface layer is responsible for locale detection and formatting.
- Domain logic remains language-neutral (no UI text in domain types).
- SQL remains explicit and parameterized; translations are repository-first and resolved in-memory at render time.
- Templates are server-rendered; translations must be resolved before rendering.

No additional always-on infrastructure is introduced by default. DB-backed translation stores are discouraged for core UI copy and considered an opt-in extension only when runtime-editable content is strictly required.

---

## 3. Scope

### In scope

- Locale detection and negotiation
- UI string catalogs (repo-based) and optional DB-backed translation tables
- Template integration for Askama-rendered pages
- Query patterns for localized content with fallback semantics
- Pluralization and ICU-style message formatting
- Caching and runtime performance considerations
- CI checks for translation key consistency

### Out of scope

- Machine translation services and automatic content translation
- Complex editorial workflows beyond basic admin CRUD for translations
- Translation management tooling (use external tools as needed)

---

## 4. Locale Detection (Interface Layer)

Locale resolution must follow a predictable precedence:

1. Explicit user preference (Principal/profile)
2. Persisted cookie (`locale`)
3. `Accept-Language` header
4. Application default (e.g., `en`)

Implement resolution as middleware that:

- Normalizes and validates locale tags (BCP47)
- Attaches a `Locale` value to request state available to handlers and templates
- Optionally sets/updates the locale cookie when user preference is explicit

---

## 5. Translation Sources (Repository-First)

Repository-based catalogs are the normative approach for Tee applications and must be used for all static UI copy and messages:

- Store translation files in the repo (Fluent, JSON, or PO files).
- Load bundles into memory at startup; allow hot-reload in development.
- Benefits: code review, CI checks, fast runtime, simple deployment, and predictable behavior.

DB-backed translations are strongly discouraged for core UI strings. They may be considered only for specific admin-editable content and only when the team accepts the operational and testing overhead (caching, migrations, admin commands). If used, they must be treated as an opt-in extension and carefully audited.

---

## 6. (Optional) DB-backed Translations — Appendix

If a project chooses to opt into DB-backed translations for runtime-editable content, treat the following as an appendix and not the primary recommended flow. Any DB-backed approach must include:

- Explicit migrations creating translation tables and indexes
- Command handlers for editing translations that run in transactions and invalidate caches
- Strong caching and monitoring to prevent runtime performance regressions

Example conceptual table (for reference only):

```sql
CREATE TABLE task_translations (
  task_id UUID NOT NULL REFERENCES tasks(id),
  locale TEXT NOT NULL,
  title TEXT,
  description TEXT,
  PRIMARY KEY (task_id, locale)
);

CREATE INDEX idx_task_translations_locale ON task_translations (task_id, locale);
```

Prefer repository-based catalogs unless there is a compelling, documented need for runtime-editable translations.

---

## 7. Template Integration (Askama)

- Resolve translation keys in Query handlers or a translator component before passing data to Askama templates.
- Prefer providing fully localized view models to templates (strings only) to keep templates free of I/O.
- If helper translation functions are needed in templates, implement them as pure in-memory lookups exposed to Askama — do not perform DB calls from templates.

Example: Query handler builds `TaskView { title: String, description: String, ... }` for the resolved locale.

---

## 8. Query Patterns and Fallbacks (Repository-First)

For repository-based catalogs, Query handlers should resolve translation keys and pass localized strings to templates. The resolution strategy:

1. Attempt to find the key in the requested locale bundle
2. If missing, lookup the key in the default locale bundle (e.g., `en`)
3. If still missing, return a stable placeholder (e.g., the key or a short fallback) and emit a metric/log

When DB-backed translations are used as an opt-in extension, implement an explicit join/fallback strategy in SQL as shown below (conceptual) and cache results to avoid per-request DB overhead:

```sql
SELECT coalesce(tt.title, t.title_default) AS title
FROM tasks t
LEFT JOIN task_translations tt ON tt.task_id = t.id AND tt.locale = $1
WHERE t.id = $2;
```

Keep SQL explicit and parameterized; prefer in-memory bundle resolution for UI copy.

---

## 9. Pluralization, Interpolation and Formatting

- Use a message format engine that supports ICU/plural rules (e.g., `fluent` or MessageFormat) rather than ad-hoc plural code.
- Perform number/date/time formatting in the Interface layer using the request `Locale` and a small formatting helper.
- Avoid embedding pluralization logic in templates; provide preformatted strings where possible.

---

## 10. Commands and Translation Editing

- Editing translations is a separate Command (admin flow). Translation Commands run in explicit transactions and update DB rows and cache state.
- Translation edits must not be mixed with domain state transitions in the same transaction unless intentionally coupled.

---

## 11. Caching and Performance

- Load repo-based catalogs into an in-memory cache keyed by locale.
- For DB-backed translations, cache `entity_id+locale` results with a short TTL and invalidate on translation Commands.
- Measure cache hit-rate; avoid per-request DB lookups for every template string.

---

## 12. CI and Quality Gates

- Enforce key consistency across locales in CI (missing keys fail build or warn).
- Lint translation files for syntax errors (Fluent/PO/JSON linting).
- Add unit tests for plural rules and formatting.

---

## 13. Observability

- Emit metrics: `i18n_missing_keys_total`, `i18n_cache_hits`, `i18n_cache_misses`.
- Log missing translations at WARN with route and locale context.

---

## 14. Migration Requirements

Repository-based catalogs do not require DB migrations. If a project opts into DB-backed translations, add idempotent migrations (e.g., `0003_add_translations.sql`) to create translation tables and indexes and include tests for migration behavior.

---

## 15. Testing Requirements

- Unit tests for translator logic and pluralization rules.
- Integration tests verifying query fallbacks and formatted outputs for several locales.
- End-to-end tests for major UI flows rendered in at least two locales.

---

## 16. Non-Goals

- Automatic machine translation of user content.
- Heavyweight translation management UIs beyond simple admin CRUD.
- Introducing an external translation service or separate always-on tier by default.

---

## 17. Summary

SPEC-04 provides a minimal, maintainable i18n/l10n approach that fits the Tee System constraints: locale resolution in the Interface layer, language-neutral domain, explicit SQL for translation joins, repo-first catalogs with optional DB-backed translations, and robust CI checks. Implementations should favor repo-based catalogs for static UI copy and reserve DB-backed translations for content that must be edited at runtime.

Any implementation of this SPEC must fully comply with the **Tee System Architecture Guidelines (v1.0)** and SPEC-01..SPEC-03.

---

## Implementation Recipe (practical)

The following concrete steps translate the SPEC into an implementable plan in Rust. These incorporate the 15 recommendations (crates, layout, loader, Translator API, middleware, handler integration, formatting, testing, CI, caching, hot-reload, embedding).

1) Format & crates
- Message format: Fluent (.ftl) is recommended. Use `fluent-bundle` and `fluent-syntax` for runtime formatting and parsing. Use `unic-langid` or `icu_locid` for BCP47 locale IDs. Optionally use `i18n-embed` for compile-time embedding.

2) Repo layout
- Store catalogs under `locales/<locale>/main.ftl` (e.g., `locales/en/main.ftl`, `locales/fr/main.ftl`). Keep keys stable and reviewable in PRs.

3) Load bundles at startup
- Read locale files into `FluentBundle` instances at service startup and store in an in-memory map keyed by locale. Support hot-reload in dev.

4) Translator wrapper
- Implement a small, thread-safe `Translator` service that exposes `t(locale, key, args)` and handles fallback to the default locale.

5) Locale middleware
- Add middleware to resolve locale precedence: user profile → cookie → `Accept-Language` → default. Normalize tags and attach the chosen `Locale` to request state.

6) Handler / Query integration
- Resolve translation keys in Query/handler code and construct fully-localized view models passed to Askama templates. Avoid I/O in templates.

7) Askama templates
- Templates receive localized strings only. If helper translation calls are needed, expose a pure, in-memory translator function to Askama (no DB access).

8) Pluralization and interpolation
- Use Fluent args (or MessageFormat) for pluralization and interpolation; leave rules to the message engine.

9) Date/number formatting
- Format numbers and dates in the Interface layer using the request `Locale`. Use ICU crates if precise locale formatting is required.

10) Hot-reload in development
- Watch `locales/` during development and reload bundles on changes (or restart via `cargo-watch`).

11) CI checks
- Lint Fluent files and run a key-consistency check across locales in CI; fail or warn on missing keys according to policy.

12) Tests
- Unit-test translator behavior, plural rules, and fallback behavior. Integration-test rendering pages in two locales.

13) Caching & performance
- Keep bundles in memory for fast lookups. If DB-backed translations are used (opt-in), cache `entity_id+locale` results with TTL and invalidate on writes.

14) Observability
- Export metrics: `i18n_missing_keys_total`, `i18n_cache_hits`, `i18n_cache_misses`. Log missing keys at WARN with route and locale.

15) Optional embedding
- For immutable deployments, consider `i18n-embed` to embed catalogs into the binary while keeping repo files for editing and CI.

Minimal loader + translator sketches (conceptual):

```rust
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;
use std::collections::HashMap;

fn load_bundles() -> HashMap<LanguageIdentifier, FluentBundle<FluentResource>> {
    let mut bundles = HashMap::new();
    for entry in std::fs::read_dir("locales").unwrap() {
        let loc = entry.unwrap().file_name().into_string().unwrap();
        let id: LanguageIdentifier = loc.parse().unwrap();
        let ftl = std::fs::read_to_string(format!("locales/{}/main.ftl", loc)).unwrap();
        let res = FluentResource::try_new(ftl).unwrap();
        let mut bundle = FluentBundle::new(&[id.clone()]);
        bundle.add_resource(res).unwrap();
        bundles.insert(id, bundle);
    }
    bundles
}
```

Translator sketch:

```rust
pub struct Translator { bundles: Arc<HashMap<LanguageIdentifier, FluentBundle<FluentResource>>>, default: LanguageIdentifier }
impl Translator {
  pub fn t(&self, locale: &LanguageIdentifier, key: &str, args: Option<&FluentArgs>) -> String { /* lookup with fallback */ }
}
```

Middleware should attach the resolved `Locale` to request state so handlers can call `translator.t(&locale, "page.title")` and build view models.

These steps provide a practical, Rust-aligned implementation path that preserves the Tee constraints and favors repository-first, file-based translations.

<!-- Required locales for deployments -->
## Required Locales

Use the current UI in [templates](../../templates/) to provide the EN definitions of all elements of the Talsk UI.

Implementations MUST include repository-based Fluent catalogs for the following locales:

- `en` (default)
- `es`
- `nl`
- `de`
- `fr`

Each locale MUST provide a `main.ftl` (or the project's chosen message file) under `locales/<locale>/` (for example `locales/en/main.ftl`). CI checks should validate that keys present in `en` are present in the other required locales according to the project's policy (fail or warn on missing keys).

