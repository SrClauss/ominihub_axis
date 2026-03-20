use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{
    handlers::{auth, coverage, hubs, roaming, ws},
    AppState,
};

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/coverage/map", get(coverage::get_coverage_map))
        .route("/v1/coverage/version", get(coverage::get_coverage_version))
        .route("/v1/coverage/validate", post(coverage::validate_coverage))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/verify", get(auth::verify_token))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/public-key", get(auth::public_key))
        .route("/hubs/register", post(hubs::register_hub))
        .route("/hubs", get(hubs::list_hubs))
        .route("/hubs/:id/status", get(hubs::hub_status))
        .route("/hubs/:id/heartbeat", put(hubs::heartbeat))
        .route("/hubs/:id/boundary", put(hubs::update_boundary))
        .route("/hubs/:id/contains", post(hubs::check_hub_contains_location))
        .route("/roaming/validate", post(roaming::validate_roaming))
        .route("/v1/stream", get(ws::ws_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok", "service": "axis-core"}))
}
