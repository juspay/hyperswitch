use common_enums::enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    id_type::{self, GenerateId},
    pii,
    types::{keymanager, MinorUnit},
};
// use diesel_models::relay::RelayUpdateInternal;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{router_data::ErrorResponse, router_response_types};

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
    pub error_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub response_data: Option<pii::SecretSerdeValue>,
}

impl Relay {
    pub fn new(
        relay_request: &api_models::relay::RelayRequest,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> Self {
        let relay_id = id_type::RelayId::generate();
        Self {
            id: relay_id.clone(),
            connector_resource_id: relay_request.connector_resource_id.clone(),
            connector_id: relay_request.connector_id.clone(),
            profile_id: profile_id.clone(),
            merchant_id: merchant_id.clone(),
            relay_type: common_enums::RelayType::Refund,
            request_data: relay_request.data.clone().map(From::from),
            status: common_enums::RelayStatus::Created,
            connector_reference_id: None,
            error_code: None,
            error_message: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            response_data: None,
        }
    }
}

impl From<api_models::relay::RelayData> for RelayData {
    fn from(relay: api_models::relay::RelayData) -> Self {
        match relay {
            api_models::relay::RelayData::Refund(relay_refund_request) => {
                Self::Refund(RelayRefundData {
                    amount: relay_refund_request.amount,
                    currency: relay_refund_request.currency,
                    reason: relay_refund_request.reason,
                })
            }
        }
    }
}

impl From<api_models::relay::RelayRefundRequestData> for RelayRefundData {
    fn from(relay: api_models::relay::RelayRefundRequestData) -> Self {
        Self {
            amount: relay.amount,
            currency: relay.currency,
            reason: relay.reason,
        }
    }
}

impl RelayUpdate {
    pub fn from(
        response: Result<router_response_types::RefundsResponseData, ErrorResponse>,
    ) -> Self {
        match response {
            Err(error) => Self::ErrorUpdate {
                error_code: error.code,
                error_message: error.reason.unwrap_or(error.message),
                status: common_enums::RelayStatus::Failure,
            },
            Ok(response) => Self::StatusUpdate {
                connector_reference_id: Some(response.connector_refund_id),
                status: common_enums::RelayStatus::from(response.refund_status),
            },
        }
    }
}

impl From<RelayData> for api_models::relay::RelayData {
    fn from(relay: RelayData) -> Self {
        match relay {
            RelayData::Refund(relay_refund_request) => {
                Self::Refund(api_models::relay::RelayRefundRequestData {
                    amount: relay_refund_request.amount,
                    currency: relay_refund_request.currency,
                    reason: relay_refund_request.reason,
                })
            }
        }
    }
}

impl From<Relay> for api_models::relay::RelayResponse {
    fn from(value: Relay) -> Self {
        let error = value
            .error_code
            .zip(value.error_message)
            .map(
                |(error_code, error_message)| api_models::relay::RelayError {
                    code: error_code,
                    message: error_message,
                },
            );

        let data = value.request_data.map(|relay_data| match relay_data {
            RelayData::Refund(relay_refund_request) => {
                api_models::relay::RelayData::Refund(api_models::relay::RelayRefundRequestData {
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
            data,
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
        error_message: String,
        status: enums::RelayStatus,
    },
    StatusUpdate {
        connector_reference_id: Option<String>,
        status: common_enums::RelayStatus,
    },
}
