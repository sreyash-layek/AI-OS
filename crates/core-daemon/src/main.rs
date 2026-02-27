mod index_store;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::Path as StdPath,
    sync::Arc,
    time::UNIX_EPOCH,
};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
    time::{sleep, Duration},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

use crate::index_store::IndexStore;

use core_daemon::classify_tool_preview;
use core_daemon::speech;
use core_daemon::types::{
    ChatRequest, ChatResponse, CreateIndexScopeRequest, DeleteScopeResponse, EventEnvelope,
    HealthResponse, IndexEventType, IndexScope, IndexScopesResponse, IngestIndexEventRequest,
    IngestIndexEventResponse, SearchQuery, SearchResponse, SpeakRequest, SpeakResponse,
    UpdateIndexScopeRequest, UpdateIndexScopeResponse, UpdateVoiceSettingsRequest,
    VoiceProviderHealth, VoiceSettings,
};

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
    events_tx: broadcast::Sender<EventEnvelope>,
    voice_settings: Arc<Mutex<VoiceSettings>>,
    active_speech_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    index_store: Arc<IndexStore>,
    watcher_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let (events_tx, _) = broadcast::channel::<EventEnvelope>(128);

    let db_path = std::env::var("AIOS_INDEX_DB").unwrap_or_else(|_| "./.aios/index.db".to_string());
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let index_store = IndexStore::new(db_path).expect("initialize index store");

    let state = AppState {
        name: Arc::new("AI-OS Core Daemon".to_string()),
        events_tx,
        voice_settings: Arc::new(Mutex::new(VoiceSettings {
            provider: "mock".to_string(),
            auto_speak: false,
            default_voice: "system-default".to_string(),
        })),
        active_speech_task: Arc::new(Mutex::new(None)),
        index_store: Arc::new(index_store),
        watcher_tasks: Arc::new(Mutex::new(HashMap::new())),
    };

    start_watchers_for_existing_scopes(&state).await;

    let app = app_router(state);

    let addr: SocketAddr = "0.0.0.0:7777".parse().expect("valid socket address");
    info!("core-daemon listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");

    axum::serve(listener, app).await.expect("start server");
}

fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/chat", post(chat))
        .route("/v1/events", get(events))
        .route("/v1/speak", post(speak))
        .route("/v1/speak/stop", post(stop_speak))
        .route(
            "/v1/config/voice",
            get(get_voice_config).post(update_voice_config),
        )
        .route("/v1/config/voice/health", get(voice_health))
        .route(
            "/v1/index/scopes",
            get(list_index_scopes).post(create_index_scope),
        )
        .route(
            "/v1/index/scopes/{id}",
            delete(delete_index_scope).patch(update_index_scope),
        )
        .route("/v1/index/events", post(ingest_index_event))
        .route("/v1/search", get(search_scopes))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn start_watchers_for_existing_scopes(state: &AppState) {
    let scopes = state.index_store.list_scopes().unwrap_or_default();
    for scope in scopes.into_iter().filter(|s| s.enabled) {
        start_scope_watcher(state.clone(), scope).await;
    }
}

