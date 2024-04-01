use common_utils::pii::{Email, SecretSerdeValue};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData},
    consts,
    core::errors,
    types::{self, api, storage::enums},
};

pub struct BillwerkRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for BillwerkRouterData<T>
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
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub struct BillwerkAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) public_api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for BillwerkAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                public_api_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillwerkTokenRequestIntent {
    ChargeAndStore,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillwerkStrongAuthRule {
    UseScaIfAvailableAuth,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillwerkTokenRequest {
    number: cards::CardNumber,
    month: Secret<String>,
    year: Secret<String>,
    cvv: Secret<String>,
    pkey: Secret<String>,
    recurring: Option<bool>,
    intent: Option<BillwerkTokenRequestIntent>,
    strong_authentication_rule: Option<BillwerkStrongAuthRule>,
}

impl TryFrom<&types::TokenizationRouterData> for BillwerkTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let connector_auth = &item.connector_auth_type;
                let auth_type = BillwerkAuthType::try_from(connector_auth)?;
                Ok(Self {
                    number: ccard.card_number.clone(),
                    month: ccard.card_exp_month.clone(),
                    year: ccard.get_card_expiry_year_2_digit()?,
                    cvv: ccard.card_cvc,
                    pkey: auth_type.public_api_key,
                    recurring: None,
                    intent: None,
                    strong_authentication_rule: None,
                })
            }
            api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("billwerk"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BillwerkTokenResponse {
    id: Secret<String>,
    recurring: Option<bool>,
}

impl<T>
    TryFrom<
        types::ResponseRouterData<
            api::PaymentMethodToken,
            BillwerkTokenResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::PaymentMethodToken, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::PaymentMethodToken,
            BillwerkTokenResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct BillwerkCustomerObject {
    handle: Option<String>,
    email: Option<Email>,
    address: Option<Secret<String>>,
    address2: Option<Secret<String>>,
    city: Option<String>,
    country: Option<common_enums::CountryAlpha2>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct BillwerkPaymentsRequest {
    handle: String,
    amount: i64,
    source: Secret<String>,
    currency: common_enums::Currency,
    customer: BillwerkCustomerObject,
    metadata: Option<SecretSerdeValue>,
    settle: bool,
}

impl TryFrom<&BillwerkRouterData<&types::PaymentsAuthorizeRouterData>> for BillwerkPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BillwerkRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotImplemented(format!(
                "Three_ds payments through Billwerk"
            ))
            .into());
        };
        let source = match item.router_data.get_payment_method_token()? {
            types::PaymentMethodToken::Token(pm_token) => Ok(Secret::new(pm_token)),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_token",
            }),
        }?;
        Ok(Self {
            handle: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount,
            source,
            currency: item.router_data.request.currency,
            customer: BillwerkCustomerObject {
                handle: item.router_data.customer_id.clone(),
                email: item.router_data.request.email.clone(),
                address: item.router_data.get_optional_billing_line1(),
                address2: item.router_data.get_optional_billing_line2(),
                city: item.router_data.get_optional_billing_city(),
                country: item.router_data.get_optional_billing_country(),
                first_name: item.router_data.get_optional_billing_first_name(),
                last_name: item.router_data.get_optional_billing_last_name(),
            },
            metadata: item.router_data.request.metadata.clone(),
            settle: item.router_data.request.is_auto_capture()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BillwerkPaymentState {
    Created,
    Authorized,
    Pending,
    Settled,
    Failed,
    Cancelled,
}

impl From<BillwerkPaymentState> for enums::AttemptStatus {
    fn from(item: BillwerkPaymentState) -> Self {
        match item {
            BillwerkPaymentState::Created | BillwerkPaymentState::Pending => Self::Pending,
            BillwerkPaymentState::Authorized => Self::Authorized,
            BillwerkPaymentState::Settled => Self::Charged,
            BillwerkPaymentState::Failed => Self::Failure,
            BillwerkPaymentState::Cancelled => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillwerkPaymentsResponse {
    state: BillwerkPaymentState,
    handle: String,
    error: Option<String>,
    error_state: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BillwerkPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BillwerkPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let error_response = if item.response.error.is_some() || item.response.error_state.is_some()
        {
            Some(types::ErrorResponse {
                code: item
                    .response
                    .error_state
                    .clone()
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .error_state
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.error,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.handle.clone()),
            })
        } else {
            None
        };
        let payments_response = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(item.response.handle.clone()),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(item.response.handle),
            incremental_authorization_allowed: None,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.state),
            response: error_response.map_or_else(|| Ok(payments_response), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct BillwerkCaptureRequest {
    amount: i64,
}

impl TryFrom<&BillwerkRouterData<&types::PaymentsCaptureRouterData>> for BillwerkCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BillwerkRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
        })
    }
}

// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct BillwerkRefundRequest {
    pub invoice: String,
    pub amount: i64,
    pub text: Option<String>,
}

impl<F> TryFrom<&BillwerkRouterData<&types::RefundsRouterData<F>>> for BillwerkRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BillwerkRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            invoice: item.router_data.request.connector_transaction_id.clone(),
            text: item.router_data.request.reason.clone(),
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefundState {
    Refunded,
    Failed,
    Processing,
}

impl From<RefundState> for enums::RefundStatus {
    fn from(item: RefundState) -> Self {
        match item {
            RefundState::Refunded => Self::Success,
            RefundState::Failed => Self::Failure,
            RefundState::Processing => Self::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    state: RefundState,
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
                refund_status: enums::RefundStatus::from(item.response.state),
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
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BillwerkErrorResponse {
    pub code: Option<i32>,
    pub error: String,
    pub message: Option<String>,
}
