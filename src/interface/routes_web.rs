use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{header::SET_COOKIE, HeaderMap},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use chrono::{NaiveDate, TimeZone, Utc};
use fluent_bundle::FluentArgs;
use unic_langid::LanguageIdentifier;
use uuid::Uuid;

use crate::app::{commands, queries};
use crate::data::auth_repo;
use crate::domain::principal::Principal;
use crate::domain::task::TaskStatus;
use crate::interface::auth::{self, AuthContext};
use crate::interface::error::AppError;
use crate::interface::i18n::{format_date, format_datetime, locale_cookie, Locale, Translator};
use crate::interface::state::{AppState, I18nState};

#[derive(Clone)]
struct LayoutTexts {
    brand: String,
    nav_tasks: String,
    theme_label: String,
    theme_light: String,
    theme_dark: String,
    theme_system: String,
    locale_label: String,
}

#[derive(Clone)]
struct LocaleOption {
    code: String,
    label: String,
    selected: bool,
}

#[derive(Clone)]
struct StatusTexts {
    planned: String,
    in_progress: String,
    completed: String,
}

#[derive(Clone)]
struct TasksListTexts {
    page_title: String,
    heading: String,
    subheading: String,
    sign_out: String,
    new_task: String,
    filter_status_all: String,
    filter_status: String,
    filter_search: String,
    filter_search_placeholder: String,
    filter_priority: String,
    filter_priority_any: String,
    filter_created_after: String,
    filter_created_before: String,
    filter_sort: String,
    filter_sort_updated_default: String,
    filter_sort_updated: String,
    filter_sort_due: String,
    filter_sort_priority: String,
    filter_reset: String,
    filter_apply: String,
    empty_state: String,
    table_title: String,
    table_status: String,
    table_priority: String,
    table_due: String,
    table_updated: String,
    table_actions: String,
    action_view: String,
    scroll_hint: String,
    no_due: String,
}

#[derive(Clone)]
struct TaskListRow {
    id: Uuid,
    title: String,
    status_label: String,
    status_class: &'static str,
    priority: i16,
    due: String,
    updated: String,
}

#[derive(Clone)]
struct TaskFormTexts {
    title_label: String,
    title_placeholder: String,
    due_date_label: String,
    priority_label: String,
    priority_1: String,
    priority_2: String,
    priority_3: String,
    priority_4: String,
    priority_5: String,
    description_label: String,
    description_placeholder: String,
    submit_create: String,
    submit_save: String,
    cancel: String,
    update_heading: String,
    description_heading: String,
}

#[derive(Clone)]
struct TaskCreateTexts {
    page_title: String,
    heading: String,
    subheading: String,
}

#[derive(Clone)]
struct TaskDetailTexts {
    page_title: String,
    timestamps_line: String,
    start: String,
    complete: String,
    delete: String,
    confirm_delete: String,
    metadata_heading: String,
    metadata_id: String,
    metadata_status: String,
    metadata_priority: String,
    metadata_due: String,
    metadata_created: String,
    metadata_updated: String,
    back_to_list: String,
    sign_out: String,
    no_due: String,
}

#[derive(Clone)]
struct TaskDetailView {
    id: Uuid,
    title: String,
    description: String,
    status_label: String,
    status_class: &'static str,
    priority: i16,
    due_display: String,
    due_input_value: Option<String>,
    created_display: String,
    updated_display: String,
    row_version: i64,
}

#[derive(Clone)]
struct LoginTexts {
    page_title: String,
    brand: String,
    brand_subtitle: String,
    heading: String,
    subheading: String,
    email: String,
    password: String,
    remember_me: String,
    submit: String,
    back_to_tasks: String,
    error_email_required: String,
    error_password_required: String,
    error_session_expired: String,
    error_invalid_credentials: String,
}

#[derive(Template)]
#[template(path = "tasks_list.html")]
struct TasksListTemplate {
    locale: String,
    layout: LayoutTexts,
    locale_options: Vec<LocaleOption>,
    texts: TasksListTexts,
    status_texts: StatusTexts,
    form_texts: TaskFormTexts,
    tasks: Vec<TaskListRow>,
    filters: TaskListFilters,
    csrf_token: String,
}

#[derive(Template)]
#[template(path = "task_new.html")]
struct TaskNewTemplate {
    locale: String,
    layout: LayoutTexts,
    locale_options: Vec<LocaleOption>,
    form_texts: TaskFormTexts,
    page_texts: TaskCreateTexts,
    error_message: Option<String>,
    csrf_token: String,
}

