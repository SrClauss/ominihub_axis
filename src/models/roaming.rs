use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoamingValidation {
    pub id: Uuid,
    pub driver_id: Option<Uuid>,
    pub origin_hub_id: Option<Uuid>,
    pub target_hub_id: Option<Uuid>,
    pub allowed: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RoamingValidateRequest {
    pub driver_id: Uuid,
    pub driver_home_hub: String,
    pub target_hub: String,
}

#[derive(Debug, Serialize)]
pub struct RoamingValidateResponse {
    pub allowed: bool,
    pub origin_hub_id: Option<String>,
    pub reason: Option<String>,
}
