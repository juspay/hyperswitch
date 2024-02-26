use common_enums::Currency;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct PlaidRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PlaidRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}


#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PlaidPaymentsRequest {
    amount: PlaidAmount,
    recipient_id: String,
    reference: String,
    schedule: Option<PlaidSchedule>,
    options: Option<PlaidOptions>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PlaidAmount {
    currency: Currency,
    value: f64,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PlaidSchedule {
    interval: String,
    interval_execution_day: String,
    start_date: String,
    end_date: Option<String>,
    adjusted_start_date: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PlaidOptions {
    request_refund_details: bool,
    iban: Option<String>,
    bacs: Option<PlaidBacs>,
    scheme: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PlaidBacs {
    acount: Secret<String>,
    sort_code: Secret<String>,
}

impl TryFrom<&PlaidRouterData<&types::PaymentsAuthorizeRouterData>> for PlaidPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlaidRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::BankRedirect(ref bank) => match bank {
                api_models::payments::BankRedirectData::OpenBanking => {
                    let amount = item.amount as f64;
                    let currency = item.router_data.request.currency;
                    let reference =
                        item.router_data.connector_request_reference_id.clone();
                    let recipient_id = item.router_data.connector_customer.ok_or(errors::ConnectorError::MissingRequiredField { field_name: "connector_customer" })?;
                    Ok(Self {
                       amount: PlaidAmount { currency, value: amount },
                        reference,
                        recipient_id,
                        schedule: None,
                        options:None,
                    })
                }
                _ => Err(errors::ConnectorError::NotImplemented("Payment method type".to_string()).into()),
            }
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
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
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
    PaymentStausInputNeeded,
    PaymentStausInitiatied,
    PaymentStausInsuficientFunds,
    PaymentStausFailed,
    PaymentStausBlcoked,
    PaymentStausCancelled,
    PaymentStausExecuted,
    PaymentStausSettled,
    PaymentStausEstablished,
    PaymentStausRejected,
    #[default]
    PaymentStausAuthorising,
}

impl From<PlaidPaymentStatus> for enums::AttemptStatus {
    fn from(item: PlaidPaymentStatus) -> Self {
        match item {
            // Double check these with someone
            PlaidPaymentStatus::PaymentStausAuthorising => Self::Authorizing,
            PlaidPaymentStatus::PaymentStausBlcoked => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStausCancelled => Self::Voided,
            PlaidPaymentStatus::PaymentStausEstablished => Self::Authorized,
            PlaidPaymentStatus::PaymentStausExecuted => Self::Authorized,
            PlaidPaymentStatus::PaymentStausFailed => Self::Failure,
            PlaidPaymentStatus::PaymentStausInitiatied => Self::AuthenticationPending,
            PlaidPaymentStatus::PaymentStausInputNeeded => Self::AuthenticationPending,
            PlaidPaymentStatus::PaymentStausInsuficientFunds => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStausRejected => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStausSettled => Self::Charged
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaidSyncRequest {
    payment_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaidSyncResponse {
    payment_id: String,
    amount: PlaidAmount,
    status: PlaidPaymentStatus,
    recipient_id: String,
    reference: String,
    last_status_update: String,
    adjusted_reference: String,
    schedule: Option<PlaidSchedule>,
    iban: Option<String>,
    bacs: Option<PlaidBacs>,
    scheme: Option<String>,
    adjusted_scheme: Option<String>,
    request_id: String,
    // TODO: add refund related objects
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData> {

        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>) -> Result<Self, Self::Error> {
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
                        resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
// #[derive(Default, Debug, Serialize)]
// pub struct PlaidRefundRequest {
//     pub amount: i64,
// }

// impl<F> TryFrom<&PlaidRouterData<&types::RefundsRouterData<F>>> for PlaidRefundRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &PlaidRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
//         Ok(Self {
//             amount: item.amount.to_owned(),
//         })
//     }
// }

// // Type definition for Refund Response

// #[allow(dead_code)]
// #[derive(Debug, Serialize, Default, Deserialize, Clone)]
// pub enum RefundStatus {
//     Succeeded,
//     Failed,
//     #[default]
//     Processing,
// }

// impl From<RefundStatus> for enums::RefundStatus {
//     fn from(item: RefundStatus) -> Self {
//         match item {
//             RefundStatus::Succeeded => Self::Success,
//             RefundStatus::Failed => Self::Failure,
//             RefundStatus::Processing => Self::Pending,
//             //TODO: Review mapping
//         }
//     }
// }

// //TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Clone, Serialize, Deserialize)]
// pub struct RefundResponse {
//     id: String,
//     status: RefundStatus,
// }

// impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
//     for types::RefundsRouterData<api::Execute>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

// impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
//     for types::RefundsRouterData<api::RSync>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

//TODO: Fill the struct with respective fields
##[derive(Debug, Deserialize, PartialEq, Eq)]
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