async fn start_scope_watcher(state: AppState, scope: IndexScope) {
    let interval_secs = std::env::var("AIOS_WATCH_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v >= 1 && *v <= 60)
        .unwrap_or(5);
    let mut tasks = state.watcher_tasks.lock().await;
    if tasks.contains_key(&scope.id) {
        return;
    }

    let scope_id = scope.id.clone();
    let scope_path = scope.path.clone();
    let tx = state.events_tx.clone();
    let store = state.index_store.clone();

    let handle = tokio::spawn(async move {
        let mut previous: HashMap<String, (i64, String)> = HashMap::new();

        loop {
            let scan = tokio::task::spawn_blocking({
                let root = scope_path.clone();
                move || scan_scope_files(&root)
            })
            .await;

            let Ok(current_files) = scan else {
                sleep(Duration::from_secs(interval_secs)).await;
                continue;
            };

            let mut current: HashMap<String, (i64, String)> = HashMap::new();
            for (path, size, mtime) in current_files {
                current.insert(path, (size, mtime));
            }

            let mut creates: Vec<(String, i64, String)> = Vec::new();
            let mut updates: Vec<(String, i64, String)> = Vec::new();
            let mut deletes: Vec<String> = Vec::new();

            for (path, (size, mtime)) in &current {
                match previous.get(path) {
                    None => creates.push((path.clone(), *size, mtime.clone())),
                    Some((prev_size, prev_mtime)) if prev_size != size || prev_mtime != mtime => {
                        updates.push((path.clone(), *size, mtime.clone()))
                    }
                    _ => {}
                }
            }

            for old_path in previous.keys() {
                if !current.contains_key(old_path) {
                    deletes.push(old_path.clone());
                }
            }

            // Heuristic rename detection: same (size, mtime) appears as delete+create in same poll batch.
            let mut deleted_by_sig: HashMap<(i64, String), Vec<String>> = HashMap::new();
            for old_path in &deletes {
                if let Some((size, mtime)) = previous.get(old_path) {
                    deleted_by_sig
                        .entry((*size, mtime.clone()))
                        .or_default()
                        .push(old_path.clone());
                }
            }

            let mut renames: Vec<(String, String, i64, String)> = Vec::new();
            let mut remaining_creates: Vec<(String, i64, String)> = Vec::new();
            for (new_path, size, mtime) in creates {
                let sig = (size, mtime.clone());
                if let Some(candidates) = deleted_by_sig.get_mut(&sig) {
                    if let Some(old_path) = candidates.pop() {
                        renames.push((old_path, new_path, size, mtime));
                        continue;
                    }
                }
                remaining_creates.push((new_path, size, mtime));
            }

            let renamed_from = renames
                .iter()
                .map(|(old_path, _, _, _)| old_path.clone())
                .collect::<HashSet<_>>();
            let remaining_deletes = deletes
                .into_iter()
                .filter(|path| !renamed_from.contains(path))
                .collect::<Vec<_>>();

            for (old_path, new_path, size, mtime) in &renames {
                let _ = store.rename_file_metadata(
                    old_path,
                    new_path,
                    &scope_id,
                    Some(*size),
                    Some(mtime),
                );
            }
            for (path, size, mtime) in &remaining_creates {
                let _ = store.upsert_file_metadata(&scope_id, path, Some(*size), Some(mtime));
            }
            for (path, size, mtime) in &updates {
                let _ = store.upsert_file_metadata(&scope_id, path, Some(*size), Some(mtime));
            }
            for old_path in &remaining_deletes {
                let _ = store.delete_file_metadata(old_path);
            }

            // Debounced batch event (single websocket message per polling cycle)
            let total_changes =
                renames.len() + remaining_creates.len() + updates.len() + remaining_deletes.len();
            if total_changes > 0 {
                let sample_paths = remaining_creates
                    .iter()
                    .map(|(p, _, _)| p.clone())
                    .chain(updates.iter().map(|(p, _, _)| p.clone()))
                    .chain(remaining_deletes.iter().cloned())
                    .take(10)
                    .collect::<Vec<_>>();

                let _ = tx.send(EventEnvelope {
                    event: "index_batch_applied".to_string(),
                    at: chrono::Utc::now().to_rfc3339(),
                    source: "core-daemon",
                    data: serde_json::json!({
                        "scope_id": scope_id.clone(),
                        "counts": {
                            "create": remaining_creates.len(),
                            "update": updates.len(),
                            "delete": remaining_deletes.len(),
                            "rename": renames.len()
                        },
                        "sample_paths": sample_paths,
                    }),
                });
            }

            previous = current;
            sleep(Duration::from_secs(interval_secs)).await;
        }
    });

    tasks.insert(scope.id, handle);
}

async fn stop_scope_watcher(state: &AppState, scope_id: &str) {
    if let Some(handle) = state.watcher_tasks.lock().await.remove(scope_id) {
        handle.abort();
    }
}

fn should_index_file(path: &StdPath) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    let lower = name.to_lowercase();
    !(lower.starts_with(".~")
        || lower.starts_with("~$")
        || lower.ends_with(".tmp")
        || lower.ends_with(".swp")
        || lower.ends_with(".part")
        || lower == ".ds_store"
        || lower == "thumbs.db")
}

