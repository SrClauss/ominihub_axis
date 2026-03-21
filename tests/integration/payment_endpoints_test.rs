use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{
    cleanup_test_db, create_test_hub, create_test_hub_for_franchisee, create_test_user,
    generate_test_jwt, setup_test_db, test_rsa_keys,
};
use axis_core::models::franchise_payment::CreatePaymentRequest;
use axis_core::services::payment_service::PaymentService;
use chrono::NaiveDate;

/// Admin pode criar um pagamento de franquia via POST /api/v1/admin/franchise-payments.
#[tokio::test]
async fn test_admin_create_payment() {
    let pool = setup_test_db().await;
    let admin = create_test_user(&pool, "admin-cp@test.com", "admin").await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-admin-cp").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let token = generate_test_jwt(admin.id, "admin", &priv_key);
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/franchise-payments")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "hub_id": hub_id,
                        "due_date": "2026-04-01",
                        "amount": 299.99,
                        "notes": "Pagamento de teste"
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
    assert_eq!(json["status"], "pending");
    assert_eq!(json["amount"], 299.99);

    cleanup_test_db(&pool).await;
}

/// Sem token de autorização, a criação deve retornar 401 Unauthorized.
#[tokio::test]
async fn test_admin_create_payment_unauthorized() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-unauth").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/franchise-payments")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "hub_id": hub_id,
                        "due_date": "2026-04-01",
                        "amount": 299.99
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

/// Franqueado pode listar seus pagamentos via GET /api/v1/franchise/payments.
#[tokio::test]
async fn test_franchisee_list_payments() {
    let pool = setup_test_db().await;
    let franchisee = create_test_user(&pool, "franchisee-lp@test.com", "franchisee").await;
    let hub_id =
        create_test_hub_for_franchisee(&pool, "My Hub", "my-hub-lp", &franchisee.email).await;

    // Criar um pagamento para o hub
    let payment_svc = PaymentService::new(pool.clone());
    payment_svc
        .create_payment(CreatePaymentRequest {
            hub_id,
            due_date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            amount: 299.99,
            notes: None,
        })
        .await
        .unwrap();

    let (priv_key, pub_key) = test_rsa_keys();
    let token = generate_test_jwt(franchisee.id, "franchisee", &priv_key);
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/franchise/payments")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payments: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!payments.is_empty());

    cleanup_test_db(&pool).await;
}

/// Driver (não-admin) tentando criar pagamento deve receber 403 Forbidden.
#[tokio::test]
async fn test_non_admin_create_payment_forbidden() {
    let pool = setup_test_db().await;
    let driver = create_test_user(&pool, "driver-nb@test.com", "driver").await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-driver").await;

    let (priv_key, pub_key) = test_rsa_keys();
    let token = generate_test_jwt(driver.id, "driver", &priv_key);
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/franchise-payments")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "hub_id": hub_id,
                        "due_date": "2026-04-01",
                        "amount": 299.99
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    cleanup_test_db(&pool).await;
}

/// O endpoint de health check deve retornar 200.
#[tokio::test]
async fn test_health_endpoint() {
    let pool = setup_test_db().await;
    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    cleanup_test_db(&pool).await;
}
