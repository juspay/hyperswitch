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
