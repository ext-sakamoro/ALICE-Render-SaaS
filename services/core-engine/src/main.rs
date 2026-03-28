use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Debug, Default, Serialize)]
struct Stats {
    frames_rendered: u64,
    scenes_uploaded: u64,
    materials_compiled: u64,
    preset_queries: u64,
    total_requests: u64,
}

type AppState = Arc<Mutex<Stats>>;

// --- request / response types ---

#[derive(Debug, Deserialize)]
struct RenderFrameRequest {
    scene_id: String,
    width: u32,
    height: u32,
    samples: Option<u32>,
    preset: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SceneRequest {
    scene_id: String,
    graph: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct MaterialRequest {
    name: String,
    base_color: Option<[f32; 3]>,
    metallic: Option<f32>,
    roughness: Option<f32>,
    emission: Option<[f32; 3]>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    ok: bool,
    request_id: String,
    data: T,
}

fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        ok: true,
        request_id: Uuid::new_v4().to_string(),
        data,
    })
}

// --- handlers ---

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "alice-render-saas-core" }))
}

async fn render_frame(
    State(state): State<AppState>,
    Json(req): Json<RenderFrameRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let mut s = state.lock().unwrap();
    s.frames_rendered += 1;
    s.total_requests += 1;
    let samples = req.samples.unwrap_or(128);
    let preset = req.preset.unwrap_or_else(|| "studio".to_string());
    (
        StatusCode::OK,
        ok(serde_json::json!({
            "scene_id": req.scene_id,
            "width": req.width,
            "height": req.height,
            "samples": samples,
            "preset": preset,
            "frame_id": Uuid::new_v4().to_string(),
            "render_time_ms": 4200,
            "download_url": format!("/frames/{}.exr", Uuid::new_v4()),
        })),
    )
}

async fn render_scene(
    State(state): State<AppState>,
    Json(req): Json<SceneRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let mut s = state.lock().unwrap();
    s.scenes_uploaded += 1;
    s.total_requests += 1;
    (
        StatusCode::OK,
        ok(serde_json::json!({
            "scene_id": req.scene_id,
            "objects": req.graph.get("objects").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            "lights": req.graph.get("lights").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            "status": "uploaded",
        })),
    )
}

async fn render_material(
    State(state): State<AppState>,
    Json(req): Json<MaterialRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let mut s = state.lock().unwrap();
    s.materials_compiled += 1;
    s.total_requests += 1;
    (
        StatusCode::OK,
        ok(serde_json::json!({
            "name": req.name,
            "material_id": Uuid::new_v4().to_string(),
            "metallic": req.metallic.unwrap_or(0.0),
            "roughness": req.roughness.unwrap_or(0.5),
            "compiled": true,
        })),
    )
}

async fn render_presets(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let mut s = state.lock().unwrap();
    s.preset_queries += 1;
    s.total_requests += 1;
    (
        StatusCode::OK,
        ok(serde_json::json!({
            "presets": ["studio", "outdoor", "product", "cinematic", "hdri-sky", "night"],
        })),
    )
}

async fn render_stats(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let s = state.lock().unwrap();
    ok(serde_json::json!({
        "frames_rendered": s.frames_rendered,
        "scenes_uploaded": s.scenes_uploaded,
        "materials_compiled": s.materials_compiled,
        "preset_queries": s.preset_queries,
        "total_requests": s.total_requests,
    }))
}

// --- main ---

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state: AppState = Arc::new(Mutex::new(Stats::default()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/render/frame", post(render_frame))
        .route("/api/v1/render/scene", post(render_scene))
        .route("/api/v1/render/material", post(render_material))
        .route("/api/v1/render/presets", get(render_presets))
        .route("/api/v1/render/stats", get(render_stats))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8120".to_string())
        .parse()
        .unwrap_or(8120);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("alice-render-saas-core listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
