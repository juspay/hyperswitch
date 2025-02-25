use common_utils::{errors::CustomResult, id_type};
use diesel_models::organization as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::organization::OrganizationInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> OrganizationInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_organization(
        &self,
        organization: storage::OrganizationNew,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        organization
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Organization::find_by_org_id(&conn, org_id.to_owned())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
        update: storage::OrganizationUpdate,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::Organization::update_by_org_id(&conn, org_id.to_owned(), update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
