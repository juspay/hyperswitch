use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::{PeekInterface, Secret},
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData {}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaymentOptions {
    submit_for_settlement: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BraintreePaymentsRequest {
    transaction: TransactionBody,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionBody {
    amount: String,
    device_data: DeviceData,
    options: PaymentOptions,
    credit_card: Card,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: Option<Secret<String>>,
    expiration_month: Option<Secret<String>>,
    expiration_year: Option<Secret<String>>,
    cvv: Option<String>,
}

impl TryFrom<&types::PaymentsRouterData> for BraintreePaymentsRequest {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(item: &types::PaymentsRouterData) -> Result<Self, Self::Error> {
        let ccard = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Some(ccard),
            api::PaymentMethod::BankTransfer => None,
            api::PaymentMethod::PayLater(_) => None,
            api::PaymentMethod::Wallet => None,
            api::PaymentMethod::Paypal => None,
        };

        let braintree_payment_request = TransactionBody {
            amount: item.amount.to_string(),
            device_data: DeviceData {},
            options: PaymentOptions {
                submit_for_settlement: true,
            },
            credit_card: Card {
                number: ccard.map(|x| x.card_number.peek().clone().into()),
                expiration_month: ccard.map(|x| x.card_exp_month.peek().clone().into()),
                expiration_year: ccard.map(|x| x.card_exp_year.peek().clone().into()),
                cvv: ccard.map(|x| x.card_cvc.peek().clone().into()),
            },
            kind: "sale".to_string(),
        };
        Ok(BraintreePaymentsRequest {
            transaction: braintree_payment_request,
        })
    }
}

// Auth Struct
pub struct BraintreeAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
}

impl TryFrom<&types::ConnectorAuthType> for BraintreeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = item {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BraintreePaymentStatus {
    Succeeded,
    Failed,
    Authorized,
    AuthorizedExpired,
    ProcessorDeclined,
    GatewayRejected,
    Voided,
    SubmittedForSettlement,
    Settling,
    Settled,
    SettlementPending,
    SettlementDeclined,
    SettlementConfirmed,
}

// Default should be Processing
impl Default for BraintreePaymentStatus {
    fn default() -> Self {
        BraintreePaymentStatus::Settling
    }
}

impl From<BraintreePaymentStatus> for enums::AttemptStatus {
    fn from(item: BraintreePaymentStatus) -> Self {
        match item {
            BraintreePaymentStatus::Succeeded => enums::AttemptStatus::Charged,
            BraintreePaymentStatus::Failed => enums::AttemptStatus::Failure,
            BraintreePaymentStatus::AuthorizedExpired => enums::AttemptStatus::AuthorizationFailed,
            BraintreePaymentStatus::GatewayRejected => enums::AttemptStatus::Failure,
            BraintreePaymentStatus::ProcessorDeclined => enums::AttemptStatus::Failure,
            BraintreePaymentStatus::SettlementDeclined => enums::AttemptStatus::Failure,
            BraintreePaymentStatus::Authorized => enums::AttemptStatus::Authorized,
            BraintreePaymentStatus::Voided => enums::AttemptStatus::Voided,
            _ => enums::AttemptStatus::Pending,
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BraintreePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            response: Some(types::PaymentsResponseData {
                connector_transaction_id: item.response.transaction.id,
                //TODO: Add redirection details here
                redirection_data: None,
                redirect: false,
            }),
            error_response: None,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BraintreePaymentsResponse {
    transaction: TransactionResponse,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    id: String,
    currency_iso_code: String,
    amount: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub api_error_response: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub message: String,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(RefundRequest {})
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

// Default should be Processing
impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::Processing
    }
}

impl From<self::RefundStatus> for enums::RefundStatus {
    fn from(item: self::RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => enums::RefundStatus::Success,
            self::RefundStatus::Failed => enums::RefundStatus::Failure,
            self::RefundStatus::Processing => enums::RefundStatus::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            response: Some(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            error_response: None,
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            response: Some(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            error_response: None,
            ..item.data
        })
    }
}
