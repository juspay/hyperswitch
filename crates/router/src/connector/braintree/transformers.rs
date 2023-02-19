use base64::Engine;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
    utils::OptionExt,
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaymentOptions {
    submit_for_settlement: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
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

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionBody {
    amount: String,
    device_data: DeviceData,
    options: PaymentOptions,
    #[serde(flatten)]
    payment_method_data_type: PaymentMethodType,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum PaymentMethodType {
    CreditCard(Card),
    PaymentMethodNonce(Nonce),
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Nonce {
    payment_method_nonce: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    credit_card: CardDetails,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    number: String,
    expiration_month: String,
    expiration_year: String,
    cvv: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BraintreePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let submit_for_settlement = matches!(
            item.request.capture_method,
            Some(enums::CaptureMethod::Automatic) | None
        );

        let amount = item.request.amount.to_string();
        let device_data = DeviceData {};
        let options = PaymentOptions {
            submit_for_settlement,
        };
        let kind = "sale".to_string();

        let payment_method_data_type = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Ok(PaymentMethodType::CreditCard(Card {
                credit_card: CardDetails {
                    number: ccard.card_number.peek().clone(),
                    expiration_month: ccard.card_exp_month.peek().clone(),
                    expiration_year: ccard.card_exp_year.peek().clone(),
                    cvv: ccard.card_cvc.peek().clone(),
                },
            })),
            api::PaymentMethod::Wallet(ref wallet_data) => {
                Ok(PaymentMethodType::PaymentMethodNonce(Nonce {
                    payment_method_nonce: wallet_data
                        .token
                        .to_owned()
                        .get_required_value("token")
                        .change_context(errors::ConnectorError::RequestEncodingFailed)
                        .attach_printable("No token passed")?,
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented(format!(
                "Current Payment Method - {:?}",
                item.request.payment_method_data
            ))),
        }?;
        let braintree_transaction_body = TransactionBody {
            amount,
            device_data,
            options,
            payment_method_data_type,
            kind,
        };
        Ok(Self {
            transaction: braintree_transaction_body,
        })
    }
}

pub struct BraintreeAuthType {
    pub(super) auth_header: String,
    pub(super) merchant_id: String,
}

impl TryFrom<&types::ConnectorAuthType> for BraintreeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key: public_key,
            key1: merchant_id,
            api_secret: private_key,
        } = item
        {
            let auth_key = format!("{public_key}:{private_key}");
            let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
            Ok(Self {
                auth_header,
                merchant_id: merchant_id.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Eq, PartialEq)]
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
    #[default]
    Settling,
    Settled,
    SettlementPending,
    SettlementDeclined,
    SettlementConfirmed,
}

impl From<BraintreePaymentStatus> for enums::AttemptStatus {
    fn from(item: BraintreePaymentStatus) -> Self {
        match item {
            BraintreePaymentStatus::Succeeded
            | BraintreePaymentStatus::SubmittedForSettlement
            | BraintreePaymentStatus::Settling => Self::Charged,
            BraintreePaymentStatus::AuthorizedExpired => Self::AuthorizationFailed,
            BraintreePaymentStatus::Failed
            | BraintreePaymentStatus::GatewayRejected
            | BraintreePaymentStatus::ProcessorDeclined
            | BraintreePaymentStatus::SettlementDeclined => Self::Failure,
            BraintreePaymentStatus::Authorized => Self::Authorized,
            BraintreePaymentStatus::Voided => Self::Voided,
            _ => Self::Pending,
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
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.transaction.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction.id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
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
        Ok(Self {
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
        Ok(Self {
            transaction: Amount { amount: None },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize, Clone)]
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
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
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
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}
