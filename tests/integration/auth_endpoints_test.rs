use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{
    cleanup_test_db, create_test_user, setup_test_db, test_rsa_keys,
};

/// Registrar um novo usuário deve retornar 201 com token e dados do usuário.
#[tokio::test]
async fn test_register_endpoint() {
    let pool = setup_test_db().await;
    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "newuser@test.com",
                        "password": "securepass123",
                        "role": "driver"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["token"].is_string());
    assert!(json["refresh_token"].is_string());
    assert_eq!(json["user"]["email"], "newuser@test.com");
    assert_eq!(json["user"]["role"], "driver");

    cleanup_test_db(&pool).await;
}

/// Login com credenciais válidas deve retornar 200 com tokens.
#[tokio::test]
async fn test_login_endpoint() {
    let pool = setup_test_db().await;
    create_test_user(&pool, "login@test.com", "driver").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "login@test.com",
                        "password": "test123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].is_string());

    cleanup_test_db(&pool).await;
}

/// Tentar registrar com email já existente deve retornar 409 Conflict.
#[tokio::test]
async fn test_register_duplicate_email() {
    let pool = setup_test_db().await;
    create_test_user(&pool, "existing@test.com", "driver").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "existing@test.com",
                        "password": "pass123",
                        "role": "driver"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    cleanup_test_db(&pool).await;
}

/// Login com credenciais inválidas deve retornar 401 Unauthorized.
#[tokio::test]
async fn test_login_wrong_password() {
    let pool = setup_test_db().await;
    create_test_user(&pool, "wrongpass@test.com", "driver").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "wrongpass@test.com",
                        "password": "senhaerrada"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    cleanup_test_db(&pool).await;
}

/// Registrar com role inválida deve retornar 400 Bad Request.
#[tokio::test]
async fn test_register_invalid_role() {
    let pool = setup_test_db().await;
    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "newuser2@test.com",
                        "password": "pass123",
                        "role": "superuser"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_test_db(&pool).await;
}
