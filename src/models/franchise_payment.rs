use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "pending"),
            PaymentStatus::Paid => write!(f, "paid"),
            PaymentStatus::Overdue => write!(f, "overdue"),
            PaymentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FranchisePayment {
    pub id: Uuid,
    pub hub_id: Uuid,
    pub due_date: NaiveDate,
    pub amount: f64,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub transaction_id: Option<String>,
    pub gateway_payment_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub hub_id: Uuid,
    pub due_date: NaiveDate,
    pub amount: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarkPaidRequest {
    pub payment_method: Option<String>,
    pub transaction_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentReport {
    pub total_franchises: i64,
    pub active_franchises: i64,
    pub overdue_franchises: i64,
    pub total_revenue: f64,
    pub pending_revenue: f64,
    pub by_status: PaymentReportByStatus,
}

#[derive(Debug, Serialize)]
pub struct PaymentReportByStatus {
    pub active: i64,
    pub grace: i64,
    pub restricted: i64,
    pub suspended: i64,
}