#[derive(Template)]
#[template(path = "task_detail.html")]
struct TaskDetailTemplate {
    locale: String,
    layout: LayoutTexts,
    locale_options: Vec<LocaleOption>,
    texts: TaskDetailTexts,
    form_texts: TaskFormTexts,
    task: TaskDetailView,
    can_start: bool,
    can_complete: bool,
    can_edit: bool,
    csrf_token: String,
}

#[derive(Template)]
#[template(path = "auth_login.html")]
struct LoginTemplate {
    locale: String,
    layout: LayoutTexts,
    locale_options: Vec<LocaleOption>,
    texts: LoginTexts,
    error_message: Option<String>,
    email_error: Option<String>,
    password_error: Option<String>,
    email_value: String,
    remember_me_checked: bool,
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct CreateTaskForm {
    title: String,
    description: String,
    due_at: Option<String>,
    priority: Option<String>,
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateTaskForm {
    title: String,
    description: String,
    due_at: Option<String>,
    priority: String,
    expected_row_version: i64,
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct TaskActionForm {
    expected_row_version: i64,
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
    remember_me: Option<String>,
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct LogoutForm {
    csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct SetLocaleForm {
    locale: String,
    redirect_to: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct TaskListParams {
    status: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
    q: Option<String>,
    priority: Option<String>,
    sort: Option<String>,
}

#[derive(Clone)]
pub struct TaskListFilters {
    pub status: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub q: Option<String>,
    pub priority: Option<String>,
    pub sort: Option<String>,
}

fn build_layout_texts(translator: &Translator, locale: &Locale) -> LayoutTexts {
    LayoutTexts {
        brand: translator.text(&locale.0, "layout-brand", None),
        nav_tasks: translator.text(&locale.0, "layout-nav-tasks", None),
        theme_label: translator.text(&locale.0, "layout-theme-label", None),
        theme_light: translator.text(&locale.0, "layout-theme-light", None),
        theme_dark: translator.text(&locale.0, "layout-theme-dark", None),
        theme_system: translator.text(&locale.0, "layout-theme-system", None),
        locale_label: translator.text(&locale.0, "layout-locale-label", None),
    }
}

fn build_locale_options(translator: &Translator, locale: &Locale) -> Vec<LocaleOption> {
    translator
        .supported_locales()
        .iter()
        .map(|loc| {
            let key = format!("locale-name-{}", loc);
            LocaleOption {
                code: loc.to_string(),
                label: translator.text(&locale.0, &key, None),
                selected: loc == &locale.0,
            }
        })
        .collect()
}

fn build_status_texts(translator: &Translator, locale: &Locale) -> StatusTexts {
    StatusTexts {
        planned: translator.text(&locale.0, "tasks-status-planned", None),
        in_progress: translator.text(&locale.0, "tasks-status-in-progress", None),
        completed: translator.text(&locale.0, "tasks-status-completed", None),
    }
}

fn build_tasks_list_texts(translator: &Translator, locale: &Locale) -> TasksListTexts {
    TasksListTexts {
        page_title: translator.text(&locale.0, "tasks-list-page-title", None),
        heading: translator.text(&locale.0, "tasks-list-heading", None),
        subheading: translator.text(&locale.0, "tasks-list-subheading", None),
        sign_out: translator.text(&locale.0, "auth-sign-out", None),
        new_task: translator.text(&locale.0, "tasks-list-new-task", None),
        filter_status_all: translator.text(&locale.0, "tasks-filters-status-all", None),
        filter_status: translator.text(&locale.0, "tasks-filters-status", None),
        filter_search: translator.text(&locale.0, "tasks-filters-search", None),
        filter_search_placeholder: translator.text(
            &locale.0,
            "tasks-filters-search-placeholder",
            None,
        ),
        filter_priority: translator.text(&locale.0, "tasks-filters-priority", None),
        filter_priority_any: translator.text(&locale.0, "tasks-filters-priority-any", None),
        filter_created_after: translator.text(&locale.0, "tasks-filters-created-after", None),
        filter_created_before: translator.text(&locale.0, "tasks-filters-created-before", None),
        filter_sort: translator.text(&locale.0, "tasks-filters-sort", None),
        filter_sort_updated_default: translator.text(
            &locale.0,
            "tasks-filters-sort-updated-default",
            None,
        ),
        filter_sort_updated: translator.text(&locale.0, "tasks-filters-sort-updated", None),
        filter_sort_due: translator.text(&locale.0, "tasks-filters-sort-due", None),
        filter_sort_priority: translator.text(&locale.0, "tasks-filters-sort-priority", None),
        filter_reset: translator.text(&locale.0, "tasks-filters-reset", None),
        filter_apply: translator.text(&locale.0, "tasks-filters-apply", None),
        empty_state: translator.text(&locale.0, "tasks-list-empty", None),
        table_title: translator.text(&locale.0, "tasks-table-title", None),
        table_status: translator.text(&locale.0, "tasks-table-status", None),
        table_priority: translator.text(&locale.0, "tasks-table-priority", None),
        table_due: translator.text(&locale.0, "tasks-table-due", None),
        table_updated: translator.text(&locale.0, "tasks-table-updated", None),
        table_actions: translator.text(&locale.0, "tasks-table-actions", None),
        action_view: translator.text(&locale.0, "tasks-list-view", None),
        scroll_hint: translator.text(&locale.0, "tasks-list-scroll-hint", None),
        no_due: translator.text(&locale.0, "tasks-common-no-due", None),
    }
}

fn build_task_form_texts(translator: &Translator, locale: &Locale) -> TaskFormTexts {
    TaskFormTexts {
        title_label: translator.text(&locale.0, "tasks-form-title", None),
        title_placeholder: translator.text(&locale.0, "tasks-form-title-placeholder", None),
        due_date_label: translator.text(&locale.0, "tasks-form-due-date", None),
        priority_label: translator.text(&locale.0, "tasks-form-priority", None),
        priority_1: translator.text(&locale.0, "tasks-form-priority-1", None),
        priority_2: translator.text(&locale.0, "tasks-form-priority-2", None),
        priority_3: translator.text(&locale.0, "tasks-form-priority-3", None),
        priority_4: translator.text(&locale.0, "tasks-form-priority-4", None),
        priority_5: translator.text(&locale.0, "tasks-form-priority-5", None),
        description_label: translator.text(&locale.0, "tasks-form-description", None),
        description_placeholder: translator.text(
            &locale.0,
            "tasks-form-description-placeholder",
            None,
        ),
        submit_create: translator.text(&locale.0, "tasks-form-submit-create", None),
        submit_save: translator.text(&locale.0, "tasks-form-submit-save", None),
        cancel: translator.text(&locale.0, "tasks-form-cancel", None),
        update_heading: translator.text(&locale.0, "tasks-form-update-heading", None),
        description_heading: translator.text(&locale.0, "tasks-form-description-heading", None),
    }
}

fn build_task_create_texts(translator: &Translator, locale: &Locale) -> TaskCreateTexts {
    TaskCreateTexts {
        page_title: translator.text(&locale.0, "tasks-new-page-title", None),
        heading: translator.text(&locale.0, "tasks-new-heading", None),
        subheading: translator.text(&locale.0, "tasks-new-subheading", None),
    }
}

fn build_task_detail_texts(
    translator: &Translator,
    locale: &Locale,
    task_id: &Uuid,
    created: &str,
    updated: &str,
) -> TaskDetailTexts {
    let mut args = FluentArgs::new();
    args.set("id", task_id.to_string());
    let page_title = translator.text(&locale.0, "tasks-detail-page-title", Some(&args));

    let mut ts_args = FluentArgs::new();
    ts_args.set("updated", updated.to_string());
    ts_args.set("created", created.to_string());
    let timestamps_line =
        translator.text(&locale.0, "tasks-detail-updated-created", Some(&ts_args));

    TaskDetailTexts {
        page_title,
        timestamps_line,
        start: translator.text(&locale.0, "tasks-detail-start", None),
        complete: translator.text(&locale.0, "tasks-detail-complete", None),
        delete: translator.text(&locale.0, "tasks-detail-delete", None),
        confirm_delete: translator.text(&locale.0, "tasks-detail-confirm-delete", None),
        metadata_heading: translator.text(&locale.0, "tasks-detail-metadata", None),
        metadata_id: translator.text(&locale.0, "tasks-detail-metadata-id", None),
        metadata_status: translator.text(&locale.0, "tasks-detail-metadata-status", None),
        metadata_priority: translator.text(&locale.0, "tasks-detail-metadata-priority", None),
        metadata_due: translator.text(&locale.0, "tasks-detail-metadata-due-date", None),
        metadata_created: translator.text(&locale.0, "tasks-detail-metadata-created", None),
        metadata_updated: translator.text(&locale.0, "tasks-detail-metadata-updated", None),
        back_to_list: translator.text(&locale.0, "tasks-detail-back-to-list", None),
        sign_out: translator.text(&locale.0, "auth-sign-out", None),
        no_due: translator.text(&locale.0, "tasks-common-no-due", None),
    }
}

fn build_login_texts(translator: &Translator, locale: &Locale) -> LoginTexts {
    LoginTexts {
        page_title: translator.text(&locale.0, "auth-login-page-title", None),
        brand: translator.text(&locale.0, "auth-login-brand", None),
        brand_subtitle: translator.text(&locale.0, "auth-login-brand-subtitle", None),
        heading: translator.text(&locale.0, "auth-login-heading", None),
        subheading: translator.text(&locale.0, "auth-login-subheading", None),
        email: translator.text(&locale.0, "auth-login-email", None),
        password: translator.text(&locale.0, "auth-login-password", None),
        remember_me: translator.text(&locale.0, "auth-login-remember-me", None),
        submit: translator.text(&locale.0, "auth-login-submit", None),
        back_to_tasks: translator.text(&locale.0, "auth-login-back", None),
        error_email_required: translator.text(&locale.0, "auth-login-error-email-required", None),
        error_password_required: translator.text(
            &locale.0,
            "auth-login-error-password-required",
            None,
        ),
        error_session_expired: translator.text(&locale.0, "auth-login-error-session-expired", None),
        error_invalid_credentials: translator.text(
            &locale.0,
            "auth-login-error-invalid-credentials",
            None,
        ),
    }
}

pub fn router(
    pool: sqlx::PgPool,
    auth_settings: crate::interface::state::AuthSettings,
    translator: Translator,
) -> Router {
    let state = AppState {
        pool,
        auth: auth_settings,
        i18n: I18nState { translator },
    };
    Router::new()
        .route("/", get(|| async { Redirect::to("/tasks") }))
        .route("/auth/login", get(auth_login_form).post(auth_login_submit))
        .route("/auth/logout", axum::routing::post(auth_logout))
        .route("/auth/me", get(auth_me))
        .route("/i18n/locale", post(set_locale))
        .route("/tasks", get(tasks_list).post(tasks_create))
        .route("/tasks/new", get(task_new))
        .route("/tasks/:id", get(task_detail))
        .route("/tasks/:id/start", axum::routing::post(task_start))
        .route("/tasks/:id/complete", axum::routing::post(task_complete))
        .route(
            "/tasks/:id/update",
            axum::routing::post(task_update_details),
        )
        .route("/tasks/:id/delete", axum::routing::post(task_delete))
        .with_state(state)
}

async fn auth_login_form(
    State(st): State<AppState>,
    locale: Locale,
    auth_ctx: AuthContext,
) -> Result<Response, AppError> {
    if auth_ctx.principal.is_some() {
        return Ok(Redirect::to("/tasks").into_response());
    }
    let layout = build_layout_texts(&st.i18n.translator, &locale);
    let locale_options = build_locale_options(&st.i18n.translator, &locale);
    let texts = build_login_texts(&st.i18n.translator, &locale);
    let csrf_token = auth::generate_login_csrf_token();
    let template = LoginTemplate {
        locale: locale.0.to_string(),
        layout,
        locale_options,
        texts,
        error_message: None,
        email_error: None,
        password_error: None,
        email_value: "".to_string(),
        remember_me_checked: false,
        csrf_token: csrf_token.clone(),
    };
    render_login(template, csrf_token)
}

async fn auth_login_submit(
    State(st): State<AppState>,
    locale: Locale,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let layout = build_layout_texts(&st.i18n.translator, &locale);
    let locale_options = build_locale_options(&st.i18n.translator, &locale);
    let texts = build_login_texts(&st.i18n.translator, &locale);
    let mut email_error = None;
    let mut password_error = None;
    let email_trimmed = form.email.trim();
    let password_trimmed = form.password.trim();
    if email_trimmed.is_empty() {
        email_error = Some(texts.error_email_required.clone());
    }
    if password_trimmed.is_empty() {
        password_error = Some(texts.error_password_required.clone());
    }

    if email_error.is_some() || password_error.is_some() {
        let csrf_token = auth::generate_login_csrf_token();
        let template = LoginTemplate {
            locale: locale.0.to_string(),
            layout: layout.clone(),
            locale_options: locale_options.clone(),
            texts: texts.clone(),
            error_message: None,
            email_error,
            password_error,
            email_value: email_trimmed.to_string(),
            remember_me_checked: form.remember_me.is_some(),
            csrf_token: csrf_token.clone(),
        };
        return render_login(template, csrf_token);
    }

    if auth::validate_login_csrf(&headers, &form.csrf_token).is_err() {
        let csrf_token = auth::generate_login_csrf_token();
        let template = LoginTemplate {
            locale: locale.0.to_string(),
            layout: layout.clone(),
            locale_options: locale_options.clone(),
            texts: texts.clone(),
            error_message: Some(texts.error_session_expired.clone()),
            email_error: None,
            password_error: None,
            email_value: email_trimmed.to_string(),
            remember_me_checked: form.remember_me.is_some(),
            csrf_token: csrf_token.clone(),
        };
        return render_login(template, csrf_token);
    }

    let user = auth_repo::fetch_user_by_email(&st.pool, email_trimmed).await?;
    let remember_me = form.remember_me.is_some();
    let is_valid = match user.as_ref() {
        Some(row) if row.is_active => auth::verify_password(&row.password_hash, password_trimmed),
        _ => false,
    };

    if !is_valid {
        tracing::info!(email = %email_trimmed, "login failed");
        let csrf_token = auth::generate_login_csrf_token();
        let invalid_credentials_msg = texts.error_invalid_credentials.clone();
        let template = LoginTemplate {
            locale: locale.0.to_string(),
            layout,
            locale_options,
            texts,
            error_message: Some(invalid_credentials_msg),
            email_error: None,
            password_error: None,
            email_value: email_trimmed.to_string(),
            remember_me_checked: remember_me,
            csrf_token: csrf_token.clone(),
        };
        return render_login(template, csrf_token);
    }

    let user = user.expect("validated above");
    let session_token = auth::generate_session_token();
    let session_hash = auth::hash_token(&session_token);
    let lifetime = if remember_me {
        st.auth.remember_me_lifetime
    } else {
        st.auth.session_lifetime
    };
    let expires_at = chrono::Duration::from_std(lifetime)
        .map_err(|_| AppError::Internal("invalid session lifetime".to_string()))?;
    let expires_at = Utc::now() + expires_at;
    let session_id =
        auth_repo::insert_session(&st.pool, user.id, &session_hash, expires_at).await?;
    tracing::info!(user_id = %user.id, session_id = %session_id, "login success");

    let mut response = Redirect::to("/tasks").into_response();
    let session_cookie = auth::session_cookie(&session_token, &st.auth, remember_me);
    let login_cookie = auth::clear_login_csrf_cookie();
    response
        .headers_mut()
        .append(SET_COOKIE, session_cookie.to_string().parse().unwrap());
    response
        .headers_mut()
        .append(SET_COOKIE, login_cookie.to_string().parse().unwrap());
    let locale_cookie_value = locale_cookie(&locale.0);
    response
        .headers_mut()
        .append(SET_COOKIE, locale_cookie_value.to_string().parse().unwrap());
    Ok(response)
}

async fn auth_logout(
    State(st): State<AppState>,
    auth_ctx: AuthContext,
    Form(form): Form<LogoutForm>,
) -> Result<Response, AppError> {
    if auth_ctx.principal.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    auth::validate_csrf(&auth_ctx, &form.csrf_token)?;
    if let Some(session_id) = auth_ctx.session_id {
        auth_repo::revoke_session(&st.pool, session_id).await?;
        tracing::info!(session_id = %session_id, "logout");
    }
    let mut response = Redirect::to("/auth/login").into_response();
    let cookie = auth::clear_session_cookie();
    response
        .headers_mut()
        .append(SET_COOKIE, cookie.to_string().parse().unwrap());
    Ok(response)
}

async fn auth_me(auth_ctx: AuthContext) -> Result<impl IntoResponse, AppError> {
    let principal = auth_ctx.principal.ok_or(AppError::AuthenticationRequired)?;
    Ok(Json(principal))
}

async fn set_locale(
    State(st): State<AppState>,
    locale: Locale,
    Form(form): Form<SetLocaleForm>,
) -> Result<Response, AppError> {
    let requested = form
        .locale
        .parse::<LanguageIdentifier>()
        .ok()
        .map(|loc| st.i18n.translator.normalize_locale(&loc))
        .unwrap_or_else(|| locale.0.clone());
    let redirect_target = sanitize_redirect(form.redirect_to.as_deref());
    let mut response = Redirect::to(&redirect_target).into_response();
    let cookie = locale_cookie(&requested);
    response
        .headers_mut()
        .append(SET_COOKIE, cookie.to_string().parse().unwrap());
    Ok(response)
}

fn render_login(template: LoginTemplate, csrf_token: String) -> Result<Response, AppError> {
    let html = template.render()?;
    let mut response = Html(html).into_response();
    let cookie = auth::login_csrf_cookie(&csrf_token);
    response
        .headers_mut()
        .append(SET_COOKIE, cookie.to_string().parse().unwrap());
    if let Ok(locale) = template.locale.parse::<LanguageIdentifier>() {
        let locale_cookie = locale_cookie(&locale);
        response
            .headers_mut()
            .append(SET_COOKIE, locale_cookie.to_string().parse().unwrap());
    }
    Ok(response)
}

fn require_principal(auth: &AuthContext) -> Result<Principal, Redirect> {
    match auth.principal.clone() {
        Some(principal) => Ok(principal),
        None => Err(Redirect::to("/auth/login")),
    }
}

fn require_csrf_token(auth: &AuthContext) -> Result<String, AppError> {
    auth.csrf_token
        .clone()
        .ok_or(AppError::AuthenticationRequired)
}

fn ensure_csrf_valid(auth: &AuthContext, submitted: &str) -> Result<(), AppError> {
    auth::validate_csrf(auth, submitted)
}

async fn tasks_list(
    State(st): State<AppState>,
    locale: Locale,
    auth: AuthContext,
    Query(params): Query<TaskListParams>,
) -> Result<Response, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    let csrf_token = require_csrf_token(&auth)?;
    let layout = build_layout_texts(&st.i18n.translator, &locale);
    let locale_options = build_locale_options(&st.i18n.translator, &locale);
    let texts = build_tasks_list_texts(&st.i18n.translator, &locale);
    let status_texts = build_status_texts(&st.i18n.translator, &locale);
    let form_texts = build_task_form_texts(&st.i18n.translator, &locale);

    let invalid_status_msg =
        st.i18n
            .translator
            .text(&locale.0, "error-invalid-status-filter", None);
    let invalid_priority_filter_msg =
        st.i18n
            .translator
            .text(&locale.0, "error-invalid-priority-filter", None);
    let invalid_sort_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-sort-option", None);
    let invalid_date_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-date", None);

    let status = match params.status.as_deref() {
        None | Some("") => None,
        Some(raw) => {
            let normalized = raw.trim().to_ascii_uppercase();
            Some(
                TaskStatus::parse(&normalized)
                    .map_err(|_| AppError::BadRequest(invalid_status_msg.clone()))?,
            )
        }
    };

    let created_after =
        parse_date_filter(params.created_after.as_deref(), false, &invalid_date_msg)?;
    let created_before =
        parse_date_filter(params.created_before.as_deref(), true, &invalid_date_msg)?;
    let priority = match params.priority.as_deref() {
        None | Some("") => None,
        Some(raw) => {
            let parsed: i16 = raw
                .parse()
                .map_err(|_| AppError::BadRequest(invalid_priority_filter_msg.clone()))?;
            if !(1..=5).contains(&parsed) {
                return Err(AppError::BadRequest(invalid_priority_filter_msg.clone()));
            }
            Some(parsed)
        }
    };
    let sort = match params.sort.as_deref() {
        None | Some("") => None,
        Some("updated_at") | Some("due_at") | Some("priority") => params.sort.clone(),
        Some(_) => {
            return Err(AppError::BadRequest(invalid_sort_msg));
        }
    };
    let query = queries::list_tasks::ListTasksQuery {
        status,
        created_after,
        created_before,
        search: params.q.clone().filter(|s| !s.trim().is_empty()),
        priority,
        sort,
        limit: 50,
    };
    let tasks = queries::list_tasks::handle(&st.pool, &principal, query).await?;
    let task_rows: Vec<TaskListRow> = tasks
        .into_iter()
        .map(|t| TaskListRow {
            id: t.id,
            title: t.title,
            status_label: localized_status_label(t.status, &status_texts),
            status_class: status_badge_class(t.status),
            priority: t.priority,
            due: t
                .due_at
                .map(|d| format_date(&d, &locale.0))
                .unwrap_or_else(|| texts.no_due.clone()),
            updated: format_datetime(&t.updated_at, &locale.0),
        })
        .collect();
    let html = TasksListTemplate {
        locale: locale.0.to_string(),
        layout,
        locale_options,
        texts,
        status_texts,
        form_texts,
        tasks: task_rows,
        filters: TaskListFilters {
            status: params.status,
            created_after: params.created_after,
            created_before: params.created_before,
            q: params.q,
            priority: params.priority,
            sort: params.sort,
        },
        csrf_token,
    }
    .render()?;
    Ok(Html(html).into_response())
}

async fn task_new(
    State(st): State<AppState>,
    locale: Locale,
    auth: AuthContext,
) -> Result<Response, AppError> {
    if let Err(redirect) = require_principal(&auth) {
        return Ok(redirect.into_response());
    }
    let csrf_token = require_csrf_token(&auth)?;
    let layout = build_layout_texts(&st.i18n.translator, &locale);
    let locale_options = build_locale_options(&st.i18n.translator, &locale);
    let form_texts = build_task_form_texts(&st.i18n.translator, &locale);
    let page_texts = build_task_create_texts(&st.i18n.translator, &locale);
    let html = TaskNewTemplate {
        locale: locale.0.to_string(),
        layout,
        locale_options,
        form_texts,
        page_texts,
        error_message: None,
        csrf_token,
    }
    .render()?;
    Ok(Html(html).into_response())
}

async fn tasks_create(
    State(st): State<AppState>,
    locale: Locale,
    auth: AuthContext,
    Form(form): Form<CreateTaskForm>,
) -> Result<impl IntoResponse, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    ensure_csrf_valid(&auth, &form.csrf_token)?;
    let invalid_due_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-due-date", None);
    let invalid_priority_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-priority", None);
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: form.title,
        description_raw: form.description,
        due_at: parse_due_date(form.due_at.as_deref(), &invalid_due_msg)?,
        priority_raw: parse_priority(form.priority.as_deref(), &invalid_priority_msg)?,
    };

    commands::create_task::handle(&st.pool, &principal, cmd).await?;
    Ok(Redirect::to("/tasks").into_response())
}

async fn task_detail(
    State(st): State<AppState>,
    locale: Locale,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    let csrf_token = require_csrf_token(&auth)?;
    let task = queries::get_task::handle(&st.pool, &principal, id).await?;
    let can_start = task.status == TaskStatus::Planned;
    let can_complete = task.status == TaskStatus::InProgress;
    let can_edit = task.status != TaskStatus::Completed;
    let layout = build_layout_texts(&st.i18n.translator, &locale);
    let locale_options = build_locale_options(&st.i18n.translator, &locale);
    let status_texts = build_status_texts(&st.i18n.translator, &locale);
    let form_texts = build_task_form_texts(&st.i18n.translator, &locale);
    let created_display = format_datetime(&task.created_at, &locale.0);
    let updated_display = format_datetime(&task.updated_at, &locale.0);
    let texts = build_task_detail_texts(
        &st.i18n.translator,
        &locale,
        &task.id,
        &created_display,
        &updated_display,
    );
    let due_display = task
        .due_at
        .map(|d| format_date(&d, &locale.0))
        .unwrap_or_else(|| texts.no_due.clone());
    let due_at_value = task
        .due_at
        .as_ref()
        .map(|d| d.format("%Y-%m-%d").to_string());
    let task_view = TaskDetailView {
        id: task.id,
        title: task.title,
        description: task.description,
        status_label: localized_status_label(task.status, &status_texts),
        status_class: status_badge_class(task.status),
        priority: task.priority,
        due_display,
        due_input_value: due_at_value,
        created_display,
        updated_display,
        row_version: task.row_version,
    };
    let html = TaskDetailTemplate {
        locale: locale.0.to_string(),
        layout,
        locale_options,
        texts,
        form_texts,
        task: task_view,
        can_start,
        can_complete,
        can_edit,
        csrf_token,
    }
    .render()?;
    Ok(Html(html).into_response())
}

async fn task_start(
    State(st): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Form(form): Form<TaskActionForm>,
) -> Result<impl IntoResponse, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    ensure_csrf_valid(&auth, &form.csrf_token)?;
    let cmd = commands::start_task::StartTaskCommand {
        id,
        expected_row_version: form.expected_row_version,
    };
    commands::start_task::handle(&st.pool, &principal, cmd).await?;
    Ok(Redirect::to(&format!("/tasks/{id}")).into_response())
}

async fn task_complete(
    State(st): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Form(form): Form<TaskActionForm>,
) -> Result<impl IntoResponse, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    ensure_csrf_valid(&auth, &form.csrf_token)?;
    let cmd = commands::complete_task::CompleteTaskCommand {
        id,
        expected_row_version: form.expected_row_version,
    };
    commands::complete_task::handle(&st.pool, &principal, cmd).await?;
    Ok(Redirect::to(&format!("/tasks/{id}")).into_response())
}

async fn task_update_details(
    State(st): State<AppState>,
    locale: Locale,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Form(form): Form<UpdateTaskForm>,
) -> Result<impl IntoResponse, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    ensure_csrf_valid(&auth, &form.csrf_token)?;
    let invalid_due_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-due-date", None);
    let invalid_priority_msg = st
        .i18n
        .translator
        .text(&locale.0, "error-invalid-priority", None);
    let cmd = commands::update_task_details::UpdateTaskDetailsCommand {
        id,
        title_raw: form.title,
        description_raw: form.description,
        due_at: parse_due_date(form.due_at.as_deref(), &invalid_due_msg)?,
        priority_raw: parse_priority(Some(&form.priority), &invalid_priority_msg)?.unwrap_or(3),
        expected_row_version: form.expected_row_version,
    };
    commands::update_task_details::handle(&st.pool, &principal, cmd).await?;
    Ok(Redirect::to(&format!("/tasks/{id}")).into_response())
}

