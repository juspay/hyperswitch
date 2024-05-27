use common_utils::pii;
use error_stack::ResultExt;
use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::utils as connector_utils;
use crate::{
    connector::utils::{self, is_payment_failure, CryptoData, PaymentsAuthorizeRequestData},
    consts,
    core::errors,
    services,
    types::{self, domain, storage::enums, transformers::ForeignTryFrom},
};

#[derive(Debug, Serialize)]
pub struct CryptopayRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&types::api::CurrencyUnit, enums::Currency, i64, T)> for CryptopayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct CryptopayPaymentsRequest {
    price_amount: String,
    price_currency: enums::Currency,
    pay_currency: String,
    success_redirect_url: Option<String>,
    unsuccess_redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<pii::SecretSerdeValue>,
    custom_id: String,
}

impl TryFrom<&CryptopayRouterData<&types::PaymentsAuthorizeRouterData>>
    for CryptopayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CryptopayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let cryptopay_request = match item.router_data.request.payment_method_data {
            domain::PaymentMethodData::Crypto(ref cryptodata) => {
                let pay_currency = cryptodata.get_pay_currency()?;
                Ok(Self {
                    price_amount: item.amount.to_owned(),
                    price_currency: item.router_data.request.currency,
                    pay_currency,
                    success_redirect_url: item.router_data.request.router_return_url.clone(),
                    unsuccess_redirect_url: item.router_data.request.router_return_url.clone(),
                    //Cryptopay only accepts metadata as Object. If any other type, payment will fail with error.
                    metadata: item.router_data.request.get_metadata_as_object(),
                    custom_id: item.router_data.connector_request_reference_id.clone(),
                })
            }
            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::MandatePayment {}
            | domain::PaymentMethodData::Reward {}
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("CryptoPay"),
                ))
            }
        }?;
        Ok(cryptopay_request)
    }
}

// Auth Struct
pub struct CryptopayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CryptopayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CryptopayPaymentStatus {
    New,
    Completed,
    Unresolved,
    Refunded,
    Cancelled,
}

impl From<CryptopayPaymentStatus> for enums::AttemptStatus {
    fn from(item: CryptopayPaymentStatus) -> Self {
        match item {
            CryptopayPaymentStatus::New => Self::AuthenticationPending,
            CryptopayPaymentStatus::Completed => Self::Charged,
            CryptopayPaymentStatus::Cancelled => Self::Failure,
            CryptopayPaymentStatus::Unresolved | CryptopayPaymentStatus::Refunded => {
                Self::Unresolved
            } //mapped refunded to Unresolved because refund api is not available, also merchant has done the action on the connector dashboard.
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptopayPaymentsResponse {
    data: CryptopayPaymentResponseData,
}

impl<F, T>
    ForeignTryFrom<(
        types::ResponseRouterData<F, CryptopayPaymentsResponse, T, types::PaymentsResponseData>,
        diesel_models::enums::Currency,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, currency): (
            types::ResponseRouterData<F, CryptopayPaymentsResponse, T, types::PaymentsResponseData>,
            diesel_models::enums::Currency,
        ),
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.data.status.clone());
        let response = if is_payment_failure(status) {
            let payment_response = &item.response.data;
            Err(types::ErrorResponse {
                code: payment_response
                    .name
                    .clone()
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: payment_response
                    .status_context
                    .clone()
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                reason: payment_response.status_context.clone(),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(payment_response.id.clone()),
            })
        } else {
            let redirection_data = item
                .response
                .data
                .hosted_page_url
                .map(|x| services::RedirectForm::from((x, services::Method::Get)));
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.data.id.clone(),
                ),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .data
                    .custom_id
                    .or(Some(item.response.data.id)),
                incremental_authorization_allowed: None,
                charge_id: None,
            })
        };

        match item.response.data.price_amount {
            Some(price_amount) => {
                let amount_captured = Some(
                    connector_utils::to_currency_lower_unit(price_amount, currency)?
                        .parse::<i64>()
                        .change_context(errors::ConnectorError::ParsingFailed)?,
                );

                Ok(Self {
                    status,
                    response,
                    amount_captured,
                    ..item.data
                })
            }
            None => Ok(Self {
                status,
                response,
                ..item.data
            }),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CryptopayErrorData {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CryptopayErrorResponse {
    pub error: CryptopayErrorData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptopayPaymentResponseData {
    pub id: String,
    pub custom_id: Option<String>,
    pub customer_id: Option<String>,
    pub status: CryptopayPaymentStatus,
    pub status_context: Option<String>,
    pub address: Option<Secret<String>>,
    pub network: Option<String>,
    pub uri: Option<String>,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    pub pay_amount: Option<String>,
    pub pay_currency: Option<String>,
    pub fee: Option<String>,
    pub fee_currency: Option<String>,
    pub paid_amount: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub success_redirect_url: Option<String>,
    pub unsuccess_redirect_url: Option<String>,
    pub hosted_page_url: Option<Url>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptopayWebhookDetails {
    #[serde(rename = "type")]
    pub service_type: String,
    pub event: WebhookEvent,
    pub data: CryptopayPaymentResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    TransactionCreated,
    TransactionConfirmed,
    StatusChanged,
}
