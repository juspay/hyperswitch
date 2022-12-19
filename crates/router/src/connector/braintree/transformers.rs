use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaymentOptions {
    submit_for_settlement: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BraintreePaymentsRequest {
    transaction: TransactionBody,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BraintreeApiVersion {
    version: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BraintreeSessionRequest {
    client_token: BraintreeApiVersion,
}

impl TryFrom<&types::PaymentsSessionRouterData> for BraintreeSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            client_token: BraintreeApiVersion {
                version: "2".to_string(),
            },
        })
    }
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
    number: String,
    expiration_month: String,
    expiration_year: String,
    cvv: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BraintreePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let submit_for_settlement = matches!(
                    item.request.capture_method,
                    Some(enums::CaptureMethod::Automatic) | None
                );
                let braintree_payment_request = TransactionBody {
                    amount: item.request.amount.to_string(),
                    device_data: DeviceData {},
                    options: PaymentOptions {
                        submit_for_settlement,
                    },
                    credit_card: Card {
                        number: ccard.card_number.peek().clone(),
                        expiration_month: ccard.card_exp_month.peek().clone(),
                        expiration_year: ccard.card_exp_year.peek().clone(),
                        cvv: ccard.card_cvc.peek().clone(),
                    },
                    kind: "sale".to_string(),
                };
                Ok(BraintreePaymentsRequest {
                    transaction: braintree_payment_request,
                })
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
        }
    }
}

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

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
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

impl Default for BraintreePaymentStatus {
    fn default() -> Self {
        BraintreePaymentStatus::Settling
    }
}

impl From<BraintreePaymentStatus> for enums::AttemptStatus {
    fn from(item: BraintreePaymentStatus) -> Self {
        match item {
            BraintreePaymentStatus::Succeeded => enums::AttemptStatus::Charged,
            BraintreePaymentStatus::AuthorizedExpired => enums::AttemptStatus::AuthorizationFailed,
            BraintreePaymentStatus::Failed
            | BraintreePaymentStatus::GatewayRejected
            | BraintreePaymentStatus::ProcessorDeclined
            | BraintreePaymentStatus::SettlementDeclined => enums::AttemptStatus::Failure,
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
            status: enums::AttemptStatus::from(item.response.transaction.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction.id,
                ),
                redirection_data: None,
                redirect: false,
                // TODO: Implement mandate fetch for other connectors
                mandate_reference: None,
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, BraintreeSessionTokenResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreeSessionTokenResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: types::api::SessionToken::Paypal {
                    session_token: item.response.client_token.value,
                },
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreePaymentsResponse {
    transaction: TransactionResponse,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientToken {
    pub value: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeSessionTokenResponse {
    pub client_token: ClientToken,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    id: String,
    currency_iso_code: String,
    amount: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub api_error_response: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub message: String,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeRefundRequest {
    transaction: Amount,
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct Amount {
    amount: Option<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BraintreeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(BraintreeRefundRequest {
            transaction: Amount { amount: None },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

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
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
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
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
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
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}
