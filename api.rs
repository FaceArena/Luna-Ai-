use crate::state::{Chat, Cfg, Me, Memory};
use gloo_net::http::Request;

pub async fn me() -> Option<Me> {
    let r = Request::get("/api/me").send().await.ok()?;
    if !r.ok() {
        return None;
    }
    r.json().await.ok()
}

pub async fn get_personal_settings() -> Result<Option<Cfg>, ()> {
    let response = Request::get("/api/personal-settings").send().await.map_err(|_| ())?;
    if !response.ok() { return Err(()); }
    response.json().await.map_err(|_| ())
}

pub async fn put_personal_settings(cfg: &Cfg) -> bool {
    match Request::put("/api/personal-settings").json(cfg) {
        Ok(request) => request.send().await.map(|r| r.ok()).unwrap_or(false),
        Err(_) => false,
    }
}

pub async fn logout() {
    let _ = Request::post("/api/logout").send().await;
}


pub async fn memories() -> Result<Vec<Memory>, ()> {
    let response = Request::get("/api/memories").send().await.map_err(|_| ())?;
    if !response.ok() { return Err(()); }
    response.json().await.map_err(|_| ())
}

pub async fn update_memory(id: &str, memory: &serde_json::Value) -> Result<Memory, ()> {
    let response = Request::put(&format!("/api/memories/{id}"))
        .json(memory).map_err(|_| ())?.send().await.map_err(|_| ())?;
    if !response.ok() { return Err(()); }
    response.json().await.map_err(|_| ())
}

pub async fn delete_memory(id: &str) -> bool {
    Request::delete(&format!("/api/memories/{id}")).send().await.map(|r| r.ok()).unwrap_or(false)
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

pub async fn set_plan(plan: usize) -> Result<(), String> {
    let request = Request::put("/api/plan")
        .json(&serde_json::json!({ "plan": plan }))
        .map_err(|_| "Не удалось подготовить запрос.".to_string())?;
    let response = request.send().await
        .map_err(|_| "Не удалось связаться с сервером.".to_string())?;
    if response.ok() {
        Ok(())
    } else {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {detail}"))
    }
}
