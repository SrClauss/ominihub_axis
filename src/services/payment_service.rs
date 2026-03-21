use anyhow::Result;
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::franchise_payment::{
    CreatePaymentRequest, FranchisePayment, MarkPaidRequest, PaymentReport, PaymentReportByStatus,
};
use crate::models::payment_adjustment::{CreateAdjustmentRequest, PaymentAdjustment};

pub struct PaymentService {
    pool: PgPool,
}

impl PaymentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_payment(&self, req: CreatePaymentRequest) -> Result<FranchisePayment> {
        let payment = sqlx::query_as::<_, FranchisePayment>(
            r#"
            INSERT INTO franchise_payments (hub_id, due_date, amount, notes)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(req.hub_id)
        .bind(req.due_date)
        .bind(req.amount)
        .bind(req.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn list_payments(&self, hub_id: Option<Uuid>) -> Result<Vec<FranchisePayment>> {
        let payments = match hub_id {
            Some(id) => {
                sqlx::query_as::<_, FranchisePayment>(
                    "SELECT * FROM franchise_payments WHERE hub_id = $1 ORDER BY due_date DESC",
                )
                .bind(id)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, FranchisePayment>(
                    "SELECT * FROM franchise_payments ORDER BY due_date DESC",
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(payments)
    }

    pub async fn get_payment(&self, id: Uuid) -> Result<Option<FranchisePayment>> {
        let payment = sqlx::query_as::<_, FranchisePayment>(
            "SELECT * FROM franchise_payments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn mark_paid(&self, id: Uuid, req: MarkPaidRequest) -> Result<Option<FranchisePayment>> {
        let payment = sqlx::query_as::<_, FranchisePayment>(
            r#"
            UPDATE franchise_payments
            SET status = 'paid',
                paid_at = NOW(),
                payment_method = $2,
                transaction_id = $3,
                notes = COALESCE($4, notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(req.payment_method)
        .bind(req.transaction_id)
        .bind(req.notes)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn mark_overdue_payments(&self) -> Result<Vec<Uuid>> {
        let hub_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE franchise_payments
            SET status = 'overdue', updated_at = NOW()
            WHERE status = 'pending'
              AND due_date < CURRENT_DATE
            RETURNING hub_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(hub_ids)
    }

    pub async fn generate_monthly_charges(&self) -> Result<i64> {
        let today: NaiveDate = Utc::now().date_naive();
        let due_date = today;

        // Generate payments for hubs that don't have a pending/overdue payment this month
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            WITH hubs_needing_payment AS (
                SELECT h.id AS hub_id, h.monthly_fee
                FROM hubs h
                WHERE NOT EXISTS (
                    SELECT 1 FROM franchise_payments fp
                    WHERE fp.hub_id = h.id
                      AND fp.status IN ('pending', 'overdue')
                      AND DATE_TRUNC('month', fp.due_date) = DATE_TRUNC('month', $1::date)
                )
            ),
            inserted AS (
                INSERT INTO franchise_payments (hub_id, due_date, amount)
                SELECT hub_id, $1, monthly_fee
                FROM hubs_needing_payment
                RETURNING id
            )
            SELECT COUNT(*) FROM inserted
            "#,
        )
        .bind(due_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    pub async fn create_adjustment(
        &self,
        payment_id: Uuid,
        req: CreateAdjustmentRequest,
        admin_id: Uuid,
    ) -> Result<Option<PaymentAdjustment>> {
        let adj_type = format!("{:?}", req.adjustment_type).to_lowercase();

        let adjustment = sqlx::query_as::<_, PaymentAdjustment>(
            r#"
            INSERT INTO payment_adjustments (payment_id, adjustment_type, amount, reason, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(payment_id)
        .bind(adj_type)
        .bind(req.amount)
        .bind(req.reason)
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(adjustment)
    }

    pub async fn get_adjustments_for_payment(&self, payment_id: Uuid) -> Result<Vec<PaymentAdjustment>> {
        let adjustments = sqlx::query_as::<_, PaymentAdjustment>(
            "SELECT * FROM payment_adjustments WHERE payment_id = $1 ORDER BY created_at DESC",
        )
        .bind(payment_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(adjustments)
    }

    pub async fn get_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<PaymentReport> {
        let total_franchises: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM hubs")
                .fetch_one(&self.pool)
                .await?;

        let total_revenue: f64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0.0)
            FROM franchise_payments
            WHERE status = 'paid'
              AND ($1::date IS NULL OR due_date >= $1)
              AND ($2::date IS NULL OR due_date <= $2)
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        let pending_revenue: f64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0.0)
            FROM franchise_payments
            WHERE status IN ('pending', 'overdue')
              AND ($1::date IS NULL OR due_date >= $1)
              AND ($2::date IS NULL OR due_date <= $2)
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        let overdue_franchises: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT hub_id) FROM franchise_payments WHERE status = 'overdue'",
        )
        .fetch_one(&self.pool)
        .await?;

        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM hub_status WHERE operational_status = 'active'",
        )
        .fetch_one(&self.pool)
        .await?;

        let grace_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM hub_status WHERE operational_status = 'grace'",
        )
        .fetch_one(&self.pool)
        .await?;

        let restricted_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM hub_status WHERE operational_status = 'restricted'",
        )
        .fetch_one(&self.pool)
        .await?;

        let suspended_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM hub_status WHERE operational_status = 'suspended'",
        )
        .fetch_one(&self.pool)
        .await?;

        let active_franchises: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM hub_status WHERE operational_status = 'active'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(PaymentReport {
            total_franchises,
            active_franchises,
            overdue_franchises,
            total_revenue,
            pending_revenue,
            by_status: PaymentReportByStatus {
                active: active_count,
                grace: grace_count,
                restricted: restricted_count,
                suspended: suspended_count,
            },
        })
    }

    pub async fn set_gateway_url(&self, payment_id: Uuid, url: &str) -> Result<()> {
        sqlx::query(
            "UPDATE franchise_payments SET gateway_payment_url = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(url)
        .bind(payment_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
