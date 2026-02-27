use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    reply: String,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: String,
    status: &'static str,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = AppState {
        name: Arc::new("AI-OS Core Daemon".to_string()),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/chat", post(chat))
        .route("/v1/events", get(events))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:7777".parse().expect("valid socket address");
    info!("core-daemon listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");

    axum::serve(listener, app).await.expect("start server");
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        service: state.name.to_string(),
        status: "ok",
    })
}

async fn chat(Json(req): Json<ChatRequest>) -> impl IntoResponse {
    Json(ChatResponse {
        reply: format!(
            "Sprint 1 scaffold active. Received: '{}'. Tool execution will be added next.",
            req.message
        ),
        mode: "mock",
    })
}

async fn events(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let _ = socket
            .send(axum::extract::ws::Message::Text(
                "event:connected".to_string().into(),
            ))
            .await;

        loop {
            sleep(Duration::from_secs(15)).await;
            if socket
                .send(axum::extract::ws::Message::Text("event:heartbeat".to_string().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}
