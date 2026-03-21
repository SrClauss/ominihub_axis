use anyhow::Result;
use axum::Router;
use sqlx::PgPool;

pub mod config;
pub mod db;
pub mod handlers;
pub mod jobs;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_private_key: String,
    pub jwt_public_key: String,
    pub mercadopago_token: Option<String>,
    pub app_base_url: String,
}

pub async fn create_app() -> Result<Router> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = db::pool::create_pool(&database_url).await?;

    run_migrations(&pool).await?;

    let (private_key, public_key) = get_or_generate_rsa_keys();

    let mercadopago_token = std::env::var("MERCADOPAGO_ACCESS_TOKEN").ok();
    let app_base_url = std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let state = AppState {
        db: pool.clone(),
        jwt_private_key: private_key,
        jwt_public_key: public_key,
        mercadopago_token,
        app_base_url,
    };

    // Spawn the daily payment checker background job
    tokio::spawn(jobs::payment_checker::payment_check_worker(pool));

    Ok(routes::build_router(state))
}

/// Build a Router with an already-created pool and explicit RSA key pair.
/// Intended for use in integration tests so that the JWT key pair is known
/// to the test ahead of time.
pub fn build_test_router(
    pool: PgPool,
    jwt_private_key: String,
    jwt_public_key: String,
) -> Router {
    let state = AppState {
        db: pool,
        jwt_private_key,
        jwt_public_key,
        mercadopago_token: None,
        app_base_url: "http://localhost:8080".to_string(),
    };
    routes::build_router(state)
}

fn get_or_generate_rsa_keys() -> (String, String) {
    let private_key = std::env::var("JWT_PRIVATE_KEY").ok();
    let public_key = std::env::var("JWT_PUBLIC_KEY").ok();

    match (private_key, public_key) {
        (Some(priv_k), Some(pub_k)) => (priv_k, pub_k),
        _ => services::auth_service::generate_rsa_keys(),
    }
}

async fn run_migrations(pool: &PgPool) -> Result<()> {
    let migration_files = [
        include_str!("../migrations/001_create_users.sql"),
        include_str!("../migrations/002_create_hubs.sql"),
        include_str!("../migrations/003_create_coverage_versions.sql"),
        include_str!("../migrations/004_create_hub_validations.sql"),
        include_str!("../migrations/005_create_roaming_validations.sql"),
        include_str!("../migrations/006_add_user_hub_fk.sql"),
        include_str!("../migrations/20260321000003_create_drivers.sql"),
        include_str!("../migrations/20260321000006_create_blocked_entities.sql"),
        include_str!("../migrations/20260321000007_create_admins.sql"),
        include_str!("../migrations/20260321000008_create_admin_hub_access.sql"),
        include_str!("../migrations/20260321000009_create_hub_status.sql"),
        include_str!("../migrations/20260321001001_alter_hubs_billing.sql"),
        include_str!("../migrations/20260321001002_create_franchise_payments.sql"),
        include_str!("../migrations/20260321001003_create_payment_adjustments.sql"),
        include_str!("../migrations/20260321001004_create_hub_status_history.sql"),
        include_str!("../migrations/20260321001005_create_franchise_notifications.sql"),
        include_str!("../migrations/20260321001006_create_payment_webhook_logs.sql"),
    ];

    for sql in &migration_files {
        sqlx::raw_sql(sql).execute(pool).await?;
    }

    tracing::info!("Migrations completed successfully");
    Ok(())
}
