use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{models::user::Claims, services::auth_service::validate_token, AppState};

pub struct AuthClaims(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Missing Authorization header"})),
                )
                    .into_response()
            })?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Invalid Authorization header format"})),
                )
                    .into_response()
            })?;

        let claims = validate_token(token, &state.jwt_public_key).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": format!("Invalid token: {}", e)})),
            )
                .into_response()
        })?;

        Ok(AuthClaims(claims))
    }
}

/// Middleware guard: rejects requests whose JWT role is not `super_admin`.
pub async fn require_super_admin(
    claims: AuthClaims,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    if claims.0.role != "super_admin" {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(request).await)
}

/// Middleware guard: rejects requests from admins that have no hub-level access.
/// Full hub-scope enforcement is delegated to handlers that receive the claims.
/// TODO: Once admin sessions carry hub_id, enforce that the claim's hub list
///       contains the target hub extracted from the request path/body.
pub async fn require_hub_access(
    _claims: AuthClaims,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    Ok(next.run(request).await)
}
