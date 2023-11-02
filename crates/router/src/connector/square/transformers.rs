use api_models::payments::{BankDebitData, PayLaterData, WalletData};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{CardData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{
        self, api,
        storage::{self, enums},
    },
};

impl TryFrom<(&types::TokenizationRouterData, BankDebitData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, BankDebitData),
    ) -> Result<Self, Self::Error> {
        let (item, bank_debit_data) = value;
        match bank_debit_data {
            BankDebitData::AchBankDebit { .. } => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),

            BankDebitData::SepaBankDebit { .. }
            | BankDebitData::BecsBankDebit { .. }
            | BankDebitData::BacsBankDebit { .. } => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_data),
                connector: "Square",
            })?,
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, api_models::payments::Card)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, api_models::payments::Card),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let auth = SquareAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let exp_year = Secret::new(
            card_data
                .get_expiry_year_4_digit()
                .peek()
                .parse::<u16>()
                .into_report()
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
        );
        let exp_month = Secret::new(
            card_data
                .card_exp_month
                .peek()
                .parse::<u16>()
                .into_report()
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
        );
        //The below error will never happen because if session-id is not generated it would give error in execute_pretasks itself.
        let session_id = Secret::new(
            item.session_token
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
        );
        Ok(Self::Card(SquareTokenizeData {
            client_id: auth.key1,
            session_id,
            card_data: SquareCardData {
                exp_year,
                exp_month,
                number: card_data.card_number,
                cvv: card_data.card_cvc,
            },
        }))
    }
}

impl TryFrom<(&types::TokenizationRouterData, PayLaterData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, PayLaterData),
    ) -> Result<Self, Self::Error> {
        let (item, pay_later_data) = value;
        match pay_later_data {
            PayLaterData::AfterpayClearpayRedirect { .. } => Err(
                errors::ConnectorError::NotImplemented("Payment Method".to_string()),
            )
            .into_report(),

            PayLaterData::KlarnaRedirect { .. }
            | PayLaterData::KlarnaSdk { .. }
            | PayLaterData::AffirmRedirect { .. }
            | PayLaterData::PayBrightRedirect { .. }
            | PayLaterData::WalleyRedirect { .. }
            | PayLaterData::AlmaRedirect { .. }
            | PayLaterData::AtomeRedirect { .. } => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_data),
                connector: "Square",
            })?,
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, WalletData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: (&types::TokenizationRouterData, WalletData)) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        match wallet_data {
            WalletData::ApplePay(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
            WalletData::GooglePay(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),

            WalletData::AliPayQr(_)
            | WalletData::AliPayRedirect(_)
            | WalletData::AliPayHkRedirect(_)
            | WalletData::MomoRedirect(_)
            | WalletData::KakaoPayRedirect(_)
            | WalletData::GoPayRedirect(_)
            | WalletData::GcashRedirect(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::DanaRedirect {}
            | WalletData::GooglePayRedirect(_)
            | WalletData::GooglePayThirdPartySdk(_)
            | WalletData::MbWayRedirect(_)
            | WalletData::MobilePayRedirect(_)
            | WalletData::PaypalRedirect(_)
            | WalletData::PaypalSdk(_)
            | WalletData::SamsungPay(_)
            | WalletData::TwintRedirect {}
            | WalletData::VippsRedirect {}
            | WalletData::TouchNGoRedirect(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_) => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_data),
                connector: "Square",
            })?,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SquareCardData {
    cvv: Secret<String>,
    exp_month: Secret<u16>,
    exp_year: Secret<u16>,
    number: cards::CardNumber,
}
#[derive(Debug, Serialize)]
pub struct SquareTokenizeData {
    client_id: Secret<String>,
    session_id: Secret<String>,
    card_data: SquareCardData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SquareTokenRequest {
    Card(SquareTokenizeData),
}

impl TryFrom<&types::TokenizationRouterData> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::try_from((item, bank_debit_data))
            }
            api::PaymentMethodData::Card(card_data) => Self::try_from((item, card_data)),
            api::PaymentMethodData::Wallet(wallet_data) => Self::try_from((item, wallet_data)),
            api::PaymentMethodData::PayLater(pay_later_data) => {
                Self::try_from((item, pay_later_data))
            }
            api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
            api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_) => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_data),
                connector: "Square",
            })?,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SquareSessionResponse {
    session_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SquareSessionResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SquareSessionResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: storage::enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_id.clone()),
            response: Ok(types::PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.session_id,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SquareTokenResponse {
    card_nonce: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SquareTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SquareTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.card_nonce.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SquarePaymentsAmountData {
    amount: i64,
    currency: enums::Currency,
}
#[derive(Debug, Serialize)]
pub struct SquarePaymentsRequestExternalDetails {
    source: String,
    #[serde(rename = "type")]
    source_type: String,
}

#[derive(Debug, Serialize)]
pub struct SquarePaymentsRequest {
    amount_money: SquarePaymentsAmountData,
    idempotency_key: Secret<String>,
    source_id: Secret<String>,
    autocomplete: bool,
    external_details: SquarePaymentsRequestExternalDetails,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for SquarePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let autocomplete = item.request.is_auto_capture()?;
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                let pm_token = item.get_payment_method_token()?;
                Ok(Self {
                    idempotency_key: Secret::new(item.attempt_id.clone()),
                    source_id: Secret::new(match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }),
                    amount_money: SquarePaymentsAmountData {
                        amount: item.request.amount,
                        currency: item.request.currency,
                    },
                    autocomplete,
                    external_details: SquarePaymentsRequestExternalDetails {
                        source: "Hyperswitch".to_string(),
                        source_type: "Card".to_string(),
                    },
                })
            }
            api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::Wallet(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
            api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_) => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_data),
                connector: "Square",
            })?,
        }
    }
}

