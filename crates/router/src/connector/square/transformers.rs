use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, api, domain, storage::enums},
    unimplemented_payment_method,
};

impl TryFrom<(&types::TokenizationRouterData, domain::BankDebitData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, domain::BankDebitData),
    ) -> Result<Self, Self::Error> {
        let (_item, bank_debit_data) = value;
        match bank_debit_data {
            domain::BankDebitData::AchBankDebit { .. }
            | domain::BankDebitData::SepaBankDebit { .. }
            | domain::BankDebitData::BecsBankDebit { .. }
            | domain::BankDebitData::BacsBankDebit { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Square"),
                ))?
            }
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, domain::Card)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, domain::Card),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let auth = SquareAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let exp_year = Secret::new(
            card_data
                .get_expiry_year_4_digit()
                .peek()
                .parse::<u16>()
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
        );
        let exp_month = Secret::new(
            card_data
                .card_exp_month
                .peek()
                .parse::<u16>()
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

impl TryFrom<(&types::TokenizationRouterData, domain::PayLaterData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, domain::PayLaterData),
    ) -> Result<Self, Self::Error> {
        let (_item, pay_later_data) = value;
        match pay_later_data {
            domain::PayLaterData::AfterpayClearpayRedirect { .. }
            | domain::PayLaterData::KlarnaRedirect { .. }
            | domain::PayLaterData::KlarnaSdk { .. }
            | domain::PayLaterData::AffirmRedirect { .. }
            | domain::PayLaterData::PayBrightRedirect { .. }
            | domain::PayLaterData::WalleyRedirect { .. }
            | domain::PayLaterData::AlmaRedirect { .. }
            | domain::PayLaterData::AtomeRedirect { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Square"),
                ))?
            }
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, domain::WalletData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, domain::WalletData),
    ) -> Result<Self, Self::Error> {
        let (_item, wallet_data) = value;
        match wallet_data {
            domain::WalletData::ApplePay(_)
            | domain::WalletData::GooglePay(_)
            | domain::WalletData::AliPayQr(_)
            | domain::WalletData::AliPayRedirect(_)
            | domain::WalletData::AliPayHkRedirect(_)
            | domain::WalletData::MomoRedirect(_)
            | domain::WalletData::KakaoPayRedirect(_)
            | domain::WalletData::GoPayRedirect(_)
            | domain::WalletData::GcashRedirect(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::DanaRedirect {}
            | domain::WalletData::GooglePayRedirect(_)
            | domain::WalletData::GooglePayThirdPartySdk(_)
            | domain::WalletData::MbWayRedirect(_)
            | domain::WalletData::MobilePayRedirect(_)
            | domain::WalletData::PaypalRedirect(_)
            | domain::WalletData::PaypalSdk(_)
            | domain::WalletData::SamsungPay(_)
            | domain::WalletData::TwintRedirect {}
            | domain::WalletData::VippsRedirect {}
            | domain::WalletData::TouchNGoRedirect(_)
            | domain::WalletData::WeChatPayRedirect(_)
            | domain::WalletData::WeChatPayQr(_)
            | domain::WalletData::CashappQr(_)
            | domain::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Square"),
            ))?,
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
            domain::PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::try_from((item, bank_debit_data))
            }
            domain::PaymentMethodData::Card(card_data) => Self::try_from((item, card_data)),
            domain::PaymentMethodData::Wallet(wallet_data) => Self::try_from((item, wallet_data)),
            domain::PaymentMethodData::PayLater(pay_later_data) => {
                Self::try_from((item, pay_later_data))
            }
            domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Square"),
                ))?
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SquareSessionResponse {
    session_id: Secret<String>,
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
            status: enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_id.clone().expose()),
            response: Ok(types::PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.session_id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
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
            domain::PaymentMethodData::Card(_) => {
                let pm_token = item.get_payment_method_token()?;
                Ok(Self {
                    idempotency_key: Secret::new(item.attempt_id.clone()),
                    source_id: match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Square"),
                        )?,
                    },
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
            domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Square"),
                ))?
            }
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
            | types::ConnectorAuthType::TemporaryAuth { .. }
            | types::ConnectorAuthType::NoKey { .. }
            | types::ConnectorAuthType::CertificateAuth { .. } => {
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
#[derive(Debug, Deserialize, Serialize)]
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
                incremental_authorization_allowed: None,
                charge_id: None,
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
#[derive(Debug, Deserialize, Serialize)]
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
