
use common_utils::{id_type, errors::CustomResult};
use diesel_models::{enums, user_role as storage};
use router_env::{instrument, tracing};
use error_stack::report;
use sample::user_role::{self, UserRoleInterface};

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> UserRoleInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        user_role
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::find_by_user_id_tenant_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            tenant_id.to_owned(),
            org_id.to_owned(),
            merchant_id.to_owned(),
            profile_id.to_owned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_tenant_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            tenant_id.to_owned(),
            org_id.to_owned(),
            merchant_id.cloned(),
            profile_id.cloned(),
            update,
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::delete_by_user_id_tenant_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            tenant_id.to_owned(),
            org_id.to_owned(),
            merchant_id.to_owned(),
            profile_id.to_owned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_user_roles_by_user_id<'a>(
        &self,
        payload: user_role::ListUserRolesByUserIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::generic_user_roles_list_for_user(
            &conn,
            payload.user_id.to_owned(),
            payload.tenant_id.to_owned(),
            payload.org_id.cloned(),
            payload.merchant_id.cloned(),
            payload.profile_id.cloned(),
            payload.entity_id.cloned(),
            payload.status,
            payload.version,
            payload.limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_user_roles_by_user_id_across_tenants(
        &self,
        user_id: &str,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::list_user_roles_by_user_id_across_tenants(
            &conn,
            user_id.to_owned(),
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: user_role::ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::generic_user_roles_list_for_org_and_extra(
            &conn,
            payload.user_id.cloned(),
            payload.tenant_id.to_owned(),
            payload.org_id.to_owned(),
            payload.merchant_id.cloned(),
            payload.profile_id.cloned(),
            payload.version,
            payload.limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}