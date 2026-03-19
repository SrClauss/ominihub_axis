use anyhow::Result;
use axum::Router;
use sqlx::PgPool;

pub mod config;
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

use services::event_broadcaster::EventBroadcaster;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub broadcaster: EventBroadcaster,
    pub jwt_private_key: String,
    pub jwt_public_key: String,
}

pub async fn create_app() -> Result<Router> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = db::pool::create_pool(&database_url).await?;

    run_migrations(&pool).await?;

    let (private_key, public_key) = get_or_generate_rsa_keys();

    let broadcaster = EventBroadcaster::new();

    let state = AppState {
        db: pool.clone(),
        broadcaster: broadcaster.clone(),
        jwt_private_key: private_key,
        jwt_public_key: public_key,
    };

    // Spawn the heartbeat worker
    tokio::spawn(services::heartbeat_worker::run_heartbeat_worker(
        pool,
        broadcaster,
    ));

    Ok(routes::build_router(state))
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
    ];

    for sql in &migration_files {
        sqlx::raw_sql(sql).execute(pool).await?;
    }

    tracing::info!("Migrations completed successfully");
    Ok(())
}
