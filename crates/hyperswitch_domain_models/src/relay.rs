use common_enums::enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    id_type, pii,
    types::{keymanager, MinorUnit},
};
use diesel_models::relay::RelayUpdateInternal;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Relay {
    pub id: id_type::RelayId,
    pub connector_resource_id: String,
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub profile_id: id_type::ProfileId,
    pub merchant_id: id_type::MerchantId,
    pub relay_type: enums::RelayType,
    pub request_data: Option<RelayData>,
    pub status: enums::RelayStatus,
    pub connector_reference_id: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub response_data: Option<pii::SecretSerdeValue>,
}

impl From<Relay> for api_models::relay::RelayResponse {
    fn from(value: Relay) -> Self {
        let error = value
            .error_code
            .zip(value.error_reason)
            .map(
                |(error_code, error_message)| api_models::relay::RelayError {
                    code: error_code,
                    message: error_message,
                },
            );

        let data = value.request_data.map(|relay_data| match relay_data {
            RelayData::Refund(relay_refund_request) => {
                api_models::relay::RelayData::Refund(api_models::relay::RelayRefundRequest {
                    amount: relay_refund_request.amount,
                    currency: relay_refund_request.currency,
                    reason: relay_refund_request.reason,
                })
            }
        });
        Self {
            id: value.id,
            status: value.status,
            error,
            connector_resource_id: value.connector_resource_id,
            connector_id: value.connector_id,
            profile_id: value.profile_id,
            relay_type: value.relay_type,
            data: data,
            connector_reference_id: value.connector_reference_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RelayData {
    Refund(RelayRefundData),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayRefundData {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub enum RelayUpdate {
    ErrorUpdate {
        error_code: String,
        error_reason: String,
        status: enums::RelayStatus,
    },
    StatusUpdate {
        connector_reference_id: Option<String>,
        status: common_enums::RelayStatus,
    },
}

// impl From<Result<Response, ErrorResponse>> for RelayUpdate {
//     fn from(value: Result<Response, ErrorResponse>) -> Self {
//         match value.response {
//         Err(error) => hyperswitch_domain_models::relay::RelayUpdate::ErrorUpdate {
//             error_code: error.code,
//             error_reason: error.message,
//             status: common_enums::RelayStatus::Failure,
//         },
//         Ok(response) => hyperswitch_domain_models::relay::RelayUpdate::StatusUpdate {
//             connector_reference_id: Some(response.connector_refund_id),
//             status: common_enums::RelayStatus::from(response.refund_status),
//         },
//     }
//     }
// }

impl From<RelayUpdate> for RelayUpdateInternal {
    fn from(value: RelayUpdate) -> Self {
        match value {
            RelayUpdate::ErrorUpdate {
                error_code,
                error_reason,
                status,
            } => Self {
                error_code: Some(error_code),
                error_reason: Some(error_reason),
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
                error_reason: None,
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
            error_reason: self.error_reason,
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
            relay_type: enums::RelayType::Refund,
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
            error_reason: item.error_reason,
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
            error_reason: self.error_reason,
            created_at: self.created_at,
            modified_at: self.modified_at,
            response_data: self.response_data,
        })
    }
}
