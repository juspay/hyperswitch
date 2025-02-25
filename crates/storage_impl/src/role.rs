
use common_utils::{id_type, errors::CustomResult};
use diesel_models::{
    enums,
    role as storage,
};
use router_env::{instrument, tracing};
use error_stack::report;
use sample::role::RoleInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> RoleInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        role.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id(&conn, role_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_role_by_role_id_in_lineage(
        &self,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        profile_id: &id_type::ProfileId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id_in_lineage(
            &conn,
            role_id,
            merchant_id,
            org_id,
            profile_id,
            tenant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_by_role_id_org_id_tenant_id(
        &self,
        role_id: &str,
        org_id: &id_type::OrganizationId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id_org_id_tenant_id(&conn, role_id, org_id, tenant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::update_by_role_id(&conn, role_id, role_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::delete_by_role_id(&conn, role_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //TODO: Remove once generic_list_roles_by_entity_type is stable
    #[instrument(skip_all)]
    async fn list_roles_for_org_by_parameters(
        &self,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        entity_type: Option<enums::EntityType>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::generic_roles_list_for_org(
            &conn,
            tenant_id.to_owned(),
            org_id.to_owned(),
            merchant_id.cloned(),
            entity_type,
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn generic_list_roles_by_entity_type(
        &self,
        payload: storage::ListRolesByEntityPayload,
        is_lineage_data_required: bool,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::generic_list_roles_by_entity_type(
            &conn,
            payload,
            is_lineage_data_required,
            tenant_id,
            org_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}