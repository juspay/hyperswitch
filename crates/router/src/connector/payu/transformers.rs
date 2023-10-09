use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::AccessTokenRequestInfo,
    consts,
    core::errors,
    pii::Secret,
    types::{self, api, storage::enums},
};

const WALLET_IDENTIFIER: &str = "PBL";

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentsRequest {
    customer_ip: std::net::IpAddr,
    merchant_pos_id: Secret<String>,
    total_amount: i64,
    currency_code: enums::Currency,
    description: String,
    pay_methods: PayuPaymentMethod,
    continue_url: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentMethod {
    pay_method: PayuPaymentMethodData,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PayuPaymentMethodData {
    Card(PayuCard),
    Wallet(PayuWallet),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PayuCard {
    #[serde(rename_all = "camelCase")]
    Card {
        number: cards::CardNumber,
        expiration_month: Secret<String>,
        expiration_year: Secret<String>,
        cvv: Secret<String>,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayuWallet {
    pub value: PayuWalletCode,
    #[serde(rename = "type")]
    pub wallet_type: String,
    pub authorization_code: String,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PayuWalletCode {
    Ap,
    Jp,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PayuPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth_type = PayuAuthType::try_from(&item.connector_auth_type)?;
        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => Ok(PayuPaymentMethod {
                pay_method: PayuPaymentMethodData::Card(PayuCard::Card {
                    number: ccard.card_number,
                    expiration_month: ccard.card_exp_month,
                    expiration_year: ccard.card_exp_year,
                    cvv: ccard.card_cvc,
                }),
            }),
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                api_models::payments::WalletData::GooglePay(data) => Ok(PayuPaymentMethod {
                    pay_method: PayuPaymentMethodData::Wallet({
                        PayuWallet {
                            value: PayuWalletCode::Ap,
                            wallet_type: WALLET_IDENTIFIER.to_string(),
                            authorization_code: consts::BASE64_ENGINE
                                .encode(data.tokenization_data.token),
                        }
                    }),
                }),
                api_models::payments::WalletData::ApplePay(data) => Ok(PayuPaymentMethod {
                    pay_method: PayuPaymentMethodData::Wallet({
                        PayuWallet {
                            value: PayuWalletCode::Jp,
                            wallet_type: WALLET_IDENTIFIER.to_string(),
                            authorization_code: data.payment_data,
                        }
                    }),
                }),
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Unknown Wallet in Payment Method".to_string(),
                )),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        let browser_info = item.request.browser_info.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info",
            },
        )?;
        Ok(Self {
            customer_ip: browser_info.ip_address.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "browser_info.ip_address",
                },
            )?,
            merchant_pos_id: auth_type.merchant_pos_id,
            total_amount: item.request.amount,
            currency_code: item.request.currency,
            description: item.description.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "item.description",
                },
            )?,
            pay_methods: payment_method,
            continue_url: None,
        })
    }
}

pub struct PayuAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_pos_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PayuAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_pos_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayuPaymentStatus {
    Success,
    WarningContinueRedirect,
    #[serde(rename = "WARNING_CONTINUE_3DS")]
    WarningContinue3ds,
    WarningContinueCvv,
    #[default]
    Pending,
}

impl From<PayuPaymentStatus> for enums::AttemptStatus {
    fn from(item: PayuPaymentStatus) -> Self {
        match item {
            PayuPaymentStatus::Success => Self::Pending,
            PayuPaymentStatus::WarningContinue3ds => Self::Pending,
            PayuPaymentStatus::WarningContinueCvv => Self::Pending,
            PayuPaymentStatus::WarningContinueRedirect => Self::Pending,
            PayuPaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentsResponse {
    pub status: PayuPaymentStatusData,
    pub redirect_uri: String,
    pub iframe_allowed: Option<bool>,
    pub three_ds_protocol_version: Option<String>,
    pub order_id: String,
    pub ext_order_id: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayuPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayuPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.status_code),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .ext_order_id
                    .or(Some(item.response.order_id)),
            }),
            amount_captured: None,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentsCaptureRequest {
    order_id: String,
    order_status: OrderStatus,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for PayuPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            order_id: item.request.connector_transaction_id.clone(),
            order_status: OrderStatus::Completed,
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct PayuPaymentsCaptureResponse {
    status: PayuPaymentStatusData,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PayuPaymentsCaptureResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PayuPaymentsCaptureResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.status_code.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            amount_captured: None,
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayuAuthUpdateRequest {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
}

impl TryFrom<&types::RefreshTokenRouterData> for PayuAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
            client_id: item.get_request_id()?,
            client_secret: item.request.app_id.clone(),
        })
    }
}
#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct PayuAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub grant_type: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayuAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayuAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentsCancelResponse {
    pub order_id: String,
    pub ext_order_id: Option<String>,
    pub status: PayuPaymentStatusData,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PayuPaymentsCancelResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PayuPaymentsCancelResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.status_code.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .ext_order_id
                    .or(Some(item.response.order_id)),
            }),
            amount_captured: None,
            ..item.data
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Eq, PartialEq, Default, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    Canceled,
    Completed,
    WaitingForConfirmation,
    #[default]
    Pending,
}

