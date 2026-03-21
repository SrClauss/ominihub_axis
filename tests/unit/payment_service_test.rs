use axis_core::models::franchise_payment::{CreatePaymentRequest, MarkPaidRequest};
use axis_core::services::payment_service::PaymentService;
use chrono::{NaiveDate, Utc};

use crate::common::{cleanup_test_db, create_test_hub, setup_test_db};

#[tokio::test]
async fn test_create_payment() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-create").await;

    let service = PaymentService::new(pool.clone());

    let req = CreatePaymentRequest {
        hub_id,
        due_date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
        amount: 299.99,
        notes: Some("Pagamento de teste".to_string()),
    };

    let payment = service.create_payment(req).await.unwrap();

    assert_eq!(payment.hub_id, hub_id);
    assert_eq!(payment.amount, 299.99);
    assert_eq!(payment.status, "pending");
    assert!(payment.paid_at.is_none());

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_mark_paid() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-markpaid").await;
    let service = PaymentService::new(pool.clone());

    let req = CreatePaymentRequest {
        hub_id,
        due_date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
        amount: 299.99,
        notes: None,
    };
    let payment = service.create_payment(req).await.unwrap();

    let mark_req = MarkPaidRequest {
        payment_method: Some("credit_card".to_string()),
        transaction_id: Some("TXN123".to_string()),
        notes: Some("Pago via teste".to_string()),
    };

    let updated = service.mark_paid(payment.id, mark_req).await.unwrap().unwrap();

    assert_eq!(updated.status, "paid");
    assert!(updated.paid_at.is_some());
    assert_eq!(updated.payment_method.as_deref(), Some("credit_card"));
    assert_eq!(updated.transaction_id.as_deref(), Some("TXN123"));

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_mark_overdue_payments() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-overdue").await;
    let service = PaymentService::new(pool.clone());

    // Criar pagamento com data de vencimento ontem
    let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
    let req = CreatePaymentRequest {
        hub_id,
        due_date: yesterday,
        amount: 299.99,
        notes: None,
    };
    service.create_payment(req).await.unwrap();

    // Marcar como vencido
    let overdue_rows = service.mark_overdue_payments().await.unwrap();

    assert_eq!(overdue_rows.len(), 1);
    assert_eq!(overdue_rows[0].0, hub_id); // primeiro elemento da tupla é hub_id

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_list_payments_by_hub() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub-list").await;
    let service = PaymentService::new(pool.clone());

    for i in 1..=3 {
        let req = CreatePaymentRequest {
            hub_id,
            due_date: NaiveDate::from_ymd_opt(2026, i, 1).unwrap(),
            amount: 299.99,
            notes: None,
        };
        service.create_payment(req).await.unwrap();
    }

    let payments = service.list_payments(Some(hub_id)).await.unwrap();
    assert_eq!(payments.len(), 3);

    cleanup_test_db(&pool).await;
}
