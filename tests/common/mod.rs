use axis_core::models::user::User;
use axis_core::services::auth_service::{generate_tokens, generate_rsa_keys};
use sqlx::PgPool;
use uuid::Uuid;

/// Conecta ao banco de dados de teste e executa as migrations.
/// Usa `DATABASE_URL` ou, se não disponível, `TEST_DATABASE_URL`.
pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost/axis_test".to_string()
        });

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Falha ao conectar ao banco de dados de teste");

    let migration_files = [
        include_str!("../../migrations/001_create_users.sql"),
        include_str!("../../migrations/002_create_hubs.sql"),
        include_str!("../../migrations/003_create_coverage_versions.sql"),
        include_str!("../../migrations/004_create_hub_validations.sql"),
        include_str!("../../migrations/005_create_roaming_validations.sql"),
        include_str!("../../migrations/006_add_user_hub_fk.sql"),
        include_str!("../../migrations/20260321000003_create_drivers.sql"),
        include_str!("../../migrations/20260321000006_create_blocked_entities.sql"),
        include_str!("../../migrations/20260321000007_create_admins.sql"),
        include_str!("../../migrations/20260321000008_create_admin_hub_access.sql"),
        include_str!("../../migrations/20260321000009_create_hub_status.sql"),
        include_str!("../../migrations/20260321001001_alter_hubs_billing.sql"),
        include_str!("../../migrations/20260321001002_create_franchise_payments.sql"),
        include_str!("../../migrations/20260321001003_create_payment_adjustments.sql"),
        include_str!("../../migrations/20260321001004_create_hub_status_history.sql"),
        include_str!("../../migrations/20260321001005_create_franchise_notifications.sql"),
        include_str!("../../migrations/20260321001006_create_payment_webhook_logs.sql"),
    ];

    for sql in &migration_files {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .expect("Falha ao executar migration");
    }

    pool
}

/// Remove todos os dados de teste do banco, preservando a estrutura.
pub async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query(
        "TRUNCATE users, hubs, franchise_payments, payment_adjustments, \
         hub_status, hub_status_history, franchise_notifications, \
         payment_webhook_logs, admins, drivers, blocked_entities CASCADE",
    )
    .execute(pool)
    .await
    .expect("Falha ao limpar banco de dados de teste");
}

/// Cria um usuário de teste com email e role especificados.
/// A senha padrão é "test123".
pub async fn create_test_user(pool: &PgPool, email: &str, role: &str) -> User {
    let password_hash = bcrypt::hash("test123", bcrypt::DEFAULT_COST).unwrap();

    sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, role) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("Falha ao criar usuário de teste")
}

/// Cria um hub de teste com nome e slug especificados.
/// Retorna o UUID do hub criado.
pub async fn create_test_hub(pool: &PgPool, name: &str, slug: &str) -> Uuid {
    let boundary = serde_json::json!({
        "type": "Polygon",
        "coordinates": [[
            [-46.6333, -23.5505],
            [-46.6333, -23.5405],
            [-46.6233, -23.5405],
            [-46.6233, -23.5505],
            [-46.6333, -23.5505]
        ]]
    });

    sqlx::query_scalar(
        r#"
        INSERT INTO hubs (name, slug, boundary, api_url, status, metadata)
        VALUES ($1, $2, ST_GeomFromGeoJSON($3)::geometry, 'http://localhost:3000', 'online', '{}')
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(slug)
    .bind(boundary.to_string())
    .fetch_one(pool)
    .await
    .expect("Falha ao criar hub de teste")
}

/// Cria um hub de teste vinculado ao email de um franqueado.
/// Necessário para testes que usam `franchisee_list_payments` e similares.
pub async fn create_test_hub_for_franchisee(
    pool: &PgPool,
    name: &str,
    slug: &str,
    admin_email: &str,
) -> Uuid {
    let boundary = serde_json::json!({
        "type": "Polygon",
        "coordinates": [[
            [-46.6333, -23.5505],
            [-46.6333, -23.5405],
            [-46.6233, -23.5405],
            [-46.6233, -23.5505],
            [-46.6333, -23.5505]
        ]]
    });

    sqlx::query_scalar(
        r#"
        INSERT INTO hubs (name, slug, boundary, api_url, admin_email, status, metadata)
        VALUES ($1, $2, ST_GeomFromGeoJSON($3)::geometry, 'http://localhost:3000', $4, 'online', '{}')
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(slug)
    .bind(boundary.to_string())
    .bind(admin_email)
    .fetch_one(pool)
    .await
    .expect("Falha ao criar hub de teste para franqueado")
}

/// Gera um JWT de acesso assinado com a chave privada fornecida.
pub fn generate_test_jwt(user_id: Uuid, role: &str, private_key: &str) -> String {
    use chrono::Utc;

    let user = User {
        id: user_id,
        email: "test@test.com".to_string(),
        password_hash: "hash".to_string(),
        role: role.to_string(),
        home_hub_id: None,
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let (token, _) = generate_tokens(&user, private_key).unwrap();
    token
}

/// Gera um par de chaves RSA e retorna (chave_privada, chave_publica) em formato PEM.
pub fn test_rsa_keys() -> (String, String) {
    generate_rsa_keys()
}
