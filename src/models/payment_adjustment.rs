use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentType {
    Discount,
    Penalty,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentAdjustment {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub adjustment_type: String,
    pub amount: f64,
    pub reason: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdjustmentRequest {
    pub adjustment_type: AdjustmentType,
    pub amount: f64,
    pub reason: String,
}
