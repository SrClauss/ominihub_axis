use std::time::Duration;

use sqlx::PgPool;

use crate::services::{
    hub_status_service::HubStatusService,
    notification_service::NotificationService,
    payment_service::PaymentService,
};

/// Background worker that runs every 24 hours and:
/// 1. Generates monthly charges for hubs that don't have a pending payment this month.
/// 2. Marks pending payments as overdue when their due_date has passed.
/// 3. Updates each hub's operational status based on overdue days.
/// 4. Sends notifications for status transitions.
pub async fn payment_check_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(86_400));

    loop {
        interval.tick().await;

        tracing::info!("payment_checker: starting daily payment check");

        let payment_svc = PaymentService::new(pool.clone());
        let status_svc = HubStatusService::new(pool.clone());
        let notif_svc = NotificationService::new(pool.clone());

        // Step 1: Generate monthly charges
        match payment_svc.generate_monthly_charges().await {
            Ok(count) => tracing::info!("payment_checker: generated {} new charges", count),
            Err(e) => tracing::error!("payment_checker: error generating charges: {}", e),
        }

        // Step 2: Mark overdue payments
        match payment_svc.mark_overdue_payments().await {
            Ok(hub_ids) => {
                tracing::info!("payment_checker: marked {} payments as overdue", hub_ids.len());
                for hub_id in &hub_ids {
                    let _ = notif_svc.notify_payment_overdue(*hub_id, 1).await;
                }
            }
            Err(e) => tracing::error!("payment_checker: error marking overdue: {}", e),
        }

        // Step 3: Update hub statuses based on overdue days and send notifications
        match status_svc.check_all_hubs().await {
            Ok(changed) => {
                for (hub_id, new_status) in changed {
                    tracing::info!(
                        hub_id = %hub_id,
                        new_status = %new_status,
                        "payment_checker: hub status changed"
                    );

                    let result = match new_status.as_str() {
                        "grace" => notif_svc.notify_grace_period(hub_id).await,
                        "restricted" => notif_svc.notify_restricted(hub_id).await,
                        "suspended" => notif_svc.notify_suspended(hub_id).await,
                        _ => Ok(()),
                    };

                    if let Err(e) = result {
                        tracing::error!(
                            hub_id = %hub_id,
                            "payment_checker: error sending notification: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => tracing::error!("payment_checker: error checking hub statuses: {}", e),
        }

        tracing::info!("payment_checker: daily check complete");
    }
}
