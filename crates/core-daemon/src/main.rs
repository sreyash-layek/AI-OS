use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
}

mod types;
use types::{ChatRequest, ChatResponse, HealthResponse, ToolPreview};

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
    let normalized = req.message.to_lowercase();

    let tool_preview = if normalized.contains("open") {
        ToolPreview {
            name: "open_app_or_file".to_string(),
            risk_tier: 1,
            requires_confirmation: false,
            note: "Low-risk open action. Execution engine stub only in Sprint 1.".to_string(),
        }
    } else if normalized.contains("delete") || normalized.contains("remove") {
        ToolPreview {
            name: "delete_file".to_string(),
            risk_tier: 2,
            requires_confirmation: true,
            note: "High-risk action. Confirmation required (policy engine in later sprint)."
                .to_string(),
        }
    } else {
        ToolPreview {
            name: "search_files_semantic".to_string(),
            risk_tier: 0,
            requires_confirmation: false,
            note: "Read-only search action. Stub routing for now.".to_string(),
        }
    };

    Json(ChatResponse {
        reply: format!(
            "Sprint 1 scaffold active. Received: '{}'. Tool execution will be added next.",
            req.message
        ),
        mode: "mock",
        tool_preview,
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
