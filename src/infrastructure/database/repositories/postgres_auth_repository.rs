use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::domain::repositories::auth_repository::{
    ApiKeySummary, AuthIdentity, AuthRepository, AuthRepositoryError, TenantSummary,
};
use crate::infrastructure::database::models::{
    ApiKeyModel, NewApiKeyModel, NewTenantModel, TenantModel,
};
use crate::infrastructure::database::{DbPool, get_connection_from_pool};

pub struct PostgresAuthRepository {
    pool: DbPool,
}

impl PostgresAuthRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn resolve_key(
        &self,
        key_hash_param: &str,
    ) -> Result<Option<AuthIdentity>, AuthRepositoryError> {
        use crate::infrastructure::database::schema::api_keys::dsl as ak;
        use crate::infrastructure::database::schema::tenants::dsl as t;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let result = ak::api_keys
            .inner_join(t::tenants.on(ak::tenant_id.eq(t::id)))
            .filter(ak::key_hash.eq(key_hash_param))
            .filter(ak::revoked_at.is_null())
            .filter(t::is_active.eq(true))
            .select(ApiKeyModel::as_select())
            .first::<ApiKeyModel>(&mut conn)
            .optional()
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(result.map(|k| AuthIdentity {
            tenant_id: k.tenant_id,
            api_key_id: k.id,
        }))
    }

    async fn touch_key(&self, api_key_id: Uuid) -> Result<(), AuthRepositoryError> {
        use crate::infrastructure::database::schema::api_keys::dsl as ak;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        diesel::update(ak::api_keys.filter(ak::id.eq(api_key_id)))
            .set(ak::last_used_at.eq(chrono::Utc::now()))
            .execute(&mut conn)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn create_tenant(&self, name_param: &str) -> Result<TenantSummary, AuthRepositoryError> {
        use crate::infrastructure::database::schema::tenants::dsl as t;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let new_tenant = NewTenantModel {
            name: name_param.to_string(),
        };

        let inserted: TenantModel = diesel::insert_into(t::tenants)
            .values(&new_tenant)
            .get_result(&mut conn)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(TenantSummary {
            id: inserted.id,
            name: inserted.name,
            is_active: inserted.is_active,
            created_at: inserted.created_at,
        })
    }

    async fn create_api_key(
        &self,
        tenant_id_param: Uuid,
        name_param: Option<&str>,
        key_hash_param: &str,
        key_prefix_param: &str,
        scopes_param: &[String],
    ) -> Result<ApiKeySummary, AuthRepositoryError> {
        use crate::infrastructure::database::schema::api_keys::dsl as ak;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let new_key = NewApiKeyModel {
            tenant_id: tenant_id_param,
            key_hash: key_hash_param.to_string(),
            key_prefix: key_prefix_param.to_string(),
            name: name_param.map(|s| s.to_string()),
            scopes: scopes_param.to_vec(),
        };

        let inserted: ApiKeyModel = diesel::insert_into(ak::api_keys)
            .values(&new_key)
            .get_result(&mut conn)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(ApiKeySummary {
            id: inserted.id,
            tenant_id: inserted.tenant_id,
            name: inserted.name,
            prefix: inserted.key_prefix,
            scopes: inserted.scopes,
            last_used_at: inserted.last_used_at,
            revoked_at: inserted.revoked_at,
            created_at: inserted.created_at,
        })
    }

    async fn list_tenants(&self) -> Result<Vec<TenantSummary>, AuthRepositoryError> {
        use crate::infrastructure::database::schema::tenants::dsl as t;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let rows = t::tenants
            .order(t::created_at.desc())
            .select(TenantModel::as_select())
            .load::<TenantModel>(&mut conn)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| TenantSummary {
                id: r.id,
                name: r.name,
                is_active: r.is_active,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn list_api_keys(
        &self,
        tenant_id_param: Uuid,
    ) -> Result<Vec<ApiKeySummary>, AuthRepositoryError> {
        use crate::infrastructure::database::schema::api_keys::dsl as ak;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let rows = ak::api_keys
            .filter(ak::tenant_id.eq(tenant_id_param))
            .order(ak::created_at.desc())
            .select(ApiKeyModel::as_select())
            .load::<ApiKeyModel>(&mut conn)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|k| ApiKeySummary {
                id: k.id,
                tenant_id: k.tenant_id,
                name: k.name,
                prefix: k.key_prefix,
                scopes: k.scopes,
                last_used_at: k.last_used_at,
                revoked_at: k.revoked_at,
                created_at: k.created_at,
            })
            .collect())
    }

    async fn revoke_api_key(
        &self,
        tenant_id_param: Uuid,
        api_key_id_param: Uuid,
    ) -> Result<bool, AuthRepositoryError> {
        use crate::infrastructure::database::schema::api_keys::dsl as ak;

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        let updated = diesel::update(
            ak::api_keys
                .filter(ak::id.eq(api_key_id_param))
                .filter(ak::tenant_id.eq(tenant_id_param))
                .filter(ak::revoked_at.is_null()),
        )
        .set(ak::revoked_at.eq(chrono::Utc::now()))
        .execute(&mut conn)
        .map_err(|e| AuthRepositoryError::DatabaseError(e.to_string()))?;

        Ok(updated > 0)
    }
}
