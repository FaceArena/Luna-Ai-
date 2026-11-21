use crate::state::{Chat, Me};
use gloo_net::http::Request;

pub async fn me() -> Option<Me> {
    let r = Request::get("/api/me").send().await.ok()?;
    if !r.ok() {
        return None;
    }
    r.json().await.ok()
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
