use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{
    handlers::{auth, coverage, franchise_payments, franchisee, hubs, roaming, webhooks},
    AppState,
};

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        // Coverage
        .route("/v1/coverage/map", get(coverage::get_coverage_map))
        .route("/v1/coverage/version", get(coverage::get_coverage_version))
        .route("/v1/coverage/validate", post(coverage::validate_coverage))
        // Auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/verify", get(auth::verify_token))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/public-key", get(auth::public_key))
        // Hubs
        .route("/hubs/register", post(hubs::register_hub))
        .route("/hubs", get(hubs::list_hubs))
        .route("/hubs/:id/status", get(hubs::hub_status))
        .route("/hubs/:id/heartbeat", put(hubs::heartbeat))
        .route("/hubs/:id/boundary", put(hubs::update_boundary))
        .route("/hubs/:id/contains", post(hubs::check_hub_contains_location))
        // Roaming
        .route("/roaming/validate", post(roaming::validate_roaming))
        // Admin - franchise payments
        .route(
            "/api/v1/admin/franchise-payments",
            post(franchise_payments::admin_create_payment)
                .get(franchise_payments::admin_list_payments),
        )
        .route(
            "/api/v1/admin/franchise-payments/:id/mark-paid",
            put(franchise_payments::admin_mark_paid),
        )
        .route(
            "/api/v1/admin/franchise-payments/:id/adjustments",
            post(franchise_payments::admin_create_adjustment)
                .get(franchise_payments::admin_list_adjustments),
        )
        .route(
            "/api/v1/admin/reports/payments",
            get(franchise_payments::admin_payment_report),
        )
        .route(
            "/api/v1/admin/hubs/:id/status-history",
            get(franchise_payments::admin_hub_status_history),
        )
        // Franchisee
        .route(
            "/api/v1/franchise/payments",
            get(franchisee::franchisee_list_payments),
        )
        .route(
            "/api/v1/franchise/payments/:id",
            get(franchisee::franchisee_get_payment),
        )
        .route(
            "/api/v1/franchise/payments/:id/pay",
            post(franchisee::franchisee_start_payment),
        )
        .route(
            "/api/v1/franchise/dashboard",
            get(franchisee::franchisee_dashboard),
        )
        .route(
            "/api/v1/franchise/notifications",
            get(franchisee::franchisee_list_notifications),
        )
        .route(
            "/api/v1/franchise/notifications/:id/read",
            put(franchisee::franchisee_mark_notification_read),
        )
        // Webhooks
        .route(
            "/api/v1/webhooks/payment-gateway",
            post(webhooks::payment_gateway_webhook),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok", "service": "axis-core"}))
}
