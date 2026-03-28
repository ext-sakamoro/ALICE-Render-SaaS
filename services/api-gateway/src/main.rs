use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{any, get},
    Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

struct GatewayState {
    client: reqwest::Client,
    upstream: String,
    request_count: std::sync::atomic::AtomicU64,
    routes: DashMap<String, String>,
}

type AppState = Arc<GatewayState>;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    upstream: String,
    total_requests: u64,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "alice-render-saas-gateway",
        upstream: state.upstream.clone(),
        total_requests: state
            .request_count
            .load(std::sync::atomic::Ordering::Relaxed),
    })
}

async fn proxy(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    state
        .request_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    let url = format!("{}{}{}", state.upstream, path, query);

    let mut upstream_req = state.client.request(method, &url).body(body_bytes);
    for (k, v) in &headers {
        if k != axum::http::header::HOST {
            upstream_req = upstream_req.header(k, v);
        }
    }

    match upstream_req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = resp.bytes().await.unwrap_or_default();
            Ok((status, body).into_response())
        }
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        )),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let upstream = std::env::var("UPSTREAM_URL")
        .unwrap_or_else(|_| "http://localhost:8120".to_string());

    let routes: DashMap<String, String> = DashMap::new();
    routes.insert("/api/v1/render".to_string(), upstream.clone());

    let state = Arc::new(GatewayState {
        client: reqwest::Client::new(),
        upstream,
        request_count: std::sync::atomic::AtomicU64::new(0),
        routes,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/{*path}", any(proxy))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8220".to_string())
        .parse()
        .unwrap_or(8220);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("alice-render-saas-gateway listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
