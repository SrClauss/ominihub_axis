use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HubStatusHistory {
    pub id: Uuid,
    pub hub_id: Uuid,
    pub old_status: Option<String>,
    pub new_status: String,
    pub reason: Option<String>,
    pub changed_by: Option<Uuid>,
    pub changed_at: DateTime<Utc>,
}
