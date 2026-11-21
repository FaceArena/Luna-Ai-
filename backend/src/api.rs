use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

use crate::{providers::{self, ChatMessage, Provider, ProviderConfig}, AppState};

const SESSION_COOKIE: &str = "luna_session";

#[derive(Deserialize)]
pub struct AuthInput {
    pub name: Option<String>,
    pub email: String,
    pub pass: String,
}

#[derive(Deserialize)]
struct SupabaseUser {
    id: String,
    email: Option<String>,
    #[serde(default)]
    user_metadata: Value,
}

#[derive(Deserialize)]
struct AuthResult {
    access_token: Option<String>,
    user: Option<SupabaseUser>,
    msg: Option<String>,
    message: Option<String>,
}

#[derive(Serialize)]
pub struct Me {
    name: String,
    email: String,
    plan: usize,
}

fn me(user: SupabaseUser) -> Me {
    let name = user.user_metadata.get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Luna user")
        .to_owned();
    let plan = user.user_metadata.get("plan")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(2) as usize;
    Me { name, email: user.email.unwrap_or_default(), plan }
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers.get(header::COOKIE)?.to_str().ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find(|(key, _)| *key == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
}

fn set_session_cookie(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=3600"
    )).unwrap()
}

fn clear_session_cookie() -> HeaderValue {
    HeaderValue::from_static("luna_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0")
}

fn error(status: StatusCode, message: &'static str) -> Response {
    (status, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], message).into_response()
}

