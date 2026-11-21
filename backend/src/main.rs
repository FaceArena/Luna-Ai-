mod api;
mod providers;

use axum::{routing::{delete, get, post, put}, Json, Router};
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}, time::Instant};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
    pub supabase_url: Arc<str>,
    pub supabase_anon_key: Arc<str>,
    pub chat_requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

async fn health() -> Json<Value> {
    Json(json!({
        "service": "luna-server",
        "status": "ok",
        "api": ["auth", "chat-history", "chat"],
        "providers": ["groq", "gemini"]
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "luna_server=info,tower_http=info".into()),
        )
        .init();

    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be configured");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
        .expect("SUPABASE_ANON_KEY must be configured");
    let supabase_url = supabase_url.trim_end_matches('/').to_owned();

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent(concat!("Luna-AI/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to initialize HTTP client");

    let state = AppState {
        http,
        supabase_url: Arc::from(supabase_url),
        supabase_anon_key: Arc::from(supabase_anon_key),
        chat_requests: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/auth/telegram/exchange", post(api::telegram_exchange))
        .route("/api/register", post(api::register))
        .route("/api/login", post(api::login))
        .route("/api/me", get(api::current_user))
        .route("/api/logout", post(api::logout))
        .route("/api/chats", get(api::list_chats).delete(api::delete_all_chats))
        .route("/api/chats/:id", put(api::save_chat).delete(api::delete_chat))
        .route("/api/chat", post(api::chat))
        .route("/api/provider-settings", get(api::get_provider_settings).post(api::update_provider_settings))
        .route("/api/plan", put(api::plan))
        .with_state(state)
        .layer(axum::extract::DefaultBodyLimit::max(512 * 1024))
        .layer(TraceLayer::new_for_http());

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind Luna API listener");

    tracing::info!("Luna API listening on {}", address);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("Luna API server failed");
}
