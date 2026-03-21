use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthClaims,
    services::{
        hub_status_service::HubStatusService,
        notification_service::NotificationService,
        payment_gateway::{MercadoPagoGateway, PaymentGateway},
        payment_service::PaymentService,
    },
    AppState,
};

/// GET /api/v1/franchise/payments
/// Lists all payments for the authenticated franchisee's hub.
pub async fn franchisee_list_payments(
    State(state): State<AppState>,
    claims: AuthClaims,
) -> Response {
    let hub_id = match get_hub_id_for_franchisee(&state, &claims).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let svc = PaymentService::new(state.db.clone());
    match svc.list_payments(Some(hub_id)).await {
        Ok(payments) => Json(payments).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/franchise/payments/:id
pub async fn franchisee_get_payment(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(payment_id): Path<Uuid>,
) -> Response {
    let hub_id = match get_hub_id_for_franchisee(&state, &claims).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let svc = PaymentService::new(state.db.clone());
    match svc.get_payment(payment_id).await {
        Ok(Some(payment)) if payment.hub_id == hub_id => Json(payment).into_response(),
        Ok(Some(_)) => (StatusCode::FORBIDDEN, Json(json!({"error": "Not your payment"}))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Payment not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// POST /api/v1/franchise/payments/:id/pay
/// Generates a Mercado Pago payment link for the given payment.
pub async fn franchisee_start_payment(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(payment_id): Path<Uuid>,
) -> Response {
    let hub_id = match get_hub_id_for_franchisee(&state, &claims).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let svc = PaymentService::new(state.db.clone());

    let payment = match svc.get_payment(payment_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Payment not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))).into_response(),
    };

    if payment.hub_id != hub_id {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Not your payment"}))).into_response();
    }

    if payment.status == "paid" {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "Payment already paid"}))).into_response();
    }

    // If we already have a gateway URL, return it
    if let Some(ref url) = payment.gateway_payment_url {
        return Json(json!({"payment_url": url})).into_response();
    }

    // Generate a new payment link via Mercado Pago
    let token = match &state.mercadopago_token {
        Some(t) => t.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Payment gateway not configured"}))).into_response(),
    };

    let gateway = MercadoPagoGateway::new(token, state.app_base_url.clone());
    match gateway.create_payment_link(&payment).await {
        Ok(url) => {
            let _ = svc.set_gateway_url(payment_id, &url).await;
            Json(json!({"payment_url": url})).into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("Gateway error: {}", e)})),
        ).into_response(),
    }
}

/// GET /api/v1/franchise/dashboard
pub async fn franchisee_dashboard(
    State(state): State<AppState>,
    claims: AuthClaims,
) -> Response {
    let hub_id = match get_hub_id_for_franchisee(&state, &claims).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let payment_svc = PaymentService::new(state.db.clone());
    let status_svc = HubStatusService::new(state.db.clone());

    let payments = match payment_svc.list_payments(Some(hub_id)).await {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))).into_response(),
    };

    let operational_status = match status_svc.get_status(hub_id).await {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))).into_response(),
    };

    let pending = payments.iter().filter(|p| p.status == "pending" || p.status == "overdue").collect::<Vec<_>>();
    let overdue_days: Option<i64> = pending.iter().map(|p| {
        let now = chrono::Utc::now().date_naive();
        (now - p.due_date).num_days()
    }).filter(|&d| d > 0).max();

    let mut warnings = Vec::<String>::new();
    if let Some(days) = overdue_days {
        if days > 0 {
            warnings.push(format!("Pagamento pendente há {} dias.", days));
        }
        match operational_status.as_str() {
            "grace" => warnings.push("Seu hub está no período de carência. Regularize o pagamento.".into()),
            "restricted" => warnings.push("Seu hub está em modo RESTRITO. Regularize imediatamente.".into()),
            "suspended" => warnings.push("Seu hub está SUSPENSO. Entre em contato.".into()),
            _ => {}
        }
    }

    let history = payments.iter().take(5).collect::<Vec<_>>();

    Json(json!({
        "hub_id": hub_id,
        "operational_status": operational_status,
        "pending_payments": pending,
        "warnings": warnings,
        "payment_history": history
    })).into_response()
}

/// GET /api/v1/franchise/notifications
pub async fn franchisee_list_notifications(
    State(state): State<AppState>,
    claims: AuthClaims,
) -> Response {
    let hub_id = match get_hub_id_for_franchisee(&state, &claims).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let svc = NotificationService::new(state.db.clone());
    match svc.list_for_hub(hub_id).await {
        Ok(notifs) => Json(notifs).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// PUT /api/v1/franchise/notifications/:id/read
pub async fn franchisee_mark_notification_read(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(notif_id): Path<Uuid>,
) -> Response {
    // Verify it's a franchisee
    if claims.0.role != "franchisee" {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Franchisee access required"}))).into_response();
    }

    let svc = NotificationService::new(state.db.clone());
    match svc.mark_read(notif_id).await {
        Ok(Some(n)) => Json(n).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Notification not found or already read"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        ).into_response(),
    }
}

/// Helper: look up the hub_id for the currently authenticated franchisee.
async fn get_hub_id_for_franchisee(state: &AppState, claims: &AuthClaims) -> Result<Uuid, Response> {
    if claims.0.role != "franchisee" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Franchisee access required"}))).into_response());
    }

    let user_id = Uuid::parse_str(&claims.0.sub).map_err(|_| {
        (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token subject"}))).into_response()
    })?;

    // Look up the hub where the admin_email matches the user's email
    let hub_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT h.id FROM hubs h JOIN users u ON h.admin_email = u.email WHERE u.id = $1 LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))).into_response()
    })?;

    hub_id.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(json!({"error": "No hub associated with your account"}))).into_response()
    })
}
