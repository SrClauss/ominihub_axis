use axis_core::create_app;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "axis_core=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = create_app().await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("AXIS Core listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
