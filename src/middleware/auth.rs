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
