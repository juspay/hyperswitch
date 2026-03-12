use common_enums::enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    id_type::{self, GenerateId},
    pii,
    types::{keymanager, MinorUnit},
};
use diesel_models::relay::RelayUpdateInternal;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    errors::api_error_response::ApiErrorResponse, router_data::ErrorResponse, router_response_types,
};

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
            relay_type: relay_request.relay_type,
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
            api_models::relay::RelayData::Capture(relay_capture_request) => {
                Self::Capture(RelayCaptureData {
                    authorized_amount: relay_capture_request.authorized_amount,
                    amount_to_capture: relay_capture_request.amount_to_capture,
                    currency: relay_capture_request.currency,
                    capture_method: relay_capture_request.capture_method,
                })
            }
            api_models::relay::RelayData::IncrementalAuthorization(
                relay_incremental_authorization_request,
            ) => Self::IncrementalAuthorization(RelayIncrementalAuthorizationData {
                total_amount: relay_incremental_authorization_request.total_amount,
                additional_amount: relay_incremental_authorization_request.additional_amount,
                currency: relay_incremental_authorization_request.currency,
            }),
            api_models::relay::RelayData::Void(relay_void_request) => Self::Void(RelayVoidData {
                amount: relay_void_request.amount,
                currency: relay_void_request.currency,
                cancellation_reason: relay_void_request.cancellation_reason,
            }),
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

impl From<api_models::relay::RelayCaptureRequestData> for RelayCaptureData {
    fn from(relay: api_models::relay::RelayCaptureRequestData) -> Self {
        Self {
            authorized_amount: relay.authorized_amount,
            amount_to_capture: relay.amount_to_capture,
            currency: relay.currency,
            capture_method: relay.capture_method,
        }
    }
}

impl From<api_models::relay::RelayIncrementalAuthorizationRequestData>
    for RelayIncrementalAuthorizationData
{
    fn from(relay: api_models::relay::RelayIncrementalAuthorizationRequestData) -> Self {
        Self {
            total_amount: relay.total_amount,
            additional_amount: relay.additional_amount,
            currency: relay.currency,
        }
    }
}

impl From<api_models::relay::RelayVoidRequestData> for RelayVoidData {
    fn from(relay: api_models::relay::RelayVoidRequestData) -> Self {
        Self {
            amount: relay.amount,
            currency: relay.currency,
            cancellation_reason: relay.cancellation_reason,
        }
    }
}

impl RelayUpdate {
    pub fn from_refund_response(
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

    pub fn try_from_capture_response(
        (status, connector_resource_id, response): (
            common_enums::AttemptStatus,
            String,
            Result<router_response_types::PaymentsResponseData, ErrorResponse>,
        ),
    ) -> CustomResult<Self, ApiErrorResponse> {
        match response {
            Err(error) => {
                let relay_status = common_enums::RelayStatus::from(status);

                match relay_status {
                    common_enums::RelayStatus::Failure => Ok(Self::ErrorUpdate {
                        error_code: error.code,
                        error_message: error.reason.unwrap_or(error.message),
                        status: relay_status,
                    }),
                    common_enums::RelayStatus::Created
                    | common_enums::RelayStatus::Pending
                    | common_enums::RelayStatus::Success => Ok(Self::StatusUpdate {
                        connector_reference_id: None,
                        status: relay_status,
                    }),
                }
            }
            Ok(response) => match response {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    ..
                } => Ok(Self::StatusUpdate {
                    connector_reference_id: resource_id.get_optional_response_id(),
                    status: common_enums::RelayStatus::from(status),
                }),
                router_response_types::PaymentsResponseData::MultipleCaptureResponse {
                    capture_sync_response_list,
                } => {
                    let data = capture_sync_response_list
                        .get(&connector_resource_id)
                        .ok_or(ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to find connector_transaction_id in capture_response_list",
                        )?;

                    match data.to_owned() {
                        router_response_types::CaptureSyncResponse::Success {
                            resource_id,
                            status,
                            ..
                        } => Ok(Self::StatusUpdate {
                            connector_reference_id: resource_id.get_optional_response_id(),
                            status: common_enums::RelayStatus::from(status),
                        }),
                        router_response_types::CaptureSyncResponse::Error {
                            code,
                            reason,
                            message,
                            ..
                        } => Ok(Self::ErrorUpdate {
                            error_code: code,
                            error_message: reason.unwrap_or(message),
                            status: common_enums::RelayStatus::Failure,
                        }),
                    }
                }
                _ => Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("Payment Response Not Supported"),
            },
        }
    }

