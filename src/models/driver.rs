use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Driver {
    pub id: Uuid,
    pub email: String,
    pub phone: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub license_plate: Option<String>,
    pub vehicle_model: Option<String>,
    pub vehicle_year: Option<i32>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDriverRequest {
    pub email: String,
    pub phone: String,
    pub password: String,
    pub name: String,
    pub license_plate: Option<String>,
    pub vehicle_model: Option<String>,
    pub vehicle_year: Option<i32>,
}
