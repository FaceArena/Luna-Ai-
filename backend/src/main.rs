mod providers;

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

async fn health() -> Json<Value> {
    Json(json!({
        "service": "luna-server",
        "status": "ok",
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

    let app = Router::new()
        .route("/health", get(health))
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
