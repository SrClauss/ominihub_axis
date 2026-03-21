use axis_core::models::blocked_entity::{BlockEntityRequest, EntityType};
use axis_core::services::block_service::BlockService;

use crate::common::{cleanup_test_db, create_test_user, setup_test_db};

#[tokio::test]
async fn test_block_entity() {
    let pool = setup_test_db().await;
    let user = create_test_user(&pool, "blocked-b@test.com", "driver").await;
    let admin = create_test_user(&pool, "admin-b@test.com", "admin").await;

    let service = BlockService::new(pool.clone());

    let req = BlockEntityRequest {
        entity_type: EntityType::User,
        entity_id: user.id,
        reason: "Violação de termos de teste".to_string(),
        expires_at: None,
        hub_scope: None,
    };

    let blocked = service.block_entity(req, admin.id).await.unwrap();

    assert_eq!(blocked.entity_id, user.id);
    assert_eq!(blocked.reason, "Violação de termos de teste");
    assert!(blocked.expires_at.is_none()); // bloqueio permanente

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_is_blocked() {
    let pool = setup_test_db().await;
    let user = create_test_user(&pool, "blocked-i@test.com", "driver").await;
    let admin = create_test_user(&pool, "admin-i@test.com", "admin").await;
    let service = BlockService::new(pool.clone());

    // Inicialmente, o usuário não deve estar bloqueado
    let initially_blocked = service
        .is_blocked(EntityType::User, user.id, None)
        .await
        .unwrap();
    assert!(!initially_blocked);

    // Bloquear o usuário
    let req = BlockEntityRequest {
        entity_type: EntityType::User,
        entity_id: user.id,
        reason: "Teste".to_string(),
        expires_at: None,
        hub_scope: None,
    };
    service.block_entity(req, admin.id).await.unwrap();

    // Agora deve estar bloqueado
    let now_blocked = service
        .is_blocked(EntityType::User, user.id, None)
        .await
        .unwrap();
    assert!(now_blocked);

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_unblock_entity() {
    let pool = setup_test_db().await;
    let user = create_test_user(&pool, "blocked-u@test.com", "driver").await;
    let admin = create_test_user(&pool, "admin-u@test.com", "admin").await;
    let service = BlockService::new(pool.clone());

    // Bloquear
    let req = BlockEntityRequest {
        entity_type: EntityType::User,
        entity_id: user.id,
        reason: "Teste".to_string(),
        expires_at: None,
        hub_scope: None,
    };
    service.block_entity(req, admin.id).await.unwrap();

    // Verificar bloqueado
    assert!(service
        .is_blocked(EntityType::User, user.id, None)
        .await
        .unwrap());

    // Desbloquear
    service
        .unblock_entity(EntityType::User, user.id)
        .await
        .unwrap();

    // Verificar desbloqueado
    assert!(!service
        .is_blocked(EntityType::User, user.id, None)
        .await
        .unwrap());

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_block_entity_twice_updates() {
    let pool = setup_test_db().await;
    let user = create_test_user(&pool, "blocked-d@test.com", "driver").await;
    let admin = create_test_user(&pool, "admin-d@test.com", "admin").await;
    let service = BlockService::new(pool.clone());

    // Primeiro bloqueio
    service
        .block_entity(
            BlockEntityRequest {
                entity_type: EntityType::User,
                entity_id: user.id,
                reason: "Primeira razão".to_string(),
                expires_at: None,
                hub_scope: None,
            },
            admin.id,
        )
        .await
        .unwrap();

    // Segundo bloqueio com razão diferente (deve atualizar via ON CONFLICT)
    let updated = service
        .block_entity(
            BlockEntityRequest {
                entity_type: EntityType::User,
                entity_id: user.id,
                reason: "Segunda razão".to_string(),
                expires_at: None,
                hub_scope: None,
            },
            admin.id,
        )
        .await
        .unwrap();

    assert_eq!(updated.reason, "Segunda razão");

    // Deve existir apenas um registro
    assert!(service
        .is_blocked(EntityType::User, user.id, None)
        .await
        .unwrap());

    cleanup_test_db(&pool).await;
}
