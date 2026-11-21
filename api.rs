use crate::state::{Chat, Me};
use gloo_net::http::Request;
use serde::Serialize;

pub async fn me() -> Option<Me> {
    let r = Request::get("/api/me").send().await.ok()?;
    if !r.ok() {
        return None;
    }
    r.json().await.ok()
}

#[derive(Deserialize)]
struct PendingConfirmation {
    email: String,
}

pub enum AuthOutcome {
    SignedIn(Me),
    NeedsEmailConfirmation(String),
}

/// Единый контракт для входа и регистрации.
pub async fn auth<T: Serialize>(path: &str, body: &T) -> Result<AuthOutcome, String> {
    let r = Request::post(path)
        .json(body)
        .map_err(|_| "Не удалось подготовить запрос".to_string())?
        .send()
        .await
        .map_err(|_| "Нет связи с сервером. Проверьте интернет и повторите попытку.".to_string())?;

    let status = r.status();
    let body = r.text().await.unwrap_or_default();
    if status == 202 {
        let pending: PendingConfirmation = serde_json::from_str(&body)
            .map_err(|_| "Сервис регистрации вернул некорректный ответ".to_string())?;
        return Ok(AuthOutcome::NeedsEmailConfirmation(pending.email));
    }
    if !(200..300).contains(&status) {
        let message = body.trim();
        return Err(if message.is_empty() {
            "Не удалось выполнить запрос. Попробуйте ещё раз.".into()
        } else {
            message.to_string()
        });
    }
    let me: Me = serde_json::from_str(&body)
        .map_err(|_| "Сервер авторизации вернул некорректные данные".to_string())?;
    Ok(AuthOutcome::SignedIn(me))
}

pub async fn logout() {
    let _ = Request::post("/api/logout").send().await;
}

pub async fn chats() -> Vec<Chat> {
    match Request::get("/api/chats").send().await {
        Ok(r) if r.ok() => r.json().await.unwrap_or_default(),
        _ => vec![],
    }
}

pub async fn put_chat(c: &Chat) {
    if let Ok(rq) = Request::put(&format!("/api/chats/{}", c.id)).json(c) {
        let _ = rq.send().await;
    }
}

pub async fn del_chat(id: i64) {
    let _ = Request::delete(&format!("/api/chats/{id}")).send().await;
}

pub async fn del_all() {
    let _ = Request::delete("/api/chats").send().await;
}

pub async fn set_plan(plan: usize) {
    if let Ok(rq) = Request::put("/api/plan").json(&serde_json::json!({ "plan": plan })) {
        let _ = rq.send().await;
    }
}
