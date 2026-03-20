use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Hub {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub api_url: String,
    pub admin_email: Option<String>,
    pub status: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub boundary: Value,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterHubRequest {
    pub name: String,
    pub slug: String,
    pub api_url: String,
    pub boundary: Value,
    pub admin_email: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub timestamp: DateTime<Utc>,
    pub active_drivers: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBoundaryRequest {
    pub boundary: Value,
}

#[derive(Debug, Deserialize)]
pub struct LocationCheckRequest {
    pub lat: f64,
    pub lng: f64,
}
