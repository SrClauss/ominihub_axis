use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde_json::json;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthClaims,
    models::user::{AuthResponse, LoginRequest, RegisterRequest, User, UserPublic},
    services::auth_service::{generate_tokens, validate_token},
    AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    if !["driver", "passenger", "franchisee"].contains(&req.role.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid role. Must be driver, passenger, or franchisee"})),
        )
            .into_response();
    }

    let password_hash = match hash(&req.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to hash password: {}", e)})),
            )
                .into_response();
        }
    };

    let user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, role, home_hub_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, email, password_hash, role, home_hub_id, active, created_at, updated_at
        "#,
    )
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.role)
    .bind(req.home_hub_id)
    .fetch_one(&state.db)
    .await
    {
        Ok(u) => u,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("users_email_key") => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "Email already registered"})),
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

    let (token, refresh_token) = match generate_tokens(&user, &state.jwt_private_key) {
        Ok(tokens) => tokens,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Token generation failed: {}", e)})),
            )
                .into_response();
        }
    };

    (
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            refresh_token,
            user: UserPublic {
                id: user.id,
                email: user.email,
                role: user.role,
                home_hub_id: user.home_hub_id,
            },
        }),
    )
        .into_response()
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let user = match sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, role, home_hub_id, active, created_at, updated_at FROM users WHERE email = $1",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid credentials"})),
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

    if !user.active {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Account is inactive"})),
        )
            .into_response();
    }

    match verify(&req.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid credentials"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Password verification error: {}", e)})),
            )
                .into_response();
        }
    }

    let (token, refresh_token) = match generate_tokens(&user, &state.jwt_private_key) {
        Ok(tokens) => tokens,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Token generation failed: {}", e)})),
            )
                .into_response();
        }
    };

    Json(AuthResponse {
        token,
        refresh_token,
        user: UserPublic {
            id: user.id,
            email: user.email,
            role: user.role,
            home_hub_id: user.home_hub_id,
        },
    })
    .into_response()
}

pub async fn verify_token(
    State(state): State<AppState>,
    claims: AuthClaims,
) -> Response {
    let user_id = match claims.0.sub.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token subject"})),
            )
                .into_response();
        }
    };

    match sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, role, home_hub_id, active, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(user)) => Json(json!({
            "valid": true,
            "user": UserPublic {
                id: user.id,
                email: user.email,
                role: user.role,
                home_hub_id: user.home_hub_id,
            }
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let refresh_token = match body.get("refresh_token").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "refresh_token required"})),
            )
                .into_response();
        }
    };

    let claims = match validate_token(&refresh_token, &state.jwt_public_key) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": format!("Invalid refresh token: {}", e)})),
            )
                .into_response();
        }
    };

    if claims.token_type != "refresh" {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Not a refresh token"})),
        )
            .into_response();
    }

    let user_id = match claims.sub.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token subject"})),
            )
                .into_response();
        }
    };

    match sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, role, home_hub_id, active, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(user)) => {
            let (token, new_refresh_token) =
                match generate_tokens(&user, &state.jwt_private_key) {
                    Ok(tokens) => tokens,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": format!("Token generation failed: {}", e)})),
                        )
                            .into_response();
                    }
                };
            Json(AuthResponse {
                token,
                refresh_token: new_refresh_token,
                user: UserPublic {
                    id: user.id,
                    email: user.email,
                    role: user.role,
                    home_hub_id: user.home_hub_id,
                },
            })
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn public_key(State(state): State<AppState>) -> Response {
    Json(json!({
        "public_key": state.jwt_public_key,
        "algorithm": "RS256"
    }))
    .into_response()
}
