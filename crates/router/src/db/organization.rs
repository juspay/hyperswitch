use common_utils::errors::CustomResult;
use diesel_models::organization as storage;
use error_stack::IntoReport;

use crate::{connection, core::errors, services::Store};

#[async_trait::async_trait]
pub trait OrganizationInterface {
    async fn insert_organization(
        &self,
        organization: storage::OrganizationNew,
    ) -> CustomResult<storage::Organization, errors::StorageError>;

    async fn find_organization_by_org_id(
        &self,
        org_id: &str,
    ) -> CustomResult<storage::Organization, errors::StorageError>;

    async fn update_organization_by_org_id(
        &self,
        user_id: &str,
        update: storage::OrganizationUpdate,
    ) -> CustomResult<storage::Organization, errors::StorageError>;
}

#[async_trait::async_trait]
impl OrganizationInterface for Store {
        /// Asynchronously inserts a new organization into the database and returns the inserted organization if successful. 
    ///
    /// # Arguments
    /// * `organization` - The new organization to be inserted into the database.
    ///
    /// # Returns
    /// Returns a CustomResult containing the inserted organization if successful, otherwise returns a StorageError.
    ///
    async fn insert_organization(
        &self,
        organization: storage::OrganizationNew,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        organization
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds an organization by its ID in the database.
    /// 
    /// # Arguments
    /// 
    /// * `org_id` - A reference to a string representing the organization ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the found `storage::Organization` if successful, otherwise an `errors::StorageError`.
    /// 
    async fn find_organization_by_org_id(
        &self,
        org_id: &str,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Organization::find_by_org_id(&conn, org_id.to_string())
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates an organization using its ID and the specified update information.
    async fn update_organization_by_org_id(
        &self,
        org_id: &str,
        update: storage::OrganizationUpdate,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::Organization::update_by_org_id(&conn, org_id.to_string(), update)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl OrganizationInterface for super::MockDb {
        /// Inserts a new organization into the storage. If the organization with the same org_id already exists, returns a `StorageError::DuplicateValue` error.
    /// 
    /// # Arguments
    /// * `organization` - The new organization to be inserted into the storage.
    /// 
    /// # Returns
    /// Returns a `CustomResult` containing the inserted organization if successful, otherwise returns a `StorageError`.
    /// 
    async fn insert_organization(
        &self,
        organization: storage::OrganizationNew,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let mut organizations = self.organizations.lock().await;

        if organizations
            .iter()
            .any(|org| org.org_id == organization.org_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "org_id",
                key: None,
            })?
        }
        let org = storage::Organization {
            org_id: organization.org_id.clone(),
            org_name: organization.org_name,
        };
        organizations.push(org.clone());
        Ok(org)
    }

        /// Asynchronously finds an organization by its organization ID. 
    /// Returns a Result containing the found Organization or a StorageError if the organization is not found.
    async fn find_organization_by_org_id(
        &self,
        org_id: &str,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let organizations = self.organizations.lock().await;

        organizations
            .iter()
            .find(|org| org.org_id == org_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No organization available for org_id = {org_id}"
                ))
                .into(),
            )
    }

        /// Asynchronously updates an organization with the specified org_id using the provided OrganizationUpdate.
    ///
    /// # Arguments
    ///
    /// * `org_id` - A reference to a string representing the organization id.
    /// * `update` - A storage::OrganizationUpdate struct containing the updates to apply to the organization.
    ///
    /// # Returns
    ///
    /// A CustomResult containing the updated storage::Organization if the organization is found, otherwise an errors::StorageError.
    ///
    async fn update_organization_by_org_id(
        &self,
        org_id: &str,
        update: storage::OrganizationUpdate,
    ) -> CustomResult<storage::Organization, errors::StorageError> {
        let mut organizations = self.organizations.lock().await;

        organizations
            .iter_mut()
            .find(|org| org.org_id == org_id)
            .map(|org| match &update {
                storage::OrganizationUpdate::Update { org_name } => storage::Organization {
                    org_name: org_name.clone(),
                    ..org.to_owned()
                },
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No organization available for org_id = {org_id}"
                ))
                .into(),
            )
    }
}