// Auth Struct
pub struct SquareAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) key1: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for SquareAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1, .. } => Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
            }),

            types::ConnectorAuthType::HeaderKey { .. }
            | types::ConnectorAuthType::SignatureKey { .. }
            | types::ConnectorAuthType::MultiAuthKey { .. }
            | types::ConnectorAuthType::CurrencyAuthKey { .. }
            | types::ConnectorAuthType::NoKey { .. } => {
                Err(errors::ConnectorError::FailedToObtainAuthType.into())
            }
        }
    }
}
// PaymentsResponse
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SquarePaymentStatus {
    Completed,
    Failed,
    Approved,
    Canceled,
    Pending,
}

impl From<SquarePaymentStatus> for enums::AttemptStatus {
    fn from(item: SquarePaymentStatus) -> Self {
        match item {
            SquarePaymentStatus::Completed => Self::Charged,
            SquarePaymentStatus::Approved => Self::Authorized,
            SquarePaymentStatus::Failed => Self::Failure,
            SquarePaymentStatus::Canceled => Self::Voided,
            SquarePaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SquarePaymentsResponseDetails {
    status: SquarePaymentStatus,
    id: String,
    amount_money: SquarePaymentsAmountData,
    reference_id: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct SquarePaymentsResponse {
    payment: SquarePaymentsResponseDetails,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SquarePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SquarePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        //Since this try_from is being used in Authorize, Sync, Capture & Void flow. Field amount_captured should only be updated in case of Charged status.
        let status = enums::AttemptStatus::from(item.response.payment.status);
        let mut amount_captured = None;
        if status == enums::AttemptStatus::Charged {
            amount_captured = Some(item.response.payment.amount_money.amount)
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.payment.reference_id,
            }),
            amount_captured,
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct SquareRefundRequest {
    amount_money: SquarePaymentsAmountData,
    idempotency_key: Secret<String>,
    payment_id: Secret<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for SquareRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_money: SquarePaymentsAmountData {
                amount: item.request.refund_amount,
                currency: item.request.currency,
            },
            idempotency_key: Secret::new(item.request.refund_id.clone()),
            payment_id: Secret::new(item.request.connector_transaction_id.clone()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Completed,
    Failed,
    Pending,
    Rejected,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Completed => Self::Success,
            RefundStatus::Failed | RefundStatus::Rejected => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SquareRefundResponseDetails {
    status: RefundStatus,
    id: String,
}
#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    refund: SquareRefundResponseDetails,
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
                connector_refund_id: item.response.refund.id,
                refund_status: enums::RefundStatus::from(item.response.refund.status),
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
                connector_refund_id: item.response.refund.id,
                refund_status: enums::RefundStatus::from(item.response.refund.status),
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SquareErrorDetails {
    pub category: Option<String>,
    pub code: Option<String>,
    pub detail: Option<String>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SquareErrorResponse {
    pub errors: Vec<SquareErrorDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SquareWebhookObject {
    Payment(SquarePaymentsResponseDetails),
    Refund(SquareRefundResponseDetails),
}

#[derive(Debug, Deserialize)]
pub struct SquareWebhookData {
    pub id: String,
    pub object: SquareWebhookObject,
}

#[derive(Debug, Deserialize)]
pub struct SquareWebhookBody {
    #[serde(rename = "type")]
    pub webhook_type: String,
    pub data: SquareWebhookData,
}

impl From<SquareWebhookObject> for api::IncomingWebhookEvent {
    fn from(item: SquareWebhookObject) -> Self {
        match item {
            SquareWebhookObject::Payment(payment_data) => match payment_data.status {
                SquarePaymentStatus::Completed => Self::PaymentIntentSuccess,
                SquarePaymentStatus::Failed => Self::PaymentIntentFailure,
                SquarePaymentStatus::Pending => Self::PaymentIntentProcessing,
                SquarePaymentStatus::Approved | SquarePaymentStatus::Canceled => {
                    Self::EventNotSupported
                }
            },
            SquareWebhookObject::Refund(refund_data) => match refund_data.status {
                RefundStatus::Completed => Self::RefundSuccess,
                RefundStatus::Failed | RefundStatus::Rejected => Self::RefundFailure,
                RefundStatus::Pending => Self::EventNotSupported,
            },
        }
    }
}
