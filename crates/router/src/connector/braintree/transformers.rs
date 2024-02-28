use api_models::payments;
use base64::Engine;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self},
    consts,
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaymentOptions {
    submit_for_settlement: bool,
}

#[derive(Debug, Deserialize)]
pub struct BraintreeMeta {
    merchant_account_id: Option<Secret<String>>,
    merchant_config_currency: Option<types::storage::enums::Currency>,
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
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(_item.connector_meta_data.clone())?;

        utils::validate_currency(_item.request.currency, metadata.merchant_config_currency)?;
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
    merchant_account_id: Option<Secret<String>>,
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
    payment_method_nonce: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    credit_card: CardDetails,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cvv: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BraintreePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(item.connector_meta_data.clone())?;

        utils::validate_currency(item.request.currency, metadata.merchant_config_currency)?;
        let submit_for_settlement = matches!(
            item.request.capture_method,
            Some(enums::CaptureMethod::Automatic) | None
        );
        let merchant_account_id = metadata.merchant_account_id;
        let amount = utils::to_currency_base_unit(item.request.amount, item.request.currency)?;
        let device_data = DeviceData {};
        let options = PaymentOptions {
            submit_for_settlement,
        };
        let kind = "sale".to_string();

        let payment_method_data_type = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => Ok(PaymentMethodType::CreditCard(Card {
                credit_card: CardDetails {
                    number: ccard.card_number,
                    expiration_month: ccard.card_exp_month,
                    expiration_year: ccard.card_exp_year,
                    cvv: ccard.card_cvc,
                },
            })),
            api::PaymentMethodData::Wallet(ref wallet_data) => {
                Ok(PaymentMethodType::PaymentMethodNonce(Nonce {
                    payment_method_nonce: match wallet_data {
                        api_models::payments::WalletData::PaypalSdk(wallet_data) => {
                            Ok(wallet_data.token.to_owned())
                        }
                        api_models::payments::WalletData::ApplePay(_)
                        | api_models::payments::WalletData::GooglePay(_)
                        | api_models::payments::WalletData::SamsungPay(_)
                        | api_models::payments::WalletData::AliPayQr(_)
                        | api_models::payments::WalletData::AliPayRedirect(_)
                        | api_models::payments::WalletData::AliPayHkRedirect(_)
                        | api_models::payments::WalletData::MomoRedirect(_)
                        | api_models::payments::WalletData::KakaoPayRedirect(_)
                        | api_models::payments::WalletData::GoPayRedirect(_)
                        | api_models::payments::WalletData::GcashRedirect(_)
                        | api_models::payments::WalletData::ApplePayRedirect(_)
                        | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                        | api_models::payments::WalletData::DanaRedirect {}
                        | api_models::payments::WalletData::GooglePayRedirect(_)
                        | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                        | api_models::payments::WalletData::MbWayRedirect(_)
                        | api_models::payments::WalletData::MobilePayRedirect(_)
                        | api_models::payments::WalletData::PaypalRedirect(_)
                        | api_models::payments::WalletData::TwintRedirect {}
                        | api_models::payments::WalletData::VippsRedirect {}
                        | api_models::payments::WalletData::TouchNGoRedirect(_)
                        | api_models::payments::WalletData::WeChatPayRedirect(_)
                        | api_models::payments::WalletData::WeChatPayQr(_)
                        | api_models::payments::WalletData::CashappQr(_)
                        | api_models::payments::WalletData::SwishQr(_) => {
                            Err(errors::ConnectorError::NotImplemented(
                                utils::get_unimplemented_payment_method_error_message("braintree"),
                            ))
                        }
                    }?,
                }))
            }
            api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardToken(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("braintree"),
            )),
        }?;
        let braintree_transaction_body = TransactionBody {
            amount,
            merchant_account_id,
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
    pub(super) merchant_id: Secret<String>,
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
            let auth_key = format!("{}:{}", public_key.peek(), private_key.peek());
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

#[derive(Debug, Default, Clone, Deserialize, Eq, PartialEq, Serialize)]
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
            | BraintreePaymentStatus::Settling
            | BraintreePaymentStatus::Settled => Self::Charged,
            BraintreePaymentStatus::AuthorizedExpired => Self::AuthorizationFailed,
            BraintreePaymentStatus::Failed
            | BraintreePaymentStatus::GatewayRejected
            | BraintreePaymentStatus::ProcessorDeclined
            | BraintreePaymentStatus::SettlementDeclined => Self::Failure,
            BraintreePaymentStatus::Authorized => Self::Authorized,
            BraintreePaymentStatus::Voided => Self::Voided,
            BraintreePaymentStatus::SubmittedForSettlement
            | BraintreePaymentStatus::SettlementPending
            | BraintreePaymentStatus::SettlementConfirmed => Self::Pending,
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
        let id = item.response.transaction.id.clone();
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.transaction.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(id),
                incremental_authorization_allowed: None,
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
                session_token: types::api::SessionToken::Paypal(Box::new(
                    payments::PaypalSessionTokenResponse {
                        session_token: item.response.client_token.value.expose(),
                    },
                )),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreePaymentsResponse {
    transaction: TransactionResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientToken {
    pub value: Secret<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeSessionTokenResponse {
    pub client_token: ClientToken,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    id: String,
    currency_iso_code: String,
    amount: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeApiErrorResponse {
    pub api_error_response: ApiErrorResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorsObject {
    pub errors: Vec<ErrorObject>,
    pub transaction: Option<TransactionError>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionError {
    pub errors: Vec<ErrorObject>,
    pub credit_card: Option<CreditCardError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreditCardError {
    pub errors: Vec<ErrorObject>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorObject {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeErrorResponse {
    pub errors: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]

pub enum ErrorResponse {
    BraintreeApiErrorResponse(Box<BraintreeApiErrorResponse>),
    BraintreeErrorResponse(Box<BraintreeErrorResponse>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiErrorResponse {
    pub message: String,
    pub errors: ErrorsObject,
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
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(item.connector_meta_data.clone())?;

        utils::validate_currency(item.request.currency, metadata.merchant_config_currency)?;

        let refund_amount =
            utils::to_currency_base_unit(item.request.refund_amount, item.request.currency)?;
        Ok(Self {
            transaction: Amount {
                amount: Some(refund_amount),
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize, Clone, Serialize)]
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

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
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