async fn task_delete(
    State(st): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Form(form): Form<TaskActionForm>,
) -> Result<impl IntoResponse, AppError> {
    let principal = match require_principal(&auth) {
        Ok(principal) => principal,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    ensure_csrf_valid(&auth, &form.csrf_token)?;
    let cmd = commands::delete_task::DeleteTaskCommand {
        id,
        expected_row_version: form.expected_row_version,
    };
    commands::delete_task::handle(&st.pool, &principal, cmd).await?;
    Ok(Redirect::to("/tasks").into_response())
}

fn localized_status_label(status: TaskStatus, texts: &StatusTexts) -> String {
    match status {
        TaskStatus::Planned => texts.planned.clone(),
        TaskStatus::InProgress => texts.in_progress.clone(),
        TaskStatus::Completed => texts.completed.clone(),
    }
}

fn status_badge_class(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Planned => "inline-flex items-center gap-1 rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-slate-700 dark:bg-slate-800 dark:text-slate-200",
        TaskStatus::InProgress => "inline-flex items-center gap-1 rounded-full bg-blue-100 px-3 py-1 text-xs font-semibold text-blue-700 dark:bg-blue-900 dark:text-blue-100",
        TaskStatus::Completed => "inline-flex items-center gap-1 rounded-full bg-green-100 px-3 py-1 text-xs font-semibold text-green-700 dark:bg-green-900 dark:text-green-100",
    }
}

