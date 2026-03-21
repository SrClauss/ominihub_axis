use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::franchise_notification::{FranchiseNotification, NotificationType};

pub struct NotificationService {
    pool: PgPool,
}

impl NotificationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn send(
        &self,
        hub_id: Uuid,
        notification_type: NotificationType,
        message: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<FranchiseNotification> {
        let ntype = notification_type.to_string();

        let notification = sqlx::query_as::<_, FranchiseNotification>(
            r#"
            INSERT INTO franchise_notifications (hub_id, notification_type, message, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(hub_id)
        .bind(&ntype)
        .bind(&message)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            hub_id = %hub_id,
            notification_type = %ntype,
            "Notification sent"
        );

        Ok(notification)
    }

    pub async fn notify_payment_due(&self, hub_id: Uuid, days_remaining: i64) -> Result<()> {
        let msg = format!(
            "Sua mensalidade vence em {} dia(s). Efetue o pagamento para evitar restrições.",
            days_remaining
        );
        self.send(hub_id, NotificationType::PaymentDue, msg, None).await?;
        Ok(())
    }

    pub async fn notify_payment_overdue(&self, hub_id: Uuid, overdue_days: i64) -> Result<()> {
        let msg = format!(
            "Sua mensalidade está vencida há {} dia(s). Regularize para evitar suspensão do serviço.",
            overdue_days
        );
        self.send(hub_id, NotificationType::PaymentOverdue, msg, None).await?;
        Ok(())
    }

    pub async fn notify_grace_period(&self, hub_id: Uuid) -> Result<()> {
        let msg = "Seu hub entrou em período de carência (grace). Você tem até 3 dias para regularizar antes de entrar em modo restrito.".to_string();
        self.send(hub_id, NotificationType::GracePeriodStarted, msg, None).await?;
        Ok(())
    }

    pub async fn notify_restricted(&self, hub_id: Uuid) -> Result<()> {
        let msg = "Seu hub está em modo RESTRITO. Novos motoristas não poderão se cadastrar. Regularize o pagamento imediatamente.".to_string();
        self.send(hub_id, NotificationType::RestrictedMode, msg, None).await?;
        Ok(())
    }

    pub async fn notify_suspended(&self, hub_id: Uuid) -> Result<()> {
        let msg = "Seu hub foi SUSPENSO por inadimplência. O serviço foi interrompido. Entre em contato para regularizar.".to_string();
        self.send(hub_id, NotificationType::Suspended, msg, None).await?;
        Ok(())
    }

    pub async fn notify_payment_received(&self, hub_id: Uuid, amount: f64) -> Result<()> {
        let msg = format!(
            "Pagamento de R$ {:.2} recebido com sucesso. Obrigado!",
            amount
        );
        self.send(
            hub_id,
            NotificationType::PaymentReceived,
            msg,
            Some(serde_json::json!({ "amount": amount })),
        )
        .await?;
        Ok(())
    }

    pub async fn notify_payment_failed(&self, hub_id: Uuid, reason: &str) -> Result<()> {
        let msg = format!("Falha no processamento do pagamento: {}.", reason);
        self.send(hub_id, NotificationType::PaymentFailed, msg, None).await?;
        Ok(())
    }

    pub async fn list_for_hub(&self, hub_id: Uuid) -> Result<Vec<FranchiseNotification>> {
        let notifs = sqlx::query_as::<_, FranchiseNotification>(
            "SELECT * FROM franchise_notifications WHERE hub_id = $1 ORDER BY sent_at DESC",
        )
        .bind(hub_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(notifs)
    }

    pub async fn mark_read(&self, notification_id: Uuid) -> Result<Option<FranchiseNotification>> {
        let notif = sqlx::query_as::<_, FranchiseNotification>(
            r#"
            UPDATE franchise_notifications
            SET read_at = NOW()
            WHERE id = $1 AND read_at IS NULL
            RETURNING *
            "#,
        )
        .bind(notification_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(notif)
    }
}
