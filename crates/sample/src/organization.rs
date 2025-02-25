
// use error_stack::report;
// use router_env::{instrument, tracing};

// use hyperswitch_domain_models::errors;
use common_utils::{errors::CustomResult, id_type};
use diesel_models::{organization as storage};

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait OrganizationInterface {
    type Error;
    async fn insert_organization(
        &self,
        organization: storage::OrganizationNew,
    ) -> CustomResult<storage::Organization, Self::Error>;

    async fn find_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Organization, Self::Error>;

    async fn update_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
        update: storage::OrganizationUpdate,
    ) -> CustomResult<storage::Organization, Self::Error>;
}

// #[async_trait::async_trait]
// impl OrganizationInterface for super::MockDb {
//     async fn insert_organization(
//         &self,
//         organization: storage::OrganizationNew,
//     ) -> CustomResult<storage::Organization, errors::StorageError> {
//         let mut organizations = self.organizations.lock().await;

//         if organizations
//             .iter()
//             .any(|org| org.get_organization_id() == organization.get_organization_id())
//         {
//             Err(errors::StorageError::DuplicateValue {
//                 entity: "org_id",
//                 key: None,
//             })?
//         }
//         let org = storage::Organization::new(organization);
//         organizations.push(org.clone());
//         Ok(org)
//     }

//     async fn find_organization_by_org_id(
//         &self,
//         org_id: &id_type::OrganizationId,
//     ) -> CustomResult<storage::Organization, errors::StorageError> {
//         let organizations = self.organizations.lock().await;

//         organizations
//             .iter()
//             .find(|org| org.get_organization_id() == *org_id)
//             .cloned()
//             .ok_or(
//                 errors::StorageError::ValueNotFound(format!(
//                     "No organization available for org_id = {:?}",
//                     org_id
//                 ))
//                 .into(),
//             )
//     }

//     async fn update_organization_by_org_id(
//         &self,
//         org_id: &id_type::OrganizationId,
//         update: storage::OrganizationUpdate,
//     ) -> CustomResult<storage::Organization, errors::StorageError> {
//         let mut organizations = self.organizations.lock().await;

//         organizations
//             .iter_mut()
//             .find(|org| org.get_organization_id() == *org_id)
//             .map(|org| match &update {
//                 storage::OrganizationUpdate::Update {
//                     organization_name,
//                     organization_details,
//                     metadata,
//                 } => {
//                     organization_name
//                         .as_ref()
//                         .map(|org_name| org.set_organization_name(org_name.to_owned()));
//                     organization_details.clone_into(&mut org.organization_details);
//                     metadata.clone_into(&mut org.metadata);
//                     org
//                 }
//             })
//             .ok_or(
//                 errors::StorageError::ValueNotFound(format!(
//                     "No organization available for org_id = {:?}",
//                     org_id
//                 ))
//                 .into(),
//             )
//             .cloned()
//     }
// }
