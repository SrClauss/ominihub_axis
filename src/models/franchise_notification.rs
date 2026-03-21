use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    PaymentDue,
    PaymentOverdue,
    GracePeriodStarted,
    RestrictedMode,
    Suspended,
    PaymentReceived,
    PaymentFailed,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::PaymentDue => write!(f, "payment_due"),
            NotificationType::PaymentOverdue => write!(f, "payment_overdue"),
            NotificationType::GracePeriodStarted => write!(f, "grace_period_started"),
            NotificationType::RestrictedMode => write!(f, "restricted_mode"),
            NotificationType::Suspended => write!(f, "suspended"),
            NotificationType::PaymentReceived => write!(f, "payment_received"),
            NotificationType::PaymentFailed => write!(f, "payment_failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FranchiseNotification {
    pub id: Uuid,
    pub hub_id: Uuid,
    pub notification_type: String,
    pub message: String,
    pub metadata: Option<Value>,
    pub sent_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}
