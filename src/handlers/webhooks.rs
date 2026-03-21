use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    models::franchise_payment::MarkPaidRequest,
    services::{
        notification_service::NotificationService,
        payment_gateway::{log_webhook, mark_webhook_processed, GatewayPaymentStatus, MercadoPagoGateway, PaymentGateway},
        payment_service::PaymentService,
    },
    AppState,
};

/// POST /api/v1/webhooks/payment-gateway
/// Processes payment gateway callbacks (Mercado Pago).
pub async fn payment_gateway_webhook(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    // Log the webhook immediately
    let log_id = match log_webhook(&state.db, "mercadopago", payload["action"].as_str(), &payload, None).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to log webhook: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to log webhook"}))).into_response();
        }
    };

    let token = match &state.mercadopago_token {
        Some(t) => t.clone(),
        None => {
            let _ = mark_webhook_processed(&state.db, log_id, Some("Payment gateway not configured")).await;
            return (StatusCode::OK, Json(json!({"status": "ignored"}))).into_response();
        }
    };

    let gateway = MercadoPagoGateway::new(token, state.app_base_url.clone());

    // Parse the webhook event
    let event = match gateway.parse_webhook(&payload) {
        Ok(e) => e,
        Err(e) => {
            let msg = format!("Failed to parse webhook: {}", e);
            tracing::warn!("{}", msg);
            let _ = mark_webhook_processed(&state.db, log_id, Some(&msg)).await;
            return (StatusCode::OK, Json(json!({"status": "parse_error"}))).into_response();
        }
    };

    if event.transaction_id.is_empty() {
        let _ = mark_webhook_processed(&state.db, log_id, Some("No transaction_id")).await;
        return (StatusCode::OK, Json(json!({"status": "no_transaction_id"}))).into_response();
    }

    // Find the payment by transaction_id
    let payment_svc = PaymentService::new(state.db.clone());
    let notif_svc = NotificationService::new(state.db.clone());

    // Try to find payment by transaction_id or by external_reference in the payload
    let ext_ref = payload["data"]["external_reference"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let payment_opt = if !ext_ref.is_empty() {
        if let Ok(pid) = uuid::Uuid::parse_str(&ext_ref) {
            payment_svc.get_payment(pid).await.unwrap_or(None)
        } else {
            None
        }
    } else {
        None
    };

    match payment_opt {
        Some(payment) => {
            match event.status {
                GatewayPaymentStatus::Approved => {
                    let req = MarkPaidRequest {
                        payment_method: Some("mercadopago".to_string()),
                        transaction_id: Some(event.transaction_id.clone()),
                        notes: Some("Paid via Mercado Pago webhook".to_string()),
                    };
                    match payment_svc.mark_paid(payment.id, req).await {
                        Ok(Some(_)) => {
                            let _ = notif_svc.notify_payment_received(payment.hub_id, payment.amount).await;
                            let _ = mark_webhook_processed(&state.db, log_id, None).await;
                        }
                        Ok(None) => {
                            let _ = mark_webhook_processed(&state.db, log_id, Some("Payment not found")).await;
                        }
                        Err(e) => {
                            let msg = format!("DB error marking paid: {}", e);
                            let _ = mark_webhook_processed(&state.db, log_id, Some(&msg)).await;
                        }
                    }
                }
                GatewayPaymentStatus::Rejected | GatewayPaymentStatus::Cancelled => {
                    let _ = notif_svc
                        .notify_payment_failed(payment.hub_id, "Payment rejected or cancelled by gateway")
                        .await;
                    let _ = mark_webhook_processed(&state.db, log_id, None).await;
                }
                GatewayPaymentStatus::Pending => {
                    let _ = mark_webhook_processed(&state.db, log_id, None).await;
                }
            }
        }
        None => {
            tracing::warn!(
                transaction_id = %event.transaction_id,
                "payment_gateway_webhook: no matching payment found for external_reference"
            );
            let _ = mark_webhook_processed(&state.db, log_id, Some("Payment not found for external_reference")).await;
        }
    }

    (StatusCode::OK, Json(json!({"status": "ok"}))).into_response()
}