fn scan_scope_files(root: &str) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    let root_path = StdPath::new(root);
    if !root_path.exists() {
        return out;
    }

    let max_files = std::env::var("AIOS_WATCH_MAX_FILES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v >= 100)
        .unwrap_or(20_000);

    fn walk(dir: &StdPath, out: &mut Vec<(String, i64, String)>, max_files: usize) {
        if out.len() >= max_files {
            return;
        }

        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            if out.len() >= max_files {
                break;
            }

            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };

            if meta.is_dir() {
                walk(&path, out, max_files);
            } else if meta.is_file() {
                if !should_index_file(&path) {
                    continue;
                }
                let size = meta.len() as i64;
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_else(|| "0".to_string());
                out.push((path.to_string_lossy().to_string(), size, mtime));
            }
        }
    }

    walk(root_path, &mut out, max_files);
    out
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
    let tool_preview = classify_tool_preview(&req.message);

    let reply = format!(
        "Sprint scaffold active. Received: '{}'. Tool execution will be added next.",
        req.message
    );

    let cfg = state.voice_settings.lock().await.clone();
    if cfg.auto_speak && !reply.trim().is_empty() {
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

async fn list_index_scopes(State(state): State<AppState>) -> impl IntoResponse {
    let scopes = state.index_store.list_scopes().unwrap_or_default();
    Json(IndexScopesResponse { scopes })
}

async fn create_index_scope(
    State(state): State<AppState>,
    Json(req): Json<CreateIndexScopeRequest>,
) -> impl IntoResponse {
    let path = req.path.trim().to_string();
    if path.is_empty() {
        return Json(IndexScopesResponse { scopes: Vec::new() });
    }

    let existing = state.index_store.list_scopes().unwrap_or_default();
    if existing
        .iter()
        .any(|s| s.path.eq_ignore_ascii_case(&path) || s.path == path)
    {
        return Json(IndexScopesResponse { scopes: existing });
    }

    let scope = IndexScope {
        id: Uuid::new_v4().to_string(),
        path,
        enabled: req.enabled.unwrap_or(true),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = state.index_store.create_scope(&scope);
    if scope.enabled {
        start_scope_watcher(state.clone(), scope.clone()).await;
    }

    let _ = state.events_tx.send(EventEnvelope {
        event: "index_scope_added".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "id": scope.id,
          "path": scope.path,
          "enabled": scope.enabled
        }),
    });

    Json(IndexScopesResponse {
        scopes: state.index_store.list_scopes().unwrap_or_default(),
    })
}

async fn delete_index_scope(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let deleted = state.index_store.delete_scope(&id).unwrap_or(false);

    if deleted {
        stop_scope_watcher(&state, &id).await;

        let _ = state.events_tx.send(EventEnvelope {
            event: "index_scope_removed".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({ "id": id }),
        });
    }

    Json(DeleteScopeResponse {
        ok: true,
        deleted_id: if deleted { Some(id) } else { None },
    })
}

async fn update_index_scope(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateIndexScopeRequest>,
) -> impl IntoResponse {
    let scope = state
        .index_store
        .update_scope_enabled(&id, req.enabled)
        .unwrap_or(None);

    if let Some(scope) = &scope {
        if scope.enabled {
            start_scope_watcher(state.clone(), scope.clone()).await;
        } else {
            stop_scope_watcher(&state, &scope.id).await;
        }

        let _ = state.events_tx.send(EventEnvelope {
            event: "index_scope_updated".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({
                "id": scope.id,
                "enabled": scope.enabled,
                "path": scope.path
            }),
        });
    }

    Json(UpdateIndexScopeResponse { ok: true, scope })
}

