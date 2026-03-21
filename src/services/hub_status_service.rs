use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::hub_status_history::HubStatusHistory;

pub struct HubStatusService {
    pool: PgPool,
}

/// Days-overdue thresholds for each degradation level.
const GRACE_AFTER_DAYS: i64 = 7;
const RESTRICTED_AFTER_DAYS: i64 = 10;
const SUSPENDED_AFTER_DAYS: i64 = 15;

impl HubStatusService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the current operational status for a hub from hub_status, or "active" if not found.
    pub async fn get_status(&self, hub_id: Uuid) -> Result<String> {
        let status = sqlx::query_scalar::<_, String>(
            "SELECT operational_status FROM hub_status WHERE hub_id = $1",
        )
        .bind(hub_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| "active".to_string());

        Ok(status)
    }

    /// Compute the new status based on how many days the oldest pending/overdue payment has been
    /// outstanding. Returns None if hub has no overdue payments (stays active).
    pub fn compute_status(overdue_days: i64) -> &'static str {
        match overdue_days {
            d if d <= GRACE_AFTER_DAYS => "active",
            d if d <= RESTRICTED_AFTER_DAYS => "grace",
            d if d < SUSPENDED_AFTER_DAYS => "restricted",
            _ => "suspended",
        }
    }

    /// Update the hub status based on its most overdue payment.
    /// Returns the new status if it changed, or None if unchanged.
    pub async fn update_hub_status(
        &self,
        hub_id: Uuid,
        admin_id: Option<Uuid>,
    ) -> Result<Option<String>> {
        // Find the maximum number of days any payment is overdue for this hub
        let max_overdue_days: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT MAX(EXTRACT(DAY FROM (NOW() - due_date::timestamptz))::bigint)
            FROM franchise_payments
            WHERE hub_id = $1
              AND status IN ('pending', 'overdue')
              AND due_date < CURRENT_DATE
            "#,
        )
        .bind(hub_id)
        .fetch_one(&self.pool)
        .await?;

        let new_status = match max_overdue_days {
            Some(days) => Self::compute_status(days),
            None => "active",
        };

        let old_status = self.get_status(hub_id).await?;

        if old_status == new_status {
            return Ok(None);
        }

        // Upsert hub_status
        sqlx::query(
            r#"
            INSERT INTO hub_status (hub_id, operational_status, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (hub_id)
            DO UPDATE SET operational_status = $2, updated_at = NOW()
            "#,
        )
        .bind(hub_id)
        .bind(new_status)
        .execute(&self.pool)
        .await?;

        // Record history
        sqlx::query(
            r#"
            INSERT INTO hub_status_history (hub_id, old_status, new_status, reason, changed_by)
            VALUES ($1, $2, $3, 'automatic_payment_check', $4)
            "#,
        )
        .bind(hub_id)
        .bind(&old_status)
        .bind(new_status)
        .bind(admin_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            hub_id = %hub_id,
            old_status = %old_status,
            new_status = %new_status,
            "Hub operational status changed"
        );

        Ok(Some(new_status.to_string()))
    }

    /// Manually set a hub status (admin action).
    pub async fn set_status(
        &self,
        hub_id: Uuid,
        new_status: &str,
        reason: Option<String>,
        admin_id: Option<Uuid>,
    ) -> Result<()> {
        let old_status = self.get_status(hub_id).await?;

        sqlx::query(
            r#"
            INSERT INTO hub_status (hub_id, operational_status, restriction_reason, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (hub_id)
            DO UPDATE SET operational_status = $2, restriction_reason = $3, updated_at = NOW()
            "#,
        )
        .bind(hub_id)
        .bind(new_status)
        .bind(&reason)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO hub_status_history (hub_id, old_status, new_status, reason, changed_by)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(hub_id)
        .bind(&old_status)
        .bind(new_status)
        .bind(reason)
        .bind(admin_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check all hubs and update their statuses based on payment history.
    /// Returns list of (hub_id, new_status) for hubs that changed.
    pub async fn check_all_hubs(&self) -> Result<Vec<(Uuid, String)>> {
        let hub_ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM hubs")
            .fetch_all(&self.pool)
            .await?;

        let mut changed = Vec::new();

        for hub_id in hub_ids {
            if let Some(new_status) = self.update_hub_status(hub_id, None).await? {
                changed.push((hub_id, new_status));
            }
        }

        Ok(changed)
    }

    pub async fn get_history(&self, hub_id: Uuid) -> Result<Vec<HubStatusHistory>> {
        let history = sqlx::query_as::<_, HubStatusHistory>(
            "SELECT * FROM hub_status_history WHERE hub_id = $1 ORDER BY changed_at DESC",
        )
        .bind(hub_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }
}
