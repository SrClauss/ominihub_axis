use axis_core::models::franchise_payment::CreatePaymentRequest;
use axis_core::services::hub_status_service::HubStatusService;
use axis_core::services::payment_service::PaymentService;
use chrono::Utc;

use crate::common::{cleanup_test_db, create_test_hub, setup_test_db};

/// Testa a função pura `compute_status` com diferentes valores de dias de atraso.
#[test]
fn test_compute_status() {
    assert_eq!(HubStatusService::compute_status(0), "active");
    assert_eq!(HubStatusService::compute_status(7), "active");
    assert_eq!(HubStatusService::compute_status(8), "grace");
    assert_eq!(HubStatusService::compute_status(10), "grace");
    assert_eq!(HubStatusService::compute_status(11), "restricted");
    assert_eq!(HubStatusService::compute_status(14), "restricted");
    assert_eq!(HubStatusService::compute_status(15), "suspended");
    assert_eq!(HubStatusService::compute_status(30), "suspended");
}

/// Sem nenhum registro de status, o hub deve retornar "active" por padrão.
#[tokio::test]
async fn test_get_status_default() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-status-default").await;
    let service = HubStatusService::new(pool.clone());

    let status = service.get_status(hub_id).await.unwrap();
    assert_eq!(status, "active");

    cleanup_test_db(&pool).await;
}

/// Após criar um pagamento com 8 dias de atraso, o status do hub deve ser "grace".
#[tokio::test]
async fn test_update_hub_status_to_grace() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-grace").await;
    let service = HubStatusService::new(pool.clone());
    let payment_svc = PaymentService::new(pool.clone());

    // Pagamento vencido há 8 dias
    let due_date = Utc::now().date_naive() - chrono::Duration::days(8);
    payment_svc
        .create_payment(CreatePaymentRequest {
            hub_id,
            due_date,
            amount: 299.99,
            notes: None,
        })
        .await
        .unwrap();

    let changed = service.update_hub_status(hub_id, None).await.unwrap();
    assert_eq!(changed, Some("grace".to_string()));

    let status = service.get_status(hub_id).await.unwrap();
    assert_eq!(status, "grace");

    cleanup_test_db(&pool).await;
}

/// Após criar um pagamento com 15 dias de atraso, o status do hub deve ser "suspended".
#[tokio::test]
async fn test_update_hub_status_to_suspended() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-suspended").await;
    let service = HubStatusService::new(pool.clone());
    let payment_svc = PaymentService::new(pool.clone());

    let due_date = Utc::now().date_naive() - chrono::Duration::days(15);
    payment_svc
        .create_payment(CreatePaymentRequest {
            hub_id,
            due_date,
            amount: 299.99,
            notes: None,
        })
        .await
        .unwrap();

    let changed = service.update_hub_status(hub_id, None).await.unwrap();
    assert_eq!(changed, Some("suspended".to_string()));

    cleanup_test_db(&pool).await;
}

/// Se o status não muda, `update_hub_status` deve retornar None.
#[tokio::test]
async fn test_update_hub_status_no_change() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-nochange").await;
    let service = HubStatusService::new(pool.clone());

    // Sem pagamentos em atraso → status permanece "active"
    let result = service.update_hub_status(hub_id, None).await.unwrap();
    assert!(result.is_none());

    cleanup_test_db(&pool).await;
}