fn sanitize_redirect(candidate: Option<&str>) -> String {
    if let Some(path) = candidate {
        if path.starts_with('/') {
            return path.to_string();
        }
    }
    "/tasks".to_string()
}

fn parse_date_filter(
    raw: Option<&str>,
    end_of_day: bool,
    invalid_message: &str,
) -> Result<Option<chrono::DateTime<Utc>>, AppError> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.is_empty() {
        return Ok(None);
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest(invalid_message.to_string()))?;
    let time = if end_of_day { (23, 59, 59) } else { (0, 0, 0) };
    Ok(Some(Utc.from_utc_datetime(
        &date.and_hms_opt(time.0, time.1, time.2).unwrap(),
    )))
}

fn parse_due_date(
    raw: Option<&str>,
    invalid_message: &str,
) -> Result<Option<chrono::DateTime<Utc>>, AppError> {
    let Some(raw) = raw else { return Ok(None) };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let date = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest(invalid_message.to_string()))?;
    Ok(Some(
        Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()),
    ))
}

fn parse_priority(raw: Option<&str>, invalid_message: &str) -> Result<Option<i16>, AppError> {
    let Some(raw) = raw else { return Ok(None) };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed: i16 = trimmed
        .parse()
        .map_err(|_| AppError::BadRequest(invalid_message.to_string()))?;
    if !(1..=5).contains(&parsed) {
        return Err(AppError::BadRequest(invalid_message.to_string()));
    }
    Ok(Some(parsed))
}
