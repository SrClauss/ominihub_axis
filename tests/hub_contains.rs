use dotenvy::dotenv;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::env;
use uuid::Uuid;

async fn setup_pool() -> PgPool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to database");

    let migrations = [
        include_str!("../migrations/001_create_users.sql"),
        include_str!("../migrations/002_create_hubs.sql"),
        include_str!("../migrations/003_create_coverage_versions.sql"),
        include_str!("../migrations/004_create_hub_validations.sql"),
        include_str!("../migrations/005_create_roaming_validations.sql"),
        include_str!("../migrations/006_add_user_hub_fk.sql"),
    ];

    for sql in &migrations {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .expect("Failed to run migration");
    }

    pool
}

async fn insert_test_hub(pool: &PgPool, boundary_geojson: serde_json::Value) -> Uuid {
    let slug = format!("test-hub-{}", Uuid::new_v4());

    let row = sqlx::query(
        r#"
        INSERT INTO hubs (name, slug, boundary, api_url, admin_email, status, metadata)
        VALUES ($1, $2, ST_GeomFromGeoJSON($3)::geometry, $4, $5, 'online', $6)
        RETURNING id
        "#,
    )
    .bind("Test Hub")
    .bind(slug)
    .bind(boundary_geojson.to_string())
    .bind("https://example.com")
    .bind(Some("admin@example.com"))
    .bind(json!({}))
    .fetch_one(pool)
    .await
    .expect("Failed to insert hub");

    row.try_get("id").expect("Failed to get hub ID")
}

#[tokio::test]
async fn test_hub_contains_location_true() {
    let pool = setup_pool().await;

    let boundary_geojson = json!({
        "type": "Polygon",
        "coordinates": [
            [
                [-46.6600, -23.5700],
                [-46.6500, -23.5700],
                [-46.6500, -23.5600],
                [-46.6600, -23.5600],
                [-46.6600, -23.5700]
            ]
        ]
    });

    let hub_id = insert_test_hub(&pool, boundary_geojson).await;

    let row = sqlx::query(
        r#"
        SELECT ST_Contains(boundary, ST_SetSRID(ST_MakePoint($1, $2), 4326)) AS inside
        FROM hubs
        WHERE id = $3
        "#,
    )
    .bind(-46.6550)
    .bind(-23.5650)
    .bind(hub_id)
    .fetch_one(&pool)
    .await
    .expect("Query failed");

    let inside: bool = row.try_get("inside").expect("Missing inside column");
    assert!(inside, "Expected point to be inside the hub boundary");
}

#[tokio::test]
async fn test_hub_contains_location_false() {
    let pool = setup_pool().await;

    let boundary_geojson = json!({
        "type": "Polygon",
        "coordinates": [
            [
                [-46.6600, -23.5700],
                [-46.6500, -23.5700],
                [-46.6500, -23.5600],
                [-46.6600, -23.5600],
                [-46.6600, -23.5700]
            ]
        ]
    });

    let hub_id = insert_test_hub(&pool, boundary_geojson).await;

    let row = sqlx::query(
        r#"
        SELECT ST_Contains(boundary, ST_SetSRID(ST_MakePoint($1, $2), 4326)) AS inside
        FROM hubs
        WHERE id = $3
        "#,
    )
    .bind(-46.6700)
    .bind(-23.5800)
    .bind(hub_id)
    .fetch_one(&pool)
    .await
    .expect("Query failed");

    let inside: bool = row.try_get("inside").expect("Missing inside column");
    assert!(!inside, "Expected point to be outside the hub boundary");
}
