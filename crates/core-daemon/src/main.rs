use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{sync::broadcast, time::{sleep, Duration}};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

mod types;
use types::{
    ChatRequest, ChatResponse, EventEnvelope, HealthResponse, SpeakRequest, SpeakResponse, ToolPreview,
};

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
    events_tx: broadcast::Sender<EventEnvelope>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let (events_tx, _) = broadcast::channel::<EventEnvelope>(128);

    let state = AppState {
        name: Arc::new("AI-OS Core Daemon".to_string()),
        events_tx,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/chat", post(chat))
        .route("/v1/events", get(events))
        .route("/v1/speak", post(speak))
        .route("/v1/speak/stop", post(stop_speak))
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
            "Sprint scaffold active. Received: '{}'. Tool execution will be added next.",
            req.message
        ),
        mode: "mock",
        tool_preview,
    })
}

async fn speak(State(state): State<AppState>, Json(req): Json<SpeakRequest>) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let tx = state.events_tx.clone();
    let voice = req.voice.clone().unwrap_or_else(|| "system-default".to_string());

    let started = EventEnvelope {
        event: "speech_started".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({ "request_id": request_id, "voice": voice, "text": req.text }),
    };
    let _ = tx.send(started);

    let rid = request_id.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        let _ = tx.send(EventEnvelope {
            event: "speech_stopped".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({ "request_id": rid, "reason": "mock_complete" }),
        });
    });

    Json(SpeakResponse {
        ok: true,
        request_id,
        mode: "mock",
    })
}

async fn stop_speak(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.events_tx.send(EventEnvelope {
        event: "speech_stopped".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({ "reason": "user_stop" }),
    });

    Json(serde_json::json!({ "ok": true }))
}

async fn events(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.events_tx.subscribe();

        let connected = EventEnvelope {
            event: "connected".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({}),
        };

        let _ = socket
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&connected)
                    .unwrap_or_else(|_| "{\"event\":\"connected\"}".to_string())
                    .into(),
            ))
            .await;

        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(15)) => {
                    let heartbeat = EventEnvelope {
                        event: "heartbeat".to_string(),
                        at: chrono::Utc::now().to_rfc3339(),
                        source: "core-daemon",
                        data: serde_json::json!({}),
                    };

                    if socket
                        .send(axum::extract::ws::Message::Text(
                            serde_json::to_string(&heartbeat)
                                .unwrap_or_else(|_| "{\"event\":\"heartbeat\"}".to_string())
                                .into(),
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                evt = rx.recv() => {
                    match evt {
                        Ok(event) => {
                            if socket
                                .send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&event)
                                        .unwrap_or_else(|_| "{\"event\":\"internal_error\"}".to_string())
                                        .into(),
                                ))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    })
}
