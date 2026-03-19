use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthClaims,
    models::hub::{Hub, HeartbeatRequest, RegisterHubRequest, UpdateBoundaryRequest},
    services::event_broadcaster::WsEvent,
    AppState,
};

pub async fn register_hub(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<RegisterHubRequest>,
) -> Response {
    if claims.0.role != "franchisee" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only franchisees can register hubs"})),
        )
            .into_response();
    }

    let boundary_json = req.boundary.to_string();
    let metadata = req.metadata.unwrap_or(json!({}));

    let hub = match sqlx::query_as::<_, Hub>(
        r#"
        INSERT INTO hubs (name, slug, boundary, api_url, admin_email, metadata)
        VALUES ($1, $2, ST_GeomFromGeoJSON($3)::geometry, $4, $5, $6)
        RETURNING id, name, slug, api_url, admin_email, status, last_heartbeat,
                  metadata, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&req.slug)
    .bind(&boundary_json)
    .bind(&req.api_url)
    .bind(&req.admin_email)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await
    {
        Ok(h) => h,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("hubs_slug_key") => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "Hub slug already exists"})),
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

    (StatusCode::CREATED, Json(hub)).into_response()
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(_req): Json<HeartbeatRequest>,
) -> Response {
    let current = match sqlx::query("SELECT status FROM hubs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Hub not found"})),
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

    let current_status: String = match current.try_get("status") {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Row decode error: {}", e)})),
            )
                .into_response();
        }
    };
    let was_offline = current_status != "online";

    match sqlx::query(
        r#"
        UPDATE hubs
        SET status = 'online', last_heartbeat = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    }

    if was_offline {
        if let Ok(Some(hub_row)) = sqlx::query("SELECT name, slug FROM hubs WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
        {
            let name: String = hub_row.try_get("name").unwrap_or_else(|_| String::new());
            let slug: String = hub_row.try_get("slug").unwrap_or_else(|_| String::new());
            state.broadcaster.broadcast(WsEvent {
                event: "hub.online".to_string(),
                timestamp: Utc::now(),
                data: json!({
                    "hub_id": id.to_string(),
                    "hub_name": name,
                    "hub_slug": slug,
                }),
            });
        }
    }

    Json(json!({"status": "ok", "timestamp": Utc::now().to_rfc3339()})).into_response()
}

pub async fn list_hubs(State(state): State<AppState>) -> Response {
    match sqlx::query_as::<_, Hub>(
        r#"
        SELECT id, name, slug, api_url, admin_email, status, last_heartbeat,
               metadata, created_at, updated_at
        FROM hubs
        ORDER BY name
        "#,
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(hubs) => Json(hubs).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn hub_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match sqlx::query_as::<_, Hub>(
        r#"
        SELECT id, name, slug, api_url, admin_email, status, last_heartbeat,
               metadata, created_at, updated_at
        FROM hubs WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(hub)) => Json(hub).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Hub not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn update_boundary(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    claims: AuthClaims,
    Json(req): Json<UpdateBoundaryRequest>,
) -> Response {
    if claims.0.role != "franchisee" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only franchisees can update hub boundaries"})),
        )
            .into_response();
    }

    let boundary_json = req.boundary.to_string();

    match sqlx::query(
        r#"
        UPDATE hubs
        SET boundary = ST_GeomFromGeoJSON($1)::geometry, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(&boundary_json)
    .bind(id)
    .execute(&state.db)
    .await
    {
        Ok(result) if result.rows_affected() == 0 => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Hub not found"})),
            )
                .into_response();
        }
        Ok(_) => {}
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response();
        }
    }

    state.broadcaster.broadcast(WsEvent {
        event: "coverage.updated".to_string(),
        timestamp: Utc::now(),
        data: json!({
            "hub_id": id.to_string(),
        }),
    });

    Json(json!({"status": "updated", "hub_id": id.to_string()})).into_response()
}
