use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
    time::{sleep, Duration},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

mod speech;
mod types;
use types::{
    ChatRequest, ChatResponse, EventEnvelope, HealthResponse, SpeakRequest, SpeakResponse,
    ToolPreview, UpdateVoiceSettingsRequest, VoiceProviderHealth, VoiceSettings,
};

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
    events_tx: broadcast::Sender<EventEnvelope>,
    voice_settings: Arc<Mutex<VoiceSettings>>,
    active_speech_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let (events_tx, _) = broadcast::channel::<EventEnvelope>(128);

    let state = AppState {
        name: Arc::new("AI-OS Core Daemon".to_string()),
        events_tx,
        voice_settings: Arc::new(Mutex::new(VoiceSettings {
            provider: "mock".to_string(),
            auto_speak: false,
            default_voice: "system-default".to_string(),
        })),
        active_speech_task: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/chat", post(chat))
        .route("/v1/events", get(events))
        .route("/v1/speak", post(speak))
        .route("/v1/speak/stop", post(stop_speak))
        .route("/v1/config/voice", get(get_voice_config).post(update_voice_config))
        .route("/v1/config/voice/health", get(voice_health))
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

async fn get_voice_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.voice_settings.lock().await.clone();
    Json(cfg)
}

async fn voice_health(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.voice_settings.lock().await.clone();
    let (effective_provider, available, detail) = speech::detect_effective_provider(&cfg);

    Json(VoiceProviderHealth {
        configured_provider: cfg.provider,
        effective_provider,
        available,
        detail,
    })
}

async fn update_voice_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateVoiceSettingsRequest>,
) -> impl IntoResponse {
    let mut cfg = state.voice_settings.lock().await;

    if let Some(provider) = req.provider {
        cfg.provider = provider;
    }
    if let Some(auto_speak) = req.auto_speak {
        cfg.auto_speak = auto_speak;
    }
    if let Some(default_voice) = req.default_voice {
        cfg.default_voice = default_voice;
    }

    let updated = cfg.clone();

    let _ = state.events_tx.send(EventEnvelope {
        event: "voice_config_updated".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "provider": updated.provider,
          "auto_speak": updated.auto_speak,
          "default_voice": updated.default_voice
        }),
    });

    Json(updated)
}

async fn chat(State(state): State<AppState>, Json(req): Json<ChatRequest>) -> impl IntoResponse {
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

    let reply = format!(
        "Sprint scaffold active. Received: '{}'. Tool execution will be added next.",
        req.message
    );

    let cfg = state.voice_settings.lock().await.clone();
    if cfg.auto_speak {
        let _ = enqueue_speech(
            state.clone(),
            SpeakRequest {
                text: reply.clone(),
                voice: Some(cfg.default_voice),
            },
        )
        .await;
    }

    Json(ChatResponse {
        reply,
        mode: "mock",
        tool_preview,
    })
}

async fn speak(State(state): State<AppState>, Json(req): Json<SpeakRequest>) -> impl IntoResponse {
    let response = enqueue_speech(state, req).await;
    Json(response)
}

async fn enqueue_speech(state: AppState, req: SpeakRequest) -> SpeakResponse {
    let request_id = Uuid::new_v4().to_string();
    let cfg = state.voice_settings.lock().await.clone();
    let (effective_provider, _available, detail) = speech::detect_effective_provider(&cfg);
    let configured_provider = cfg.provider;
    let voice = req.voice.clone().unwrap_or(cfg.default_voice);

    // cancel any current active speech task before starting a new one
    if let Some(handle) = state.active_speech_task.lock().await.take() {
        handle.abort();
        let _ = state.events_tx.send(EventEnvelope {
            event: "speech_stopped".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({ "reason": "interrupted_by_new_request" }),
        });
    }

    let started = EventEnvelope {
        event: "speech_started".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "request_id": request_id,
          "voice": voice,
          "text": req.text,
          "provider": effective_provider,
          "configured_provider": configured_provider,
          "provider_detail": detail
        }),
    };
    let _ = state.events_tx.send(started);

    let tx = state.events_tx.clone();
    let rid = request_id.clone();
    let text = req.text;
    let voice_clone = voice.clone();
    let provider_clone = effective_provider.clone();

    let task = tokio::spawn(async move {
        let stop_event = if provider_clone == "system" {
            #[cfg(target_os = "linux")]
            {
                speech::run_linux_system_speech(rid, text, voice_clone).await
            }
            #[cfg(not(target_os = "linux"))]
            {
                speech::run_system_stub(rid, voice_clone).await
            }
        } else {
            speech::run_mock_speech(rid, text, voice_clone).await
        };

        let _ = tx.send(stop_event);
    });

    *state.active_speech_task.lock().await = Some(task);

    SpeakResponse {
        ok: true,
        request_id,
        mode: if effective_provider == "system" {
            "system"
        } else {
            "mock"
        },
    }
}

async fn stop_speak(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(handle) = state.active_speech_task.lock().await.take() {
        handle.abort();
    }

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
