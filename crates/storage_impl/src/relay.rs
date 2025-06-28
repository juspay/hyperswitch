use common_enums::RelayType;
use error_stack::ResultExt;
use hyperswitch_domain_models::relay::{Relay, RelayUpdate};
use common_utils::errors::CustomResult;
use common_utils::errors::ValidationError;
use masking::ExposeInterface;
use masking::Secret;
use common_utils::types::keymanager;

use crate::utils::ForeignFrom;

impl ForeignFrom<RelayUpdate> for diesel_models::relay::RelayUpdateInternal {
    fn foreign_from(value: RelayUpdate) -> Self {
        match value {
            RelayUpdate::ErrorUpdate {
                error_code,
                error_message,
                status,
            } => Self {
                error_code: Some(error_code),
                error_message: Some(error_message),
                connector_reference_id: None,
                status: Some(status),
                modified_at: common_utils::date_time::now(),
            },
            RelayUpdate::StatusUpdate {
                connector_reference_id,
                status,
            } => Self {
                connector_reference_id,
                status: Some(status),
                error_code: None,
                error_message: None,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Relay {
    type DstType = diesel_models::relay::Relay;
    type NewDstType = diesel_models::relay::RelayNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::relay::Relay {
            id: self.id,
            connector_resource_id: self.connector_resource_id,
            connector_id: self.connector_id,
            profile_id: self.profile_id,
            merchant_id: self.merchant_id,
            relay_type: self.relay_type,
            request_data: self
                .request_data
                .map(|data| {
                    serde_json::to_value(data).change_context(ValidationError::InvalidValue {
                        message: "Failed while decrypting business profile data".to_string(),
                    })
                })
                .transpose()?
                .map(Secret::new),
            status: self.status,
            connector_reference_id: self.connector_reference_id,
            error_code: self.error_code,
            error_message: self.error_message,
            created_at: self.created_at,
            modified_at: self.modified_at,
            response_data: self.response_data,
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: item.id,
            connector_resource_id: item.connector_resource_id,
            connector_id: item.connector_id,
            profile_id: item.profile_id,
            merchant_id: item.merchant_id,
            relay_type: RelayType::Refund,
            request_data: item
                .request_data
                .map(|data| {
                    serde_json::from_value(data.expose()).change_context(
                        ValidationError::InvalidValue {
                            message: "Failed while decrypting business profile data".to_string(),
                        },
                    )
                })
                .transpose()?,
            status: item.status,
            connector_reference_id: item.connector_reference_id,
            error_code: item.error_code,
            error_message: item.error_message,
            created_at: item.created_at,
            modified_at: item.modified_at,
            response_data: item.response_data,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::relay::RelayNew {
            id: self.id,
            connector_resource_id: self.connector_resource_id,
            connector_id: self.connector_id,
            profile_id: self.profile_id,
            merchant_id: self.merchant_id,
            relay_type: self.relay_type,
            request_data: self
                .request_data
                .map(|data| {
                    serde_json::to_value(data).change_context(ValidationError::InvalidValue {
                        message: "Failed while decrypting business profile data".to_string(),
                    })
                })
                .transpose()?
                .map(Secret::new),
            status: self.status,
            connector_reference_id: self.connector_reference_id,
            error_code: self.error_code,
            error_message: self.error_message,
            created_at: self.created_at,
            modified_at: self.modified_at,
            response_data: self.response_data,
        })
    }
}