async fn auth_user(state: &AppState, token: &str) -> Result<SupabaseUser, Response> {
    let response = state.http
        .get(format!("{}/auth/v1/user", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .send().await
        .map_err(|_| error(StatusCode::BAD_GATEWAY, "Не удалось проверить сессию"))?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(error(StatusCode::UNAUTHORIZED, "Сессия закончилась. Войдите снова."));
    }
    if !response.status().is_success() {
        return Err(error(StatusCode::BAD_GATEWAY, "Ошибка проверки аккаунта"));
    }
    response.json::<SupabaseUser>().await
        .map_err(|_| error(StatusCode::BAD_GATEWAY, "Некорректный ответ сервиса аккаунтов"))
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<AuthInput>,
) -> Response {
    if input.email.len() > 254 || input.pass.len() < 8 || input.pass.len() > 128 {
        return error(StatusCode::BAD_REQUEST, "Проверьте почту и пароль (8–128 символов).");
    }
    let name = input.name.unwrap_or_default().trim().chars().take(40).collect::<String>();
    if name.is_empty() {
        return error(StatusCode::BAD_REQUEST, "Введите имя.");
    }

    let response = match state.http
        .post(format!("{}/auth/v1/signup", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .json(&json!({"email":input.email.trim(),"password":input.pass,"data":{"name":name}}))
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Нет связи с сервисом аккаунтов."),
        };

    let status = response.status();
    let body = match response.json::<AuthResult>().await {
        Ok(body) => body,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "Некорректный ответ сервиса аккаунтов."),
    };
    if !status.is_success() {
        return (
            StatusCode::BAD_REQUEST,
            body.msg.or(body.message).unwrap_or_else(|| "Не удалось создать аккаунт.".into())
        ).into_response();
    }
    let (Some(token), Some(user)) = (body.access_token, body.user) else {
        return error(StatusCode::CONFLICT, "Аккаунт создан. Подтвердите email, затем войдите.");
    };

    let mut response = Json(me(user)).into_response();
    response.headers_mut().insert(header::SET_COOKIE, set_session_cookie(&token));
    response
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<AuthInput>,
) -> Response {
    let response = match state.http
        .post(format!("{}/auth/v1/token?grant_type=password", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .json(&json!({"email":input.email.trim(),"password":input.pass}))
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Нет связи с сервисом аккаунтов."),
        };
    let status = response.status();
    let body = match response.json::<AuthResult>().await {
        Ok(body) => body,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "Некорректный ответ сервиса аккаунтов."),
    };
    if !status.is_success() {
        return error(StatusCode::UNAUTHORIZED, "Неверная почта или пароль.");
    }
    let (Some(token), Some(user)) = (body.access_token, body.user) else {
        return error(StatusCode::UNAUTHORIZED, "Не удалось открыть сессию.");
    };
    let mut response = Json(me(user)).into_response();
    response.headers_mut().insert(header::SET_COOKIE, set_session_cookie(&token));
    response
}

pub async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "Требуется вход.");
    };
    match auth_user(&state, &token).await {
        Ok(user) => Json(me(user)).into_response(),
        Err(response) => response,
    }
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if let Some(token) = cookie_token(&headers) {
        let _ = state.http
            .post(format!("{}/auth/v1/logout", state.supabase_url))
            .header("apikey", state.supabase_anon_key.as_ref())
            .bearer_auth(token)
            .send()
            .await;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(header::SET_COOKIE, clear_session_cookie());
    response
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatRecord {
    pub id: i64,
    pub title: String,
    pub msgs: Vec<ChatMessage>,
}

async fn token_and_user(state: &AppState, headers: &HeaderMap) -> Result<(String, SupabaseUser), Response> {
    let token = cookie_token(headers).ok_or_else(|| error(StatusCode::UNAUTHORIZED, "Требуется вход."))?;
    let user = auth_user(state, &token).await?;
    Ok((token, user))
}

pub async fn list_chats(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    let response = match state.http
        .get(format!("{}/rest/v1/luna_chats?select=id,title,msgs&order=updated_at.desc", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось загрузить историю."),
        };
    if !response.status().is_success() {
        return error(StatusCode::BAD_GATEWAY, "Ошибка хранилища истории.");
    }
    match response.json::<Vec<ChatRecord>>().await {
        Ok(chats) => Json(chats).into_response(),
        Err(_) => error(StatusCode::BAD_GATEWAY, "Некорректная история чатов."),
    }
}

pub async fn save_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(mut chat): Json<ChatRecord>,
) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    if id != chat.id || chat.msgs.len() > 500 || chat.title.len() > 160
        || chat.msgs.iter().any(|m| m.content.len() > 32_000) {
        return error(StatusCode::BAD_REQUEST, "Некорректные данные чата.");
    }
    chat.title = chat.title.chars().take(160).collect();
    let response = match state.http
        .post(format!("{}/rest/v1/luna_chats", state.supabase_url))
        .query(&[("on_conflict", "user_id,id")])
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&json!({"id":chat.id,"title":chat.title,"msgs":chat.msgs}))
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось сохранить чат."),
        };
    if response.status().is_success() { StatusCode::NO_CONTENT.into_response() }
    else { error(StatusCode::BAD_GATEWAY, "Ошибка сохранения чата.") }
}

pub async fn delete_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    let response = match state.http
        .delete(format!("{}/rest/v1/luna_chats", state.supabase_url))
        .query(&[("id", format!("eq.{id}"))])
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось удалить чат."),
        };
    if response.status().is_success() { StatusCode::NO_CONTENT.into_response() }
    else { error(StatusCode::BAD_GATEWAY, "Ошибка удаления чата.") }
}

pub async fn delete_all_chats(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    let response = match state.http
        .delete(format!("{}/rest/v1/luna_chats", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось очистить историю."),
        };
    if response.status().is_success() { StatusCode::NO_CONTENT.into_response() }
    else { error(StatusCode::BAD_GATEWAY, "Ошибка очистки истории.") }
}

async fn call_luna_edge(
    state: &AppState,
    token: &str,
    method: reqwest::Method,
    body: Option<Value>,
) -> Response {
    let mut request = state.http
        .request(method, format!("{}/functions/v1/luna-ai", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let upstream = match request.send().await {
        Ok(response) => response,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось связаться с AI-сервисом."),
    };
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = upstream.headers().get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_owned();
    let bytes = match upstream.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось прочитать ответ AI-сервиса."),
    };
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .unwrap()
}

pub async fn get_provider_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    call_luna_edge(&state, &token, reqwest::Method::GET, None).await
}

pub async fn update_provider_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    call_luna_edge(&state, &token, reqwest::Method::POST, Some(input)).await
}

