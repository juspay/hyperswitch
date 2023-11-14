use common_utils::pii::Email;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, missing_field_err, CardData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct StaxRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for StaxRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_f64(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequestMetaData {
    tax: i64,
}

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequest {
    payment_method_id: Secret<String>,
    total: f64,
    is_refundable: bool,
    pre_auth: bool,
    meta: StaxPaymentsRequestMetaData,
    idempotency_id: Option<String>,
}

impl TryFrom<&StaxRouterData<&types::PaymentsAuthorizeRouterData>> for StaxPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StaxRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.request.currency != enums::Currency::USD {
            Err(errors::ConnectorError::NotSupported {
                message: item.router_data.request.currency.to_string(),
                connector: "Stax",
            })?
        }
        let total = item.amount;

        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                let pm_token = item.router_data.get_payment_method_token()?;
                let pre_auth = !item.router_data.request.is_auto_capture()?;
                Ok(Self {
                    meta: StaxPaymentsRequestMetaData { tax: 0 },
                    total,
                    is_refundable: true,
                    pre_auth,
                    payment_method_id: Secret::new(match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }),
                    idempotency_id: Some(item.router_data.connector_request_reference_id.clone()),
                })
            }
            api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::AchBankDebit { .. },
            ) => {
                let pm_token = item.router_data.get_payment_method_token()?;
                let pre_auth = !item.router_data.request.is_auto_capture()?;
                Ok(Self {
                    meta: StaxPaymentsRequestMetaData { tax: 0 },
                    total,
                    is_refundable: true,
                    pre_auth,
                    payment_method_id: Secret::new(match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }),
                    idempotency_id: Some(item.router_data.connector_request_reference_id.clone()),
                })
            }
            api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Upi(_) => Err(errors::ConnectorError::NotSupported {
                message: "SELECTED_PAYMENT_METHOD".to_string(),
                connector: "Stax",
            })?,
        }
    }
}

// Auth Struct
pub struct StaxAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StaxAuthType {
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

#[derive(Debug, Serialize)]
pub struct StaxCustomerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    firstname: Option<String>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for StaxCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        if item.request.email.is_none() && item.request.name.is_none() {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "email or name",
            })
            .into_report()
        } else {
            Ok(Self {
                email: item.request.email.to_owned(),
                firstname: item.request.name.to_owned(),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxCustomerResponse {
    id: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct StaxTokenizeData {
    person_name: Secret<String>,
    card_number: cards::CardNumber,
    card_exp: Secret<String>,
    card_cvv: Secret<String>,
    customer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct StaxBankTokenizeData {
    person_name: Secret<String>,
    bank_account: Secret<String>,
    bank_routing: Secret<String>,
    bank_name: api_models::enums::BankNames,
    bank_type: api_models::enums::BankType,
    bank_holder_type: api_models::enums::BankHolderType,
    customer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum StaxTokenRequest {
    Card(StaxTokenizeData),
    Bank(StaxBankTokenizeData),
}

impl TryFrom<&types::TokenizationRouterData> for StaxTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let customer_id = item.get_connector_customer_id()?;
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card_data) => {
                let stax_card_data = StaxTokenizeData {
                    card_exp: card_data
                        .get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                    person_name: card_data.card_holder_name,
                    card_number: card_data.card_number,
                    card_cvv: card_data.card_cvc,
                    customer_id: Secret::new(customer_id),
                };
                Ok(Self::Card(stax_card_data))
            }
            api_models::payments::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::AchBankDebit {
                    billing_details,
                    account_number,
                    routing_number,
                    bank_name,
                    bank_type,
                    bank_holder_type,
                    ..
                },
            ) => {
                let stax_bank_data = StaxBankTokenizeData {
                    person_name: billing_details.name,
                    bank_account: account_number,
                    bank_routing: routing_number,
                    bank_name: bank_name.ok_or_else(missing_field_err("bank_name"))?,
                    bank_type: bank_type.ok_or_else(missing_field_err("bank_type"))?,
                    bank_holder_type: bank_holder_type
                        .ok_or_else(missing_field_err("bank_holder_type"))?,
                    customer_id: Secret::new(customer_id),
                };
                Ok(Self::Bank(stax_bank_data))
            }
            api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Upi(_) => Err(errors::ConnectorError::NotSupported {
                message: "SELECTED_PAYMENT_METHOD".to_string(),
                connector: "Stax",
            })?,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxTokenResponse {
    id: Secret<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxPaymentResponseTypes {
    Charge,
    PreAuth,
}

#[derive(Debug, Deserialize)]
pub struct StaxChildCapture {
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct StaxPaymentsResponse {
    success: bool,
    id: String,
    is_captured: i8,
    is_voided: bool,
    child_captures: Vec<StaxChildCapture>,
    #[serde(rename = "type")]
    payment_response_type: StaxPaymentResponseTypes,
    idempotency_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaxMetaData {
    pub capture_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let mut connector_metadata = None;
        let mut status = match item.response.success {
            true => match item.response.payment_response_type {
                StaxPaymentResponseTypes::Charge => enums::AttemptStatus::Charged,
                StaxPaymentResponseTypes::PreAuth => match item.response.is_captured {
                    0 => enums::AttemptStatus::Authorized,
                    _ => {
                        connector_metadata =
                            item.response.child_captures.first().map(|child_captures| {
                                serde_json::json!(StaxMetaData {
                                    capture_id: child_captures.id.clone()
                                })
                            });
                        enums::AttemptStatus::Charged
                    }
                },
            },
            false => enums::AttemptStatus::Failure,
        };
        if item.response.is_voided {
            status = enums::AttemptStatus::Voided;
        }

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response.idempotency_id.unwrap_or(item.response.id),
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaxCaptureRequest {
    total: Option<f64>,
}

impl TryFrom<&StaxRouterData<&types::PaymentsCaptureRouterData>> for StaxCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StaxRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let total = item.amount;
        Ok(Self { total: Some(total) })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct StaxRefundRequest {
    pub total: f64,
}

impl<F> TryFrom<&StaxRouterData<&types::RefundsRouterData<F>>> for StaxRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &StaxRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self { total: item.amount })
    }
}

#[derive(Debug, Deserialize)]
pub struct ChildTransactionsInResponse {
    id: String,
    success: bool,
    created_at: String,
    total: f64,
}
#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    id: String,
    success: bool,
    child_transactions: Vec<ChildTransactionsInResponse>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_amount = utils::to_currency_base_unit_asf64(
            item.data.request.refund_amount,
            item.data.request.currency,
        )
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let filtered_txn: Vec<&ChildTransactionsInResponse> = item
            .response
            .child_transactions
            .iter()
            .filter(|txn| txn.total == refund_amount)
            .collect();

        let mut refund_txn = filtered_txn
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        for child in filtered_txn.iter() {
            if child.created_at > refund_txn.created_at {
                refund_txn = child;
            }
        }

        let refund_status = match refund_txn.success {
            true => enums::RefundStatus::Success,
            false => enums::RefundStatus::Failure,
        };

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: refund_txn.id.clone(),
                refund_status,
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
        let refund_status = match item.response.success {
            true => enums::RefundStatus::Success,
            false => enums::RefundStatus::Failure,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxWebhookEventType {
    PreAuth,
    Capture,
    Charge,
    Void,
    Refund,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct StaxWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: StaxWebhookEventType,
    pub id: String,
    pub auth_id: Option<String>,
    pub success: bool,
}
