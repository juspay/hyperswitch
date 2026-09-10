use common_utils::{
    crypto::Encryptable,
    custom_serde, date_time,
    errors::{CustomResult, ValidationError},
    id_type,
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use hyperswitch_masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::type_encryption::{crypto_operation, CryptoOperation};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Resource {
    pub id: id_type::ResourceId,
    pub resource_type: String,
    pub scope: String,
    pub scope_id: String,
    pub data: serde_json::Value,
    pub encrypted_data: Option<Encryptable<Secret<String>>>,
    pub created_by: String,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
}

fn key_identifier_from_scope_id(
    scope_id: &str,
) -> CustomResult<keymanager::Identifier, ValidationError> {
    let organization_id = id_type::OrganizationId::try_from_string(scope_id.to_owned())?;
    Ok(keymanager::Identifier::Merchant(
        organization_id.as_merchant_key_identifier()?,
    ))
}

impl Resource {
    pub fn key_identifier(&self) -> CustomResult<keymanager::Identifier, ValidationError> {
        key_identifier_from_scope_id(&self.scope_id)
    }

    pub fn identifier_for_diesel(
        item: &diesel_models::resource::Resource,
    ) -> CustomResult<keymanager::Identifier, ValidationError> {
        key_identifier_from_scope_id(&item.scope_id)
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Resource {
    type DstType = diesel_models::resource::Resource;
    type NewDstType = diesel_models::resource::ResourceNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::resource::Resource {
            id: self.id,
            resource_type: self.resource_type,
            scope: self.scope,
            scope_id: self.scope_id,
            data: self.data,
            encrypted_data: self.encrypted_data.map(Encryptable::into),
            created_by: self.created_by,
            created_at: self.created_at,
            modified_at: self.modified_at,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let organization_id = id_type::OrganizationId::try_from_string(item.scope_id.clone())?;
        let identifier =
            keymanager::Identifier::Merchant(organization_id.as_merchant_key_identifier()?);

        let encrypted_data = crypto_operation(
            state,
            common_utils::type_name!(Self::DstType),
            CryptoOperation::DecryptOptional(item.encrypted_data),
            identifier,
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_optionaloperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting resource".to_string(),
        })?;

        Ok(Self {
            id: item.id,
            resource_type: item.resource_type,
            scope: item.scope,
            scope_id: item.scope_id,
            data: item.data,
            encrypted_data,
            created_by: item.created_by,
            created_at: item.created_at,
            modified_at: item.modified_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::resource::ResourceNew {
            id: self.id,
            resource_type: self.resource_type,
            scope: self.scope,
            scope_id: self.scope_id,
            data: self.data,
            encrypted_data: self.encrypted_data.map(Encryptable::into),
            created_by: self.created_by,
            created_at: date_time::now(),
            modified_at: date_time::now(),
        })
    }
}

pub struct ResourceDataUpdate {
    pub data: serde_json::Value,
}

impl From<ResourceDataUpdate> for diesel_models::resource::ResourceUpdateInternal {
    fn from(value: ResourceDataUpdate) -> Self {
        Self {
            data: Some(value.data),
            modified_at: date_time::now(),
        }
    }
}

#[async_trait::async_trait]
pub trait ResourceInterface {
    type Error;

    async fn insert_linked_resource(
        &self,
        resource: Resource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Resource, Self::Error>;

    async fn find_linked_resource_by_id(
        &self,
        id: id_type::ResourceId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Resource, Self::Error>;

    async fn find_resource_scope_id(
        &self,
        id: id_type::ResourceId,
    ) -> CustomResult<String, Self::Error>;

    async fn list_linked_resources_by_scope_id_and_resource_type(
        &self,
        scope_id: String,
        resource_type: String,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<Resource>, Self::Error>;

    async fn update_linked_resource_data(
        &self,
        id: id_type::ResourceId,
        update: ResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Resource, Self::Error>;
}
