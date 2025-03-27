use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, Card, PayLaterData, PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::{refunds::Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData as _},
};

impl TryFrom<(&types::TokenizationRouterData, BankDebitData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, BankDebitData),
    ) -> Result<Self, Self::Error> {
        let (_item, bank_debit_data) = value;
        match bank_debit_data {
            BankDebitData::AchBankDebit { .. }
            | BankDebitData::SepaBankDebit { .. }
            | BankDebitData::BecsBankDebit { .. }
            | BankDebitData::BacsBankDebit { .. } => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Square"),
            ))?,
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, Card)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: (&types::TokenizationRouterData, Card)) -> Result<Self, Self::Error> {
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

impl TryFrom<(&types::TokenizationRouterData, PayLaterData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::TokenizationRouterData, PayLaterData),
    ) -> Result<Self, Self::Error> {
        let (_item, pay_later_data) = value;
        match pay_later_data {
            PayLaterData::AfterpayClearpayRedirect { .. }
            | PayLaterData::KlarnaRedirect { .. }
            | PayLaterData::KlarnaSdk { .. }
            | PayLaterData::AffirmRedirect { .. }
            | PayLaterData::PayBrightRedirect { .. }
            | PayLaterData::WalleyRedirect { .. }
            | PayLaterData::AlmaRedirect { .. }
            | PayLaterData::AtomeRedirect { .. } => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Square"),
            ))?,
        }
    }
}

impl TryFrom<(&types::TokenizationRouterData, WalletData)> for SquareTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: (&types::TokenizationRouterData, WalletData)) -> Result<Self, Self::Error> {
        let (_item, wallet_data) = value;
        match wallet_data {
            WalletData::AmazonPay(_)
            | WalletData::ApplePay(_)
            | WalletData::GooglePay(_)
            | WalletData::AliPayQr(_)
            | WalletData::AliPayRedirect(_)
            | WalletData::AliPayHkRedirect(_)
            | WalletData::AmazonPayRedirect(_)
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
            | WalletData::Paze(_)
            | WalletData::SamsungPay(_)
            | WalletData::TwintRedirect {}
            | WalletData::VippsRedirect {}
            | WalletData::TouchNGoRedirect(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_)
            | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
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
            PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::try_from((item, bank_debit_data))
            }
            PaymentMethodData::Card(card_data) => Self::try_from((item, card_data)),
            PaymentMethodData::Wallet(wallet_data) => Self::try_from((item, wallet_data)),
            PaymentMethodData::PayLater(pay_later_data) => Self::try_from((item, pay_later_data)),
            PaymentMethodData::GiftCard(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
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

impl<F, T> TryFrom<ResponseRouterData<F, SquareSessionResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SquareSessionResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_id.clone().expose()),
            response: Ok(PaymentsResponseData::SessionTokenResponse {
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

impl<F, T> TryFrom<ResponseRouterData<F, SquareTokenResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SquareTokenResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
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
            PaymentMethodData::Card(_) => {
                let pm_token = item.get_payment_method_token()?;
                Ok(Self {
                    idempotency_key: Secret::new(item.attempt_id.clone()),
                    source_id: match pm_token {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Square"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Square"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Square"))?
                        }
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
            PaymentMethodData::BankDebit(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
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

impl TryFrom<&ConnectorAuthType> for SquareAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1, .. } => Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
            }),
            ConnectorAuthType::HeaderKey { .. }
            | ConnectorAuthType::SignatureKey { .. }
            | ConnectorAuthType::MultiAuthKey { .. }
            | ConnectorAuthType::CurrencyAuthKey { .. }
            | ConnectorAuthType::TemporaryAuth { .. }
            | ConnectorAuthType::NoKey { .. }
            | ConnectorAuthType::CertificateAuth { .. } => {
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

impl<F, T> TryFrom<ResponseRouterData<F, SquarePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SquarePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        //Since this try_from is being used in Authorize, Sync, Capture & Void flow. Field amount_captured should only be updated in case of Charged status.
        let status = enums::AttemptStatus::from(item.response.payment.status);
        let mut amount_captured = None;
        if status == enums::AttemptStatus::Charged {
            amount_captured = Some(item.response.payment.amount_money.amount)
        };
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.payment.reference_id,
                incremental_authorization_allowed: None,
                charges: None,
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund.id,
                refund_status: enums::RefundStatus::from(item.response.refund.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for types::RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
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

impl From<SquareWebhookObject> for IncomingWebhookEvent {
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
