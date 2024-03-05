use common_enums::Currency;
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct PlaidRouterData<T> {
    pub amount: Option<f64>, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> TryFrom<(Option<f64>, T)> for PlaidRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (Option<f64>, T)) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct PlaidPaymentsRequest {
    amount: PlaidAmount,
    recipient_id: String,
    reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule: Option<PlaidSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<PlaidOptions>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidAmount {
    currency: Currency,
    value: f64,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidSchedule {
    interval: String,
    interval_execution_day: String,
    start_date: String,
    end_date: Option<String>,
    adjusted_start_date: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidOptions {
    request_refund_details: bool,
    iban: Option<String>,
    bacs: Option<PlaidBacs>,
    scheme: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidBacs {
    acount: Secret<String>,
    sort_code: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenRequest {
    client_name: String,
    country_codes: Vec<String>,
    language: String,
    products: Vec<String>,
    user: User,
    payment_initiation: PlaidPaymentInitiation,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct User {
    pub client_user_id: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidPaymentInitiation {
    payment_id: String,
}

impl TryFrom<&PlaidRouterData<&types::PaymentsAuthorizeRouterData>> for PlaidPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlaidRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::BankRedirect(ref bank) => match bank {
                api_models::payments::BankRedirectData::OpenBanking { .. } => {
                    let amount =
                        item.amount
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "amount",
                            })?;
                    let currency = item.router_data.request.currency;
                    let reference = "Some ref".to_string();
                    let recipient_val = item
                        .router_data
                        .connector_meta_data
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "connector_customer",
                        })?
                        .peek()
                        .clone();

                    let recipient_type =
                        serde_json::from_value::<types::MerchantRecipientData>(recipient_val)
                            .into_report()
                            .change_context(errors::ConnectorError::ParsingFailed)?;
                    let recipient_id = match recipient_type {
                        types::MerchantRecipientData::RecipientId(id) => Ok(id.peek().to_string()),
                        _ => Err(errors::ConnectorError::NotSupported {
                            message: "recipient_id not found, other methods".to_string(),
                            connector: "plaid",
                        }),
                    }
                    .into_report()?;

                    Ok(Self {
                        amount: PlaidAmount {
                            currency,
                            value: amount,
                        },
                        reference,
                        recipient_id,
                        schedule: None,
                        options: None,
                    })
                }
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment method type".to_string())
                        .into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&PlaidRouterData<&types::PaymentsSyncRouterData>> for PlaidSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlaidRouterData<&types::PaymentsSyncRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.connector_transaction_id {
            types::ResponseId::ConnectorTransactionId(ref id) => Ok(Self {
                payment_id: id.clone(),
            }),
            _ => Err(
                errors::ConnectorError::NotImplemented("ResponseId for Plaid".to_string()).into(),
            ),
        }
    }
}

impl TryFrom<&PlaidRouterData<&types::PaymentsPostProcessingRouterData>> for PlaidLinkTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlaidRouterData<&types::PaymentsPostProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::BankRedirect(ref bank) => match bank {
                api_models::payments::BankRedirectData::OpenBanking { .. } => Ok(Self {
                    // discuss this with folks
                    client_name: "Hyperswitch".to_string(),
                    country_codes: vec!["GB".to_string()],
                    language: "en".to_string(),
                    products: vec!["payment_initiation".to_string()],
                    user: User {
                        client_user_id: item
                            .router_data
                            .request
                            .customer_id
                            .clone()
                            .unwrap_or("default cust".to_string()),
                    },
                    payment_initiation: PlaidPaymentInitiation {
                        payment_id: item
                            .router_data
                            .request
                            .connector_transaction_id
                            .clone()
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "connector_transaction_id",
                            })?,
                    },
                }),
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment method type".to_string())
                        .into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PlaidAuthType {
    pub client_id: Secret<String>,
    pub secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::OpenBankingAuth {
                api_key,
                key1,
                merchant_data,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(strum::Display)]
pub enum PlaidPaymentStatus {
    PaymentStatusInputNeeded,
    PaymentStatusInitiatied,
    PaymentStatusInsuficientFunds,
    PaymentStatusFailed,
    PaymentStatusBlcoked,
    PaymentStatusCancelled,
    PaymentStatusExecuted,
    PaymentStatusSettled,
    PaymentStatusEstablished,
    PaymentStatusRejected,
    #[default]
    PaymentStatusAuthorising,
}

impl From<PlaidPaymentStatus> for enums::AttemptStatus {
    fn from(item: PlaidPaymentStatus) -> Self {
        match item {
            // Double check these with someone
            PlaidPaymentStatus::PaymentStatusAuthorising => Self::Authorizing,
            PlaidPaymentStatus::PaymentStatusBlcoked => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStatusCancelled => Self::Voided,
            PlaidPaymentStatus::PaymentStatusEstablished => Self::Authorized,
            PlaidPaymentStatus::PaymentStatusExecuted => Self::Authorized,
            PlaidPaymentStatus::PaymentStatusFailed => Self::Failure,
            PlaidPaymentStatus::PaymentStatusInitiatied => Self::AuthenticationPending,
            PlaidPaymentStatus::PaymentStatusInputNeeded => Self::AuthenticationPending,
            PlaidPaymentStatus::PaymentStatusInsuficientFunds => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStatusRejected => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStatusSettled => Self::Charged,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaidPaymentsResponse {
    status: PlaidPaymentStatus,
    payment_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.payment_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_id),
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidLinkTokenResponse {
    link_token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let session_token = Some(api::OpenBankingSessionToken {
            open_banking_session_token: item.response.link_token,
        });

        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::PostProcessingResponse { session_token }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidSyncRequest {
    payment_id: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidSyncResponse {
    payment_id: String,
    amount: PlaidAmount,
    status: PlaidPaymentStatus,
    recipient_id: String,
    reference: String,
    last_status_update: String,
    adjusted_reference: Option<String>,
    schedule: Option<PlaidSchedule>,
    iban: Option<String>,
    bacs: Option<PlaidBacs>,
    scheme: Option<String>,
    adjusted_scheme: Option<String>,
    request_id: String,
    // TODO: add refund related objects
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());
        Ok(Self {
            status,
            response: if is_payment_failure(status) {
                Err(types::ErrorResponse {
                    code: item.response.status.clone().to_string(),
                    message: item.response.status.clone().to_string(),
                    reason: Some(item.response.status.to_string()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.payment_id),
                })
            } else {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.payment_id.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.payment_id),
                    incremental_authorization_allowed: None,
                })
            },
            ..item.data
        })
    }
}

// TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PlaidRefundRequest {
    pub amount: f64,
}

impl<F> TryFrom<&PlaidRouterData<&types::RefundsRouterData<F>>> for PlaidRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PlaidRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidErrorResponse {
    pub display_message: Option<String>,
    pub error_code: Option<String>,
    pub error_message: String,
    pub error_type: Option<String>,
}

fn is_payment_failure(status: enums::AttemptStatus) -> bool {
    match status {
        common_enums::AttemptStatus::AuthenticationFailed
        | common_enums::AttemptStatus::AuthorizationFailed
        | common_enums::AttemptStatus::CaptureFailed
        | common_enums::AttemptStatus::VoidFailed
        | common_enums::AttemptStatus::Failure => true,
        common_enums::AttemptStatus::Started
        | common_enums::AttemptStatus::RouterDeclined
        | common_enums::AttemptStatus::AuthenticationPending
        | common_enums::AttemptStatus::AuthenticationSuccessful
        | common_enums::AttemptStatus::Authorized
        | common_enums::AttemptStatus::Charged
        | common_enums::AttemptStatus::Authorizing
        | common_enums::AttemptStatus::CodInitiated
        | common_enums::AttemptStatus::Voided
        | common_enums::AttemptStatus::VoidInitiated
        | common_enums::AttemptStatus::CaptureInitiated
        | common_enums::AttemptStatus::AutoRefunded
        | common_enums::AttemptStatus::PartialCharged
        | common_enums::AttemptStatus::PartialChargedAndChargeable
        | common_enums::AttemptStatus::Unresolved
        | common_enums::AttemptStatus::Pending
        | common_enums::AttemptStatus::PaymentMethodAwaited
        | common_enums::AttemptStatus::ConfirmationAwaited
        | common_enums::AttemptStatus::DeviceDataCollectionPending => false,
    }
}
