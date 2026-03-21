use anyhow::Result;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::franchise_payment::FranchisePayment;

/// Represents a payment event received from a gateway webhook.
#[derive(Debug)]
pub struct PaymentEvent {
    pub transaction_id: String,
    pub status: GatewayPaymentStatus,
    pub metadata: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GatewayPaymentStatus {
    Approved,
    Pending,
    Rejected,
    Cancelled,
}

/// Trait for payment gateway integrations.
#[async_trait::async_trait]
pub trait PaymentGateway: Send + Sync {
    /// Create a checkout link for the given payment.
    async fn create_payment_link(&self, payment: &FranchisePayment) -> Result<String>;

    /// Verify the current status of a transaction by its gateway ID.
    async fn verify_payment(&self, transaction_id: &str) -> Result<GatewayPaymentStatus>;

    /// Parse a raw webhook payload into a PaymentEvent.
    fn parse_webhook(&self, payload: &Value) -> Result<PaymentEvent>;
}

/// Mercado Pago implementation of the PaymentGateway trait.
pub struct MercadoPagoGateway {
    access_token: String,
    base_url: String,
    app_base_url: String,
}

impl MercadoPagoGateway {
    pub fn new(access_token: String, app_base_url: String) -> Self {
        Self {
            access_token,
            base_url: "https://api.mercadopago.com".to_string(),
            app_base_url,
        }
    }

    /// Build the Mercado Pago preference payload for a payment.
    fn build_preference_body(&self, payment: &FranchisePayment) -> Value {
        serde_json::json!({
            "items": [{
                "title": format!("Mensalidade AXIS Hub - {}", payment.hub_id),
                "quantity": 1,
                "unit_price": payment.amount,
                "currency_id": "BRL"
            }],
            "external_reference": payment.id.to_string(),
            "back_urls": {
                "success": format!("{}/payment/success", self.app_base_url),
                "failure": format!("{}/payment/failure", self.app_base_url),
                "pending": format!("{}/payment/pending", self.app_base_url)
            },
            "auto_return": "approved",
            "notification_url": format!("{}/api/v1/webhooks/payment-gateway", self.app_base_url)
        })
    }
}

#[async_trait::async_trait]
impl PaymentGateway for MercadoPagoGateway {
    async fn create_payment_link(&self, payment: &FranchisePayment) -> Result<String> {
        let client = reqwest::Client::new();
        let body = self.build_preference_body(payment);

        let response = client
            .post(format!("{}/checkout/preferences", self.base_url))
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("MercadoPago API error {}: {}", status, text);
        }

        let json: Value = response.json().await?;
        let url = json["init_point"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("MercadoPago response missing init_point"))?
            .to_string();

        Ok(url)
    }

    async fn verify_payment(&self, transaction_id: &str) -> Result<GatewayPaymentStatus> {
        let client = reqwest::Client::new();

        let response = client
            .get(format!("{}/v1/payments/{}", self.base_url, transaction_id))
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("MercadoPago verify error: {}", response.status());
        }

        let json: Value = response.json().await?;
        let status = json["status"].as_str().unwrap_or("unknown");

        Ok(match status {
            "approved" => GatewayPaymentStatus::Approved,
            "pending" | "in_process" => GatewayPaymentStatus::Pending,
            "rejected" => GatewayPaymentStatus::Rejected,
            "cancelled" => GatewayPaymentStatus::Cancelled,
            _ => GatewayPaymentStatus::Pending,
        })
    }

    fn parse_webhook(&self, payload: &Value) -> Result<PaymentEvent> {
        let action = payload["action"].as_str().unwrap_or("");
        let transaction_id = if let Some(s) = payload["data"]["id"].as_str() {
            s.to_string()
        } else if let Some(n) = payload["data"]["id"].as_u64() {
            n.to_string()
        } else {
            String::new()
        };

        let status = match action {
            "payment.created" | "payment.updated" => {
                let s = payload["data"]["status"].as_str().unwrap_or("pending");
                match s {
                    "approved" => GatewayPaymentStatus::Approved,
                    "rejected" => GatewayPaymentStatus::Rejected,
                    "cancelled" => GatewayPaymentStatus::Cancelled,
                    _ => GatewayPaymentStatus::Pending,
                }
            }
            _ => GatewayPaymentStatus::Pending,
        };

        Ok(PaymentEvent {
            transaction_id,
            status,
            metadata: payload.clone(),
        })
    }
}

/// Log a webhook payload to the database.
pub async fn log_webhook(
    pool: &PgPool,
    gateway: &str,
    event_type: Option<&str>,
    payload: &Value,
    payment_id: Option<Uuid>,
) -> Result<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO payment_webhook_logs (gateway, event_type, payload, payment_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(gateway)
    .bind(event_type)
    .bind(payload)
    .bind(payment_id)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// Mark a webhook log as processed.
pub async fn mark_webhook_processed(
    pool: &PgPool,
    log_id: Uuid,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE payment_webhook_logs
        SET processed = true, error_message = $2, processed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(log_id)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}
