use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthClaims,
    models::{
        franchise_payment::{CreatePaymentRequest, MarkPaidRequest},
        payment_adjustment::CreateAdjustmentRequest,
    },
    services::{
        hub_status_service::HubStatusService,
        payment_service::PaymentService,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct ListPaymentsQuery {
    pub hub_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// POST /api/v1/admin/franchise-payments
pub async fn admin_create_payment(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<CreatePaymentRequest>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = PaymentService::new(state.db.clone());
    match svc.create_payment(req).await {
        Ok(payment) => (StatusCode::CREATED, Json(payment)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/admin/franchise-payments
pub async fn admin_list_payments(
    State(state): State<AppState>,
    claims: AuthClaims,
    Query(q): Query<ListPaymentsQuery>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = PaymentService::new(state.db.clone());
    match svc.list_payments(q.hub_id).await {
        Ok(payments) => Json(payments).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// PUT /api/v1/admin/franchise-payments/:id/mark-paid
pub async fn admin_mark_paid(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
    Json(req): Json<MarkPaidRequest>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = PaymentService::new(state.db.clone());
    match svc.mark_paid(id, req).await {
        Ok(Some(payment)) => Json(payment).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Payment not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// POST /api/v1/admin/franchise-payments/:id/adjustments
pub async fn admin_create_adjustment(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAdjustmentRequest>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let admin_id = match Uuid::parse_str(&claims.0.sub) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token"}))).into_response(),
    };

    let svc = PaymentService::new(state.db.clone());
    match svc.create_adjustment(id, req, admin_id).await {
        Ok(Some(adj)) => (StatusCode::CREATED, Json(adj)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Payment not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/admin/franchise-payments/:id/adjustments
pub async fn admin_list_adjustments(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = PaymentService::new(state.db.clone());
    match svc.get_adjustments_for_payment(id).await {
        Ok(adjs) => Json(adjs).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/admin/reports/payments
pub async fn admin_payment_report(
    State(state): State<AppState>,
    claims: AuthClaims,
    Query(q): Query<ReportQuery>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = PaymentService::new(state.db.clone());
    match svc.get_report(q.start_date, q.end_date).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/admin/hubs/:id/status-history
pub async fn admin_hub_status_history(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(hub_id): Path<Uuid>,
) -> Response {
    if !is_admin(&claims) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))).into_response();
    }

    let svc = HubStatusService::new(state.db.clone());
    match svc.get_history(hub_id).await {
        Ok(history) => Json(history).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

fn is_admin(claims: &AuthClaims) -> bool {
    matches!(claims.0.role.as_str(), "super_admin" | "hub_admin" | "admin" | "finance" | "support")
}