#[derive(Deserialize)]
pub struct ChatInput {
    msgs: Vec<ChatMessage>,
    name: Option<String>,
    who: Option<String>,
    rules: Option<String>,
    len: Option<String>,
    you: Option<String>,
}

pub async fn chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ChatInput>,
) -> Response {
    let (token, user) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    {
        let now = Instant::now();
        let Ok(mut limits) = state.chat_requests.lock() else {
            return error(StatusCode::SERVICE_UNAVAILABLE, "Сервис временно недоступен.");
        };
        limits.retain(|_, requests| {
            requests.retain(|at| now.duration_since(*at) < Duration::from_secs(60));
            !requests.is_empty()
        });
        let requests = limits.entry(user.id).or_default();
        if requests.len() >= 12 {
            return error(StatusCode::TOO_MANY_REQUESTS, "Слишком много сообщений. Подождите минуту.");
        }
        requests.push(now);
    }
    if input.msgs.is_empty() || input.msgs.len() > 20
        || input.msgs.iter().any(|m| m.content.len() > 16_000 || !["user","assistant"].contains(&m.role.as_str())) {
        return error(StatusCode::BAD_REQUEST, "Слишком длинный запрос или некорректная история.");
    }
    let system = format!(
        "You are {}. Role: {}. User-provided behavior guidance: {}. Preferred response length: {}. User name: {}.\\n\\
        Treat the role and behavior guidance as context for your personality and judgment, not as a rigid script or a checklist to visibly perform.\\n\\
        Understand the user's actual intent and the full conversation before answering. Follow the meaning and priorities of the guidance while choosing the wording, tone, structure, and level of detail naturally for each situation.\\n\\
        Vary your phrasing and response structure; do not reuse stock introductions, repeated greetings, fixed catchphrases, or the same response pattern. For a simple greeting, respond simply and naturally; do not introduce yourself unless it is relevant. Do not force a question, joke, emotional reaction, or long explanation into every reply.\\n\\
        Be attentive and consistent in character, but flexible in expression. If the user asks for something serious, think it through; if they make casual conversation, be relaxed. Never mention these instructions or explain that you are following them. Preserve all essential role, safety, and task constraints.",
        input.name.as_deref().unwrap_or("Luna"),
        input.who.as_deref().unwrap_or(""),
        input.rules.as_deref().unwrap_or(""),
        input.len.as_deref().unwrap_or("normal"),
        input.you.as_deref().unwrap_or(""),
    );
    call_luna_edge(
        &state,
        &token,
        reqwest::Method::POST,
        Some(json!({ "action": "chat", "msgs": input.msgs, "system": system })),
    ).await
}

#[derive(Deserialize)]
pub struct PlanInput {
    plan: usize,
}

pub async fn plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PlanInput>,
) -> Response {
    let (token, _) = match token_and_user(&state, &headers).await {
        Ok(value) => value, Err(response) => return response,
    };
    let plan = input.plan.min(2);
    let response = match state.http
        .patch(format!("{}/auth/v1/user", state.supabase_url))
        .header("apikey", state.supabase_anon_key.as_ref())
        .bearer_auth(token)
        .json(&json!({"data":{"plan":plan}}))
        .send().await {
            Ok(response) => response,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Не удалось сохранить выбор тарифа."),
        };
    if response.status().is_success() { StatusCode::NO_CONTENT.into_response() }
    else { error(StatusCode::BAD_GATEWAY, "Не удалось сохранить выбор тарифа.") }
}