async fn ingest_index_event(
    State(state): State<AppState>,
    Json(req): Json<IngestIndexEventRequest>,
) -> impl IntoResponse {
    let path = req.path.trim().to_string();
    if path.is_empty() {
        return Json(IngestIndexEventResponse { ok: false });
    }

    let ok = match req.event_type {
        IndexEventType::Create | IndexEventType::Update => state
            .index_store
            .upsert_file_metadata(&req.scope_id, &path, req.size_bytes, req.mtime.as_deref())
            .is_ok(),
        IndexEventType::Delete => state.index_store.delete_file_metadata(&path).is_ok(),
        IndexEventType::Rename => {
            if let Some(old_path) = req.renamed_from.as_deref() {
                state
                    .index_store
                    .rename_file_metadata(
                        old_path,
                        &path,
                        &req.scope_id,
                        req.size_bytes,
                        req.mtime.as_deref(),
                    )
                    .is_ok()
            } else {
                false
            }
        }
    };

    if ok {
        let _ = state.events_tx.send(EventEnvelope {
            event: "index_event_applied".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            source: "core-daemon",
            data: serde_json::json!({
                "scope_id": req.scope_id,
                "path": path,
                "event_type": format!("{:?}", req.event_type).to_lowercase(),
            }),
        });
    }

    Json(IngestIndexEventResponse { ok })
}

