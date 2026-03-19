use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::event_broadcaster::{EventBroadcaster, WsEvent};

pub async fn run_heartbeat_worker(pool: PgPool, broadcaster: EventBroadcaster) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        interval.tick().await;

        match check_and_update_offline_hubs(&pool, &broadcaster).await {
            Ok(count) => {
                if count > 0 {
                    info!("Marked {} hubs as offline due to missed heartbeat", count);
                }
            }
            Err(e) => {
                error!("Heartbeat worker error: {}", e);
            }
        }
    }
}

async fn check_and_update_offline_hubs(
    pool: &PgPool,
    broadcaster: &EventBroadcaster,
) -> anyhow::Result<u64> {
    let stale_hubs = sqlx::query(
        r#"
        UPDATE hubs
        SET status = 'offline', updated_at = NOW()
        WHERE status = 'online'
          AND (last_heartbeat IS NULL OR last_heartbeat < NOW() - INTERVAL '60 seconds')
        RETURNING id, name, slug
        "#
    )
    .fetch_all(pool)
    .await?;

    let count = stale_hubs.len() as u64;

    for row in stale_hubs {
        use sqlx::Row;
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let slug: String = row.try_get("slug")?;
        broadcaster.broadcast(WsEvent {
            event: "hub.offline".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "hub_id": id.to_string(),
                "hub_name": name,
                "hub_slug": slug,
            }),
        });
    }

    Ok(count)
}
