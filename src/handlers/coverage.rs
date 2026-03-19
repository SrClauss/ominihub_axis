use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::AppState;

pub async fn get_coverage_map(State(state): State<AppState>) -> Response {
    let rows = match sqlx::query(
        r#"
        SELECT
            id,
            name,
            slug,
            api_url,
            status,
            updated_at,
            metadata,
            ST_AsGeoJSON(boundary)::text as geometry
        FROM hubs
        WHERE status != 'offline'
        "#
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    };

    let mut hasher = Sha256::new();
    for row in &rows {
        let id: Uuid = match row.try_get("id") {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
        };
        let updated_at: chrono::DateTime<chrono::Utc> = match row.try_get("updated_at") {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
        };
        hasher.update(id.to_string().as_bytes());
        hasher.update(updated_at.to_string().as_bytes());
    }
    let checksum = hex::encode(hasher.finalize());

    let mut features: Vec<Value> = Vec::with_capacity(rows.len());
    for row in &rows {
            let id: Uuid = match row.try_get("id") {
                Ok(v) => v,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
            };
            let name: String = match row.try_get("name") {
                Ok(v) => v,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
            };
            let slug: String = match row.try_get("slug") {
                Ok(v) => v,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
            };
            let api_url: String = match row.try_get("api_url") {
                Ok(v) => v,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
            };
            let status: String = match row.try_get("status") {
                Ok(v) => v,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
            };
            let metadata: Value = row.try_get("metadata").unwrap_or(Value::Null);
            let geometry_str: Option<String> = row.try_get("geometry").ok().flatten();
            let geometry: Value = geometry_str
                .and_then(|g| serde_json::from_str(&g).ok())
                .unwrap_or(Value::Null);

            features.push(json!({
                "type": "Feature",
                "geometry": geometry,
                "properties": {
                    "id": id.to_string(),
                    "name": name,
                    "slug": slug,
                    "api_url": api_url,
                    "status": status,
                    "metadata": metadata,
                }
            }));
    }

    Json(json!({
        "version": chrono::Utc::now().to_rfc3339(),
        "checksum": checksum,
        "type": "FeatureCollection",
        "features": features,
    }))
    .into_response()
}

pub async fn get_coverage_version(State(state): State<AppState>) -> Response {
    let rows = match sqlx::query(
        "SELECT id, updated_at FROM hubs WHERE status != 'offline'"
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    };

    let mut hasher = Sha256::new();
    for row in &rows {
        let id: Uuid = match row.try_get("id") {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
        };
        let updated_at: chrono::DateTime<chrono::Utc> = match row.try_get("updated_at") {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
        };
        hasher.update(id.to_string().as_bytes());
        hasher.update(updated_at.to_string().as_bytes());
    }
    let checksum = hex::encode(hasher.finalize());

    Json(json!({
        "version": chrono::Utc::now().to_rfc3339(),
        "checksum": checksum,
        "hub_count": rows.len(),
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
pub struct ValidateCoverageRequest {
    pub lat: f64,
    pub lng: f64,
    pub detected_hub_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}

pub async fn validate_coverage(
    State(state): State<AppState>,
    Json(req): Json<ValidateCoverageRequest>,
) -> Response {
    let hub_row = match sqlx::query(
        r#"
        SELECT id, name, slug, api_url
        FROM hubs
        WHERE ST_Contains(boundary, ST_SetSRID(ST_MakePoint($1, $2), 4326))
        LIMIT 1
        "#,
    )
    .bind(req.lng)
    .bind(req.lat)
    .fetch_optional(&state.db)
    .await
    {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    };

    let (confirmed, hub_id, hub_name, api_url) = match &hub_row {
        Some(row) => {
            let id: Uuid = row.try_get("id").unwrap_or_default();
            let name: String = row.try_get("name").unwrap_or_default();
            let url: String = row.try_get("api_url").unwrap_or_default();
            let confirmed = req.detected_hub_id.map(|did| did == id).unwrap_or(false);
            (confirmed, Some(id.to_string()), Some(name), Some(url))
        }
        None => (false, None, None, None),
    };

    if let Some(user_id) = req.user_id {
        let insert_hub_id: Option<Uuid> = hub_row.as_ref().and_then(|row| row.try_get("id").ok());
        let _ = sqlx::query(
            r#"
            INSERT INTO hub_validations (user_id, location, detected_hub_id, confirmed)
            VALUES ($1, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(req.lng)
        .bind(req.lat)
        .bind(insert_hub_id)
        .bind(confirmed)
        .execute(&state.db)
        .await;
    }

    Json(json!({
        "confirmed": confirmed,
        "hub_id": hub_id,
        "hub_name": hub_name,
        "api_url": api_url,
    }))
    .into_response()
}