async fn search_scopes(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = query.q.trim().to_string();
    let results = if q.is_empty() {
        Vec::new()
    } else {
        state.index_store.search_scope_paths(&q).unwrap_or_default()
    };

    Json(SearchResponse {
        query: query.q,
        results,
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
    let text = req.text.trim().to_string();
    let voice = req.voice.clone().unwrap_or(cfg.default_voice);

    if text.is_empty() {
        return SpeakResponse {
            ok: false,
            request_id,
            mode: "rejected_empty_text",
        };
    }

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
          "text": text,
          "provider": effective_provider,
          "configured_provider": configured_provider,
          "provider_detail": detail
        }),
    };
    let _ = state.events_tx.send(started);

    let tx = state.events_tx.clone();
    let rid = request_id.clone();
    let text = text;
    let voice_clone = voice.clone();
    let provider_clone = effective_provider.clone();

    let task = tokio::spawn(async move {
        let stop_event = if provider_clone == "system" {
            speech::run_system_speech(rid, text, voice_clone).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, Router};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::util::ServiceExt;

    fn test_state() -> AppState {
        let (events_tx, _) = broadcast::channel::<EventEnvelope>(32);
        let db_file = tempfile::NamedTempFile::new().expect("temp db file");
        let db_path = db_file.path().to_string_lossy().to_string();
        drop(db_file);
        let store = IndexStore::new(db_path).expect("init temp index store");

        AppState {
            name: Arc::new("AI-OS Core Daemon".to_string()),
            events_tx,
            voice_settings: Arc::new(Mutex::new(VoiceSettings {
                provider: "mock".to_string(),
                auto_speak: false,
                default_voice: "system-default".to_string(),
            })),
            active_speech_task: Arc::new(Mutex::new(None)),
            index_store: Arc::new(store),
            watcher_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn test_app() -> Router {
        app_router(test_state())
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let body = resp.into_body().collect().await.expect("body").to_bytes();
        serde_json::from_slice(&body).expect("valid json")
    }

    fn post_json(path: &str, payload: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "AI-OS Core Daemon");
    }

    #[tokio::test]
    async fn chat_endpoint_returns_tool_preview() {
        let app = test_app();
        let resp = app
            .oneshot(post_json(
                "/v1/chat",
                json!({ "message": "open downloads" }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let json = body_json(resp).await;
        assert_eq!(json["tool_preview"]["name"], "open_app_or_file");
    }

    #[tokio::test]
    async fn voice_config_get_and_update_work() {
        let app = test_app();

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/config/voice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), 200);
        let before = body_json(get_resp).await;
        assert_eq!(before["auto_speak"], false);

        let update_resp = app
            .clone()
            .oneshot(post_json(
                "/v1/config/voice",
                json!({ "auto_speak": true, "default_voice": "nova" }),
            ))
            .await
            .unwrap();
        assert_eq!(update_resp.status(), 200);
        let updated = body_json(update_resp).await;
        assert_eq!(updated["auto_speak"], true);
        assert_eq!(updated["default_voice"], "nova");
    }

    #[tokio::test]
    async fn voice_health_endpoint_returns_provider_resolution() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/config/voice/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let json = body_json(resp).await;
        assert!(json["configured_provider"].is_string());
        assert!(json["effective_provider"].is_string());
        assert!(json["available"].is_boolean());
        assert!(json["detail"].is_string());
    }

    #[tokio::test]
    async fn speak_endpoint_accepts_text() {
        let app = test_app();
        let resp = app
            .oneshot(post_json(
                "/v1/speak",
                json!({ "text": "Hello AI-OS", "voice": "system-default" }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let json = body_json(resp).await;
        assert_eq!(json["ok"], true);
        assert!(json["request_id"].is_string());
        assert!(json["mode"].is_string());
    }

    #[tokio::test]
    async fn enqueue_speech_rejects_empty_text() {
        let state = test_state();
        let result = enqueue_speech(
            state,
            SpeakRequest {
                text: "   ".to_string(),
                voice: None,
            },
        )
        .await;

        assert!(!result.ok);
        assert_eq!(result.mode, "rejected_empty_text");
    }

    #[tokio::test]
    async fn index_scope_create_list_delete_flow() {
        let app = test_app();

        let create_resp = app
            .clone()
            .oneshot(post_json(
                "/v1/index/scopes",
                json!({ "path": "C:/Users/sreya/Documents", "enabled": true }),
            ))
            .await
            .unwrap();
        assert_eq!(create_resp.status(), 200);
        let created = body_json(create_resp).await;
        assert_eq!(created["scopes"].as_array().unwrap().len(), 1);

        let scope_id = created["scopes"][0]["id"].as_str().unwrap().to_string();

        let list_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/index/scopes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), 200);
        let listed = body_json(list_resp).await;
        assert_eq!(listed["scopes"].as_array().unwrap().len(), 1);

        let ingest_create_resp = app
            .clone()
            .oneshot(post_json(
                "/v1/index/events",
                json!({
                    "scope_id": scope_id,
                    "path": "C:/Users/sreya/Documents/notes/todo.md",
                    "event_type": "create",
                    "size_bytes": 120,
                    "mtime": "2026-02-28T00:00:00Z"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(ingest_create_resp.status(), 200);
        let ingest_create_json = body_json(ingest_create_resp).await;
        assert_eq!(ingest_create_json["ok"], true);

        let search_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/search?q=todo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(search_resp.status(), 200);
        let searched = body_json(search_resp).await;
        assert_eq!(searched["results"].as_array().unwrap().len(), 1);

        let ingest_rename_resp = app
            .clone()
            .oneshot(post_json(
                "/v1/index/events",
                json!({
                    "scope_id": created["scopes"][0]["id"].as_str().unwrap(),
                    "path": "C:/Users/sreya/Documents/notes/todo-renamed.md",
                    "event_type": "rename",
                    "renamed_from": "C:/Users/sreya/Documents/notes/todo.md"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(ingest_rename_resp.status(), 200);
        let ingest_rename_json = body_json(ingest_rename_resp).await;
        assert_eq!(ingest_rename_json["ok"], true);

        let renamed_search_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/search?q=renamed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(renamed_search_resp.status(), 200);
        let renamed_search_json = body_json(renamed_search_resp).await;
        assert_eq!(renamed_search_json["results"].as_array().unwrap().len(), 1);

        let ingest_delete_resp = app
            .clone()
            .oneshot(post_json(
                "/v1/index/events",
                json!({
                    "scope_id": created["scopes"][0]["id"].as_str().unwrap(),
                    "path": "C:/Users/sreya/Documents/notes/todo-renamed.md",
                    "event_type": "delete"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(ingest_delete_resp.status(), 200);
        let ingest_delete_json = body_json(ingest_delete_resp).await;
        assert_eq!(ingest_delete_json["ok"], true);

        let delete_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/v1/index/scopes/{}",
                        created["scopes"][0]["id"].as_str().unwrap()
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_resp.status(), 200);
        let deleted = body_json(delete_resp).await;
        assert_eq!(deleted["ok"], true);
        assert!(deleted["deleted_id"].is_string());
    }

    #[tokio::test]
    async fn stop_speak_returns_ok() {
        let state = test_state();
        let resp = stop_speak(State(state)).await.into_response();
        assert_eq!(resp.status(), 200);
        let json = body_json(resp).await;
        assert_eq!(json["ok"], true);
    }
}
