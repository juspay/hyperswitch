use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{to_connector_meta, CardData},
    core::errors,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelcimPaymentsRequest {
    amount: i64,
    currency: enums::Currency,
    ip_address: Secret<String>,
    // ip_address: Secret<String, common_utils::pii::IpAddress>,
    card_data: HelcimCard,
    billing_address: HelcimBillingAddress,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelcimBillingAddress {
    name: Secret<String>,
    street1: String,
    postal_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelcimCard {
    card_number: cards::CardNumber,
    card_expiry: Secret<String>,
    card_c_v_v: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for HelcimPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card_data = HelcimCard {
                    card_expiry: req_card
                        .get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                    card_number: req_card.card_number,
                    card_c_v_v: req_card.card_cvc,
                };
                // let ip_address = item.request.get_browser_info()?.get_ip_address()?;
                let billing_address = HelcimBillingAddress {
                    name: req_card.card_holder_name,
                    street1: "Jump Street 21".to_string(),
                    postal_code: "H0H0H0".to_string(),
                };
                Ok(Self {
                    amount: item.request.amount,
                    currency: item.request.currency,
                    ip_address: Secret::new("127.0.0.1".to_string()),
                    card_data,
                    billing_address,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct HelcimAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for HelcimAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HelcimPaymentStatus {
    Approved,
    Declined,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HelcimTransactionType {
    Purchase,
    PreAuth,
    Capture,
    Verify,
}

impl From<HelcimPaymentsResponse> for enums::AttemptStatus {
    fn from(item: HelcimPaymentsResponse) -> Self {
        match item.transaction_type {
            HelcimTransactionType::Purchase => match item.status {
                HelcimPaymentStatus::Approved => Self::Charged,
                HelcimPaymentStatus::Declined => Self::Failure,
            },
            HelcimTransactionType::PreAuth => match item.status {
                HelcimPaymentStatus::Approved => Self::Authorized,
                HelcimPaymentStatus::Declined => Self::AuthorizationFailed,
            },
            HelcimTransactionType::Capture => match item.status {
                HelcimPaymentStatus::Approved => Self::Charged, //Is this the correct status PartialCharged
                HelcimPaymentStatus::Declined => Self::CaptureFailed,
            },
            HelcimTransactionType::Verify => match item.status {
                HelcimPaymentStatus::Approved => Self::AuthenticationSuccessful,
                HelcimPaymentStatus::Declined => Self::AuthenticationFailed,
            },
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelcimPaymentsResponse {
    amount: i64,
    status: HelcimPaymentStatus,
    transaction_id: u64,
    #[serde(rename = "type")]
    transaction_type: HelcimTransactionType,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_string(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelcimMetaData {
    pub capture_id: u64,
}

// impl utils::MultipleCaptureSyncResponse for HelcimPaymentsResponse {
//     fn get_connector_capture_id(&self) -> String {
//         self.transaction_id.to_string()
//     }

//     fn get_capture_attempt_status(&self) -> diesel_models::enums::AttemptStatus {
//         enums::AttemptStatus::from(self.to_owned())
//     }

//     fn is_capture_response(&self) -> bool {
//         true
//     }

//     fn get_amount_captured(&self) -> Option<i64> {
//         Some(self.amount)
//     }
//     fn get_connector_reference_id(&self) -> Option<String> {
//         None
//     }
// }

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.data.request.sync_type {
            types::SyncRequestType::SinglePaymentSync => Ok(Self {
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transaction_id.to_string(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                status: enums::AttemptStatus::from(item.response),
                ..item.data
            }),
            types::SyncRequestType::MultipleCaptureSync(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "manual multiple capture sync".to_string(),
                )
                .into())
                // let capture_sync_response_list =
                //     utils::construct_captures_response_hashmap(vec![item.response]);
                // Ok(Self {
                //     response: Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                //         capture_sync_response_list,
                //     }),
                //     ..item.data
                // })
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimCaptureRequest {
    pre_auth_transaction_id: u64,
    amount: i64,
    ip_address: Secret<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for HelcimCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            pre_auth_transaction_id: item
                .request
                .connector_transaction_id
                .parse::<u64>()
                .into_report()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            amount: item.request.amount_to_capture,
            ip_address: Secret::new("127.0.0.1".to_string()),
        })
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_metadata = Some(serde_json::json!(HelcimMetaData {
            capture_id: item.response.transaction_id,
        }));
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_string(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimRefundRequest {
    amount: i64,
    original_transaction_id: u64,
    ip_address: Secret<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for HelcimRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let helcim_meta_data: HelcimMetaData =
            to_connector_meta(item.request.connector_metadata.clone())?;
        let original_transaction_id = helcim_meta_data.capture_id;
        Ok(Self {
            amount: item.request.refund_amount,
            original_transaction_id,
            ip_address: Secret::new("127.0.0.1".to_string()),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HelcimRefundTransactionType {
    Refund,
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    amount: i64,
    status: HelcimPaymentStatus,
    transaction_id: u64,
    #[serde(rename = "type")]
    transaction_type: HelcimRefundTransactionType,
}

impl From<RefundResponse> for enums::RefundStatus {
    fn from(item: RefundResponse) -> Self {
        match item.transaction_type {
            HelcimRefundTransactionType::Refund => match item.status {
                HelcimPaymentStatus::Approved => Self::Success,
                HelcimPaymentStatus::Declined => Self::Failure,
            },
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct HelcimErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
