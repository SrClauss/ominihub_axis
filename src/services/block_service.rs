use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::blocked_entity::{BlockedEntity, BlockEntityRequest, EntityType};

pub struct BlockService {
    pool: PgPool,
}

impl BlockService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Bloqueia uma entidade (user ou driver). Se já existir, atualiza o bloqueio.
    pub async fn block_entity(
        &self,
        request: BlockEntityRequest,
        blocked_by: Uuid,
    ) -> Result<BlockedEntity> {
        let entity = sqlx::query_as::<_, BlockedEntity>(
            r#"
            INSERT INTO blocked_entities (entity_type, entity_id, blocked_by, reason, expires_at, hub_scope)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (entity_type, entity_id)
            DO UPDATE SET
                reason = EXCLUDED.reason,
                expires_at = EXCLUDED.expires_at,
                hub_scope = EXCLUDED.hub_scope,
                blocked_by = EXCLUDED.blocked_by,
                blocked_at = NOW()
            RETURNING *
            "#,
        )
        .bind(request.entity_type)
        .bind(request.entity_id)
        .bind(blocked_by)
        .bind(request.reason)
        .bind(request.expires_at)
        .bind(request.hub_scope)
        .fetch_one(&self.pool)
        .await?;

        Ok(entity)
    }

    /// Verifica se uma entidade está bloqueada, globalmente ou no hub especificado.
    pub async fn is_blocked(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        hub_id: Option<Uuid>,
    ) -> Result<bool> {
        let blocked = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM blocked_entities
                WHERE entity_type = $1
                  AND entity_id = $2
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (
                      ($3 IS NULL)            -- sem contexto de hub: qualquer bloqueio ativo
                      OR (hub_scope IS NULL)  -- bloqueio global
                      OR ($3 = ANY(hub_scope))-- bloqueio específico para este hub
                  )
            )
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(hub_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(blocked)
    }

    /// Remove o bloqueio de uma entidade.
    pub async fn unblock_entity(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM blocked_entities WHERE entity_type = $1 AND entity_id = $2",
        )
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
