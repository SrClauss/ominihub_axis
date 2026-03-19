use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthClaims,
    models::roaming::{RoamingValidateRequest, RoamingValidateResponse},
    AppState,
};

pub async fn validate_roaming(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<RoamingValidateRequest>,
) -> Response {
    if claims.0.sub != req.driver_id.to_string() && claims.0.role != "franchisee" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "You can only validate your own roaming"})),
        )
            .into_response();
    }

    if claims.0.role != "driver" && claims.0.role != "franchisee" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only drivers and franchisees can validate roaming"})),
        )
            .into_response();
    }

    let origin_hub = match sqlx::query(
        "SELECT id, name, slug, status FROM hubs WHERE slug = $1",
    )
    .bind(&req.driver_home_hub)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Origin hub not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    };

    let target_hub = match sqlx::query(
        "SELECT id, name, slug, status FROM hubs WHERE slug = $1",
    )
    .bind(&req.target_hub)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Target hub not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    };

    let target_status: String = match target_hub.try_get("status") {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
    };
    let allowed = target_status == "online";
    let target_slug: String = match target_hub.try_get("slug") {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
    };
    let reason = if !allowed {
        Some(format!("Target hub '{}' is not online", target_slug))
    } else {
        None
    };

    let origin_id: Uuid = match origin_hub.try_get("id") {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
    };
    let target_id: Uuid = match target_hub.try_get("id") {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Row decode error: {}", e)}))).into_response(),
    };

    let _ = sqlx::query(
        r#"
        INSERT INTO roaming_validations (driver_id, origin_hub_id, target_hub_id, allowed)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(req.driver_id)
    .bind(origin_id)
    .bind(target_id)
    .bind(allowed)
    .execute(&state.db)
    .await;

    Json(RoamingValidateResponse {
        allowed,
        origin_hub_id: Some(origin_id.to_string()),
        reason,
    })
    .into_response()
}