    pub fn try_from_incremental_authorization_response(
        response: Result<router_response_types::PaymentsResponseData, ErrorResponse>,
    ) -> CustomResult<Self, ApiErrorResponse> {
        match response {
            Err(error) => Ok(Self::ErrorUpdate {
                error_code: error.code,
                error_message: error.reason.unwrap_or(error.message),
                status: common_enums::RelayStatus::Failure,
            }),
            Ok(response) => match response {
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    connector_authorization_id,
                    status,
                    error_code,
                    error_message,
                } => match error_code {
                    Some(error_code) => Ok(Self::ErrorUpdate {
                        error_code: error_code.clone(),
                        error_message: error_message.unwrap_or(error_code),
                        status: common_enums::RelayStatus::Failure,
                    }),
                    None => Ok(Self::StatusUpdate {
                        connector_reference_id: connector_authorization_id,
                        status: common_enums::RelayStatus::from(status),
                    }),
                },
                _ => Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("Payment Response Not Supported"),
            },
        }
    }

    pub fn try_from_void_response(
        (status, response): (
            common_enums::AttemptStatus,
            Result<router_response_types::PaymentsResponseData, ErrorResponse>,
        ),
    ) -> CustomResult<Self, ApiErrorResponse> {
        match response {
            Err(error) => Ok(Self::ErrorUpdate {
                error_code: error.code,
                error_message: error.reason.unwrap_or(error.message),
                status: common_enums::RelayStatus::Failure,
            }),
            Ok(response) => match response {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    ..
                } => Ok(Self::StatusUpdate {
                    connector_reference_id: resource_id.get_optional_response_id(),
                    status: common_enums::RelayStatus::get_void_status(status),
                }),
                _ => Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("Payment Response Not Supported"),
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
            RelayData::Capture(relay_capture_request) => {
                Self::Capture(api_models::relay::RelayCaptureRequestData {
                    authorized_amount: relay_capture_request.authorized_amount,
                    amount_to_capture: relay_capture_request.amount_to_capture,
                    currency: relay_capture_request.currency,
                    capture_method: relay_capture_request.capture_method,
                })
            }
            RelayData::IncrementalAuthorization(relay_incremental_authorization_request) => {
                Self::IncrementalAuthorization(
                    api_models::relay::RelayIncrementalAuthorizationRequestData {
                        total_amount: relay_incremental_authorization_request.total_amount,
                        additional_amount: relay_incremental_authorization_request
                            .additional_amount,
                        currency: relay_incremental_authorization_request.currency,
                    },
                )
            }
            RelayData::Void(relay_void_request) => {
                Self::Void(api_models::relay::RelayVoidRequestData {
                    amount: relay_void_request.amount,
                    currency: relay_void_request.currency,
                    cancellation_reason: relay_void_request.cancellation_reason,
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
            RelayData::Capture(relay_capture_request) => {
                api_models::relay::RelayData::Capture(api_models::relay::RelayCaptureRequestData {
                    authorized_amount: relay_capture_request.authorized_amount,
                    amount_to_capture: relay_capture_request.amount_to_capture,
                    currency: relay_capture_request.currency,
                    capture_method: relay_capture_request.capture_method,
                })
            }
            RelayData::IncrementalAuthorization(relay_incremental_authorization_request) => {
                api_models::relay::RelayData::IncrementalAuthorization(
                    api_models::relay::RelayIncrementalAuthorizationRequestData {
                        total_amount: relay_incremental_authorization_request.total_amount,
                        additional_amount: relay_incremental_authorization_request
                            .additional_amount,
                        currency: relay_incremental_authorization_request.currency,
                    },
                )
            }
            RelayData::Void(relay_void_request) => {
                api_models::relay::RelayData::Void(api_models::relay::RelayVoidRequestData {
                    amount: relay_void_request.amount,
                    currency: relay_void_request.currency,
                    cancellation_reason: relay_void_request.cancellation_reason,
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
    Capture(RelayCaptureData),
    IncrementalAuthorization(RelayIncrementalAuthorizationData),
    Void(RelayVoidData),
}

impl RelayData {
    pub fn parse_relay_data(
        value: Option<pii::SecretSerdeValue>,
        relay_type: enums::RelayType,
    ) -> CustomResult<Option<Self>, ValidationError> {
        match value {
            Some(data) => match relay_type {
                enums::RelayType::Capture => Ok(Some(Self::Capture(RelayCaptureData::from_value(
                    data.expose(),
                )?))),
                enums::RelayType::Refund => Ok(Some(Self::Refund(RelayRefundData::from_value(
                    data.expose(),
                )?))),
                enums::RelayType::IncrementalAuthorization => {
                    Ok(Some(Self::IncrementalAuthorization(
                        RelayIncrementalAuthorizationData::from_value(data.expose())?,
                    )))
                }
                enums::RelayType::Void => {
                    Ok(Some(Self::Void(RelayVoidData::from_value(data.expose())?)))
                }
            },
            None => Ok(None),
        }
    }

    pub fn get_refund_data(&self) -> CustomResult<RelayRefundData, ApiErrorResponse> {
        match self.clone() {
            Self::Refund(refund_data) => Ok(refund_data),
            Self::Capture(_) | Self::IncrementalAuthorization(_) | Self::Void(_) => {
                Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("relay data does not contain relay refund data")
            }
        }
    }

    pub fn get_capture_data(&self) -> CustomResult<RelayCaptureData, ApiErrorResponse> {
        match self.clone() {
            Self::Capture(capture_data) => Ok(capture_data),
            Self::Refund(_) | Self::IncrementalAuthorization(_) | Self::Void(_) => {
                Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("relay data does not contain relay capture data")
            }
        }
    }

    pub fn get_incremental_authorization_data(
        &self,
    ) -> CustomResult<RelayIncrementalAuthorizationData, ApiErrorResponse> {
        match self.clone() {
            Self::IncrementalAuthorization(incremental_authorization_data) => {
                Ok(incremental_authorization_data)
            }
            Self::Refund(_) | Self::Capture(_) | Self::Void(_) => Err(
                ApiErrorResponse::InternalServerError,
            )
            .attach_printable("relay data does not contain relay incremental authorization data"),
        }
    }

    pub fn get_void_data(&self) -> CustomResult<RelayVoidData, ApiErrorResponse> {
        match self.clone() {
            Self::Void(void_data) => Ok(void_data),
            Self::Refund(_) | Self::Capture(_) | Self::IncrementalAuthorization(_) => {
                Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("relay data does not contain relay void data")
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayRefundData {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub reason: Option<String>,
}

impl RelayRefundData {
    pub fn from_value(value: serde_json::Value) -> CustomResult<Self, ValidationError> {
        value
            .parse_value("RelayRefundData")
            .change_context(ValidationError::InvalidValue {
                message: "Failed while deserializing RelayRefundData".to_string(),
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayCaptureData {
    pub authorized_amount: MinorUnit,
    pub amount_to_capture: MinorUnit,
    pub currency: enums::Currency,
    pub capture_method: Option<enums::CaptureMethod>,
}

impl RelayCaptureData {
    pub fn from_value(value: serde_json::Value) -> CustomResult<Self, ValidationError> {
        value
            .parse_value("RelayCaptureData")
            .change_context(ValidationError::InvalidValue {
                message: "Failed while deserializing RelayCaptureData".to_string(),
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayIncrementalAuthorizationData {
    pub total_amount: MinorUnit,
    pub additional_amount: MinorUnit,
    pub currency: enums::Currency,
}

impl RelayIncrementalAuthorizationData {
    pub fn from_value(value: serde_json::Value) -> CustomResult<Self, ValidationError> {
        value
            .parse_value("RelayIncrementalAuthorizationData")
            .change_context(ValidationError::InvalidValue {
                message: "Failed while deserializing RelayIncrementalAuthorizationData".to_string(),
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayVoidData {
    pub amount: Option<MinorUnit>,
    pub currency: Option<enums::Currency>,
    pub cancellation_reason: Option<String>,
}

impl RelayVoidData {
    pub fn from_value(value: serde_json::Value) -> CustomResult<Self, ValidationError> {
        value
            .parse_value("RelayVoidData")
            .change_context(ValidationError::InvalidValue {
                message: "Failed while deserializing RelayVoidData".to_string(),
            })
    }
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

impl From<RelayUpdate> for RelayUpdateInternal {
    fn from(value: RelayUpdate) -> Self {
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
            relay_type: item.relay_type,
            request_data: RelayData::parse_relay_data(item.request_data, item.relay_type)?,
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
