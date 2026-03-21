use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum EntityType {
    #[sqlx(rename = "user")]
    User,
    #[sqlx(rename = "driver")]
    Driver,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BlockedEntity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub blocked_by: Option<Uuid>,
    pub reason: String,
    pub blocked_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub hub_scope: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct BlockEntityRequest {
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub reason: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub hub_scope: Option<Vec<Uuid>>,
}