impl From<OrderStatus> for enums::AttemptStatus {
    fn from(item: OrderStatus) -> Self {
        match item {
            OrderStatus::New => Self::PaymentMethodAwaited,
            OrderStatus::Canceled => Self::Voided,
            OrderStatus::Completed => Self::Charged,
            OrderStatus::Pending => Self::Pending,
            OrderStatus::WaitingForConfirmation => Self::Authorized,
        }
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuPaymentStatusData {
    status_code: PayuPaymentStatus,
    severity: Option<String>,
    status_desc: Option<String>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuProductData {
    name: String,
    unit_price: String,
    quantity: String,
    #[serde(rename = "virtual")]
    virtually: Option<bool>,
    listing_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuOrderResponseData {
    order_id: String,
    ext_order_id: Option<String>,
    order_create_date: String,
    notify_url: Option<String>,
    customer_ip: std::net::IpAddr,
    merchant_pos_id: String,
    description: String,
    validity_time: Option<String>,
    currency_code: enums::Currency,
    total_amount: String,
    buyer: Option<PayuOrderResponseBuyerData>,
    pay_method: Option<PayuOrderResponsePayMethod>,
    products: Option<Vec<PayuProductData>>,
    status: OrderStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuOrderResponseBuyerData {
    ext_customer_id: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    #[serde(rename = "nin")]
    national_identification_number: Option<String>,
    language: Option<String>,
    delivery: Option<String>,
    customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayuOrderResponsePayMethod {
    CardToken,
    Pbl,
    Installemnts,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayuOrderResponseProperty {
    name: String,
    value: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct PayuPaymentsSyncResponse {
    orders: Vec<PayuOrderResponseData>,
    status: PayuPaymentStatusData,
    properties: Option<Vec<PayuOrderResponseProperty>>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayuPaymentsSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PayuPaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let order = match item.response.orders.first() {
            Some(order) => order,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(order.status.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(order.order_id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: order
                    .ext_order_id
                    .clone()
                    .or(Some(order.order_id.clone())),
            }),
            amount_captured: Some(
                order
                    .total_amount
                    .parse::<i64>()
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
            ),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Eq, PartialEq, Serialize)]
pub struct PayuRefundRequestData {
    description: String,
    amount: Option<i64>,
}

#[derive(Default, Debug, Serialize)]
pub struct PayuRefundRequest {
    refund: PayuRefundRequestData,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PayuRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            refund: PayuRefundRequestData {
                description: item.request.reason.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "item.request.reason",
                    },
                )?,
                amount: None,
            },
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Eq, PartialEq, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Finalized,
    Completed,
    Canceled,
    #[default]
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Finalized | RefundStatus::Completed => Self::Success,
            RefundStatus::Canceled => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayuRefundResponseData {
    refund_id: String,
    ext_refund_id: String,
    amount: String,
    currency_code: enums::Currency,
    description: String,
    creation_date_time: String,
    status: RefundStatus,
    status_date_time: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund: PayuRefundResponseData,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.refund.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund.refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundSyncResponse {
    refunds: Vec<PayuRefundResponseData>,
}
impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund = match item.response.refunds.first() {
            Some(refund) => refund,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: refund.refund_id.clone(),
                refund_status: enums::RefundStatus::from(refund.status.clone()),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayuErrorData {
    pub status_code: String,
    pub code: Option<String>,
    pub code_literal: Option<String>,
    pub status_desc: String,
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayuErrorResponse {
    pub status: PayuErrorData,
}

#[derive(Deserialize, Debug)]
pub struct PayuAccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}
