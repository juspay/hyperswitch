use std::net::IpAddr;

use api_models::payments::{ApplePayRedirectData, Card, GooglePayWalletData};
use cards::CardNumber;
use common_utils::{ext_traits::ValueExt, pii::Email};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use ring::digest;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    connector::utils::{
        self, BrowserInformationData, CardData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors::{self, CustomResult},
    services::{self, Method},
    types::{self, api, storage::enums, transformers::ForeignTryFrom},
    utils::OptionExt,
};
// Auth Struct
pub struct ZenAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for ZenAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequest {
    merchant_transaction_id: String,
    payment_channel: ZenPaymentChannels,
    amount: String,
    currency: enums::Currency,
    payment_specific_data: ZenPaymentData,
    customer: ZenCustomerDetails,
    custom_ipn_url: String,
    items: Vec<ZenItemObject>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ZenPaymentsRequest {
    ApiRequest(Box<ApiRequest>),
    CheckoutRequest(Box<CheckoutRequest>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutRequest {
    amount: String,
    currency: enums::Currency,
    custom_ipn_url: String,
    items: Vec<ZenItemObject>,
    merchant_transaction_id: String,
    signature: Option<Secret<String>>,
    specified_payment_channel: ZenPaymentChannels,
    terminal_uuid: Secret<String>,
    url_redirect: String,
}

#[derive(Clone, Debug, Display, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
pub enum ZenPaymentChannels {
    PclCard,
    PclGooglepay,
    PclApplepay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenCustomerDetails {
    email: Email,
    ip: IpAddr,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenPaymentData {
    browser_details: ZenBrowserDetails,
    #[serde(rename = "type")]
    payment_type: ZenPaymentTypes,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card: Option<ZenCardDetails>,
    descriptor: String,
    return_verify_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenBrowserDetails {
    color_depth: String,
    java_enabled: bool,
    lang: String,
    screen_height: String,
    screen_width: String,
    timezone: String,
    accept_header: String,
    window_size: String,
    user_agent: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ZenPaymentTypes {
    Onetime,
    ExternalPaymentToken,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenCardDetails {
    number: CardNumber,
    expiry_date: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenItemObject {
    name: String,
    price: String,
    quantity: u16,
    line_amount_total: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionObject {
    pub apple_pay: Option<ApplePaySessionData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplePaySessionData {
    pub terminal_uuid: Option<String>,
    pub pay_wall_secret: Option<String>,
}

impl TryFrom<(&types::PaymentsAuthorizeRouterData, &Card)> for ZenPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: (&types::PaymentsAuthorizeRouterData, &Card)) -> Result<Self, Self::Error> {
        let (item, ccard) = value;
        let browser_info = item.request.get_browser_info()?;
        let ip = browser_info.get_ip_address()?;
        let browser_details = get_browser_details(&browser_info)?;
        let amount = utils::to_currency_base_unit(item.request.amount, item.request.currency)?;
        let payment_specific_data = ZenPaymentData {
            browser_details,
            //Connector Specific for cards
            payment_type: ZenPaymentTypes::Onetime,
            token: None,
            card: Some(ZenCardDetails {
                number: ccard.card_number.clone(),
                expiry_date: ccard.get_card_expiry_month_year_2_digit_with_delimiter("".to_owned()),
                cvv: ccard.card_cvc.clone(),
            }),
            descriptor: item.get_description()?.chars().take(24).collect(),
            return_verify_url: item.request.router_return_url.clone(),
        };
        Ok(Self::ApiRequest(Box::new(ApiRequest {
            merchant_transaction_id: item.attempt_id.clone(),
            payment_channel: ZenPaymentChannels::PclCard,
            currency: item.request.currency,
            payment_specific_data,
            customer: get_customer(item, ip)?,
            custom_ipn_url: item.request.get_webhook_url()?,
            items: get_item_object(item, amount.clone())?,
            amount,
        })))
    }
}

impl TryFrom<(&types::PaymentsAuthorizeRouterData, &GooglePayWalletData)> for ZenPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, gpay_pay_redirect_data): (&types::PaymentsAuthorizeRouterData, &GooglePayWalletData),
    ) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_base_unit(item.request.amount, item.request.currency)?;
        let browser_info = item.request.get_browser_info()?;
        let browser_details = get_browser_details(&browser_info)?;
        let ip = browser_info.get_ip_address()?;
        let payment_specific_data = ZenPaymentData {
            browser_details,
            //Connector Specific for wallet
            payment_type: ZenPaymentTypes::ExternalPaymentToken,
            token: Some(gpay_pay_redirect_data.tokenization_data.token.clone()),
            card: None,
            descriptor: item.get_description()?.chars().take(24).collect(),
            return_verify_url: item.request.router_return_url.clone(),
        };
        Ok(Self::ApiRequest(Box::new(ApiRequest {
            merchant_transaction_id: item.attempt_id.clone(),
            payment_channel: ZenPaymentChannels::PclGooglepay,
            currency: item.request.currency,
            payment_specific_data,
            customer: get_customer(item, ip)?,
            custom_ipn_url: item.request.get_webhook_url()?,
            items: get_item_object(item, amount.clone())?,
            amount,
        })))
    }
}

impl
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &Box<ApplePayRedirectData>,
    )> for ZenPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, _apple_pay_redirect_data): (
            &types::PaymentsAuthorizeRouterData,
            &Box<ApplePayRedirectData>,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_base_unit(item.request.amount, item.request.currency)?;
        let connector_meta = item.get_connector_meta()?;
        let session: SessionObject = connector_meta
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let applepay_session_data = session
            .apple_pay
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let terminal_uuid = applepay_session_data
            .terminal_uuid
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let mut checkout_request = CheckoutRequest {
            merchant_transaction_id: item.attempt_id.clone(),
            specified_payment_channel: ZenPaymentChannels::PclApplepay,
            currency: item.request.currency,
            custom_ipn_url: item.request.get_webhook_url()?,
            items: get_item_object(item, amount.clone())?,
            amount,
            terminal_uuid: Secret::new(terminal_uuid),
            signature: None,
            url_redirect: item.request.get_return_url()?,
        };
        checkout_request.signature = Some(get_checkout_signature(
            &checkout_request,
            &applepay_session_data,
        )?);
        Ok(Self::CheckoutRequest(Box::new(checkout_request)))
    }
}

fn get_checkout_signature(
    checkout_request: &CheckoutRequest,
    session: &ApplePaySessionData,
) -> Result<Secret<String>, error_stack::Report<errors::ConnectorError>> {
    let pay_wall_secret = session
        .pay_wall_secret
        .clone()
        .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
    let mut signature_data = get_signature_data(checkout_request);
    signature_data.push_str(&pay_wall_secret);
    let payload_digest = digest::digest(&digest::SHA256, signature_data.as_bytes());
    let mut signature = hex::encode(payload_digest);
    signature.push_str(";sha256");
    Ok(Secret::new(signature))
}

/// Fields should be in alphabetical order
fn get_signature_data(checkout_request: &CheckoutRequest) -> String {
    let specified_payment_channel = match checkout_request.specified_payment_channel {
        ZenPaymentChannels::PclCard => "pcl_card",
        ZenPaymentChannels::PclGooglepay => "pcl_googlepay",
        ZenPaymentChannels::PclApplepay => "pcl_applepay",
    };
    let mut signature_data = vec![
        format!("amount={}", checkout_request.amount),
        format!("currency={}", checkout_request.currency),
        format!("customipnurl={}", checkout_request.custom_ipn_url),
    ];
    for index in 0..checkout_request.items.len() {
        let prefix = format!("items[{index}].");
        signature_data.push(format!(
            "{prefix}lineamounttotal={}",
            checkout_request.items[index].line_amount_total
        ));
        signature_data.push(format!(
            "{prefix}name={}",
            checkout_request.items[index].name
        ));
        signature_data.push(format!(
            "{prefix}price={}",
            checkout_request.items[index].price
        ));
        signature_data.push(format!(
            "{prefix}quantity={}",
            checkout_request.items[index].quantity
        ));
    }
    signature_data.push(format!(
        "merchanttransactionid={}",
        checkout_request.merchant_transaction_id
    ));
    signature_data.push(format!(
        "specifiedpaymentchannel={specified_payment_channel}"
    ));
    signature_data.push(format!(
        "terminaluuid={}",
        checkout_request.terminal_uuid.peek()
    ));
    signature_data.push(format!("urlredirect={}", checkout_request.url_redirect));
    let signature = signature_data.join("&");
    signature.to_lowercase()
}

fn get_customer(
    item: &types::PaymentsAuthorizeRouterData,
    ip: IpAddr,
) -> Result<ZenCustomerDetails, error_stack::Report<errors::ConnectorError>> {
    Ok(ZenCustomerDetails {
        email: item.request.get_email()?,
        ip,
    })
}

fn get_item_object(
    item: &types::PaymentsAuthorizeRouterData,
    amount: String,
) -> Result<Vec<ZenItemObject>, error_stack::Report<errors::ConnectorError>> {
    let order_details = item.request.get_order_details()?;
    Ok(vec![ZenItemObject {
        name: order_details.product_name,
        price: amount.clone(),
        quantity: 1,
        line_amount_total: amount,
    }])
}

fn get_browser_details(
    browser_info: &types::BrowserInformation,
) -> CustomResult<ZenBrowserDetails, errors::ConnectorError> {
    let screen_height = browser_info
        .screen_height
        .get_required_value("screen_height")
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "screen_height",
        })?;

    let screen_width = browser_info
        .screen_width
        .get_required_value("screen_width")
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "screen_width",
        })?;

    let window_size = match (screen_height, screen_width) {
        (250, 400) => "01",
        (390, 400) => "02",
        (500, 600) => "03",
        (600, 400) => "04",
        _ => "05",
    }
    .to_string();

    Ok(ZenBrowserDetails {
        color_depth: browser_info
            .color_depth
            .get_required_value("color_depth")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "color_depth",
            })?
            .to_string(),
        java_enabled: browser_info
            .java_enabled
            .get_required_value("java_enabled")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "java_enabled",
            })?,
        lang: browser_info
            .language
            .clone()
            .get_required_value("language")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "language",
            })?,
        screen_height: screen_height.to_string(),
        screen_width: screen_width.to_string(),
        timezone: browser_info
            .time_zone
            .get_required_value("time_zone")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "time_zone",
            })?
            .to_string(),
        accept_header: browser_info
            .accept_header
            .clone()
            .get_required_value("accept_header")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "accept_header",
            })?,
        user_agent: browser_info
            .user_agent
            .clone()
            .get_required_value("user_agent")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "user_agent",
            })?,
        window_size,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for ZenPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(card) => Self::try_from((item, card)),
            api_models::payments::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                api_models::payments::WalletData::ApplePayRedirect(apple_pay_redirect_data) => {
                    Self::try_from((item, apple_pay_redirect_data))
                }
                api_models::payments::WalletData::GooglePay(gpay_redirect_data) => {
                    Self::try_from((item, gpay_redirect_data))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "payment method".to_string(),
                ))?,
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            ))?,
        }
    }
}

// PaymentsResponse
#[derive(Debug, Default, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZenPaymentStatus {
    Authorized,
    Accepted,
    #[default]
    Pending,
    Rejected,
    Canceled,
}

impl ForeignTryFrom<(ZenPaymentStatus, Option<ZenActions>)> for enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(item: (ZenPaymentStatus, Option<ZenActions>)) -> Result<Self, Self::Error> {
        let (item_txn_status, item_action_status) = item;
        Ok(match item_txn_status {
            // Payment has been authorized at connector end, They will send webhook when it gets accepted
            ZenPaymentStatus::Authorized => Self::Pending,
            ZenPaymentStatus::Accepted => Self::Charged,
            ZenPaymentStatus::Pending => {
                item_action_status.map_or(Self::Pending, |action| match action {
                    ZenActions::Redirect => Self::AuthenticationPending,
                })
            }
            ZenPaymentStatus::Rejected => Self::Failure,
            ZenPaymentStatus::Canceled => Self::Voided,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    status: ZenPaymentStatus,
    id: String,
    merchant_action: Option<ZenMerchantAction>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ZenPaymentsResponse {
    ApiResponse(ApiResponse),
    CheckoutResponse(CheckoutResponse),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutResponse {
    redirect_url: url::Url,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenMerchantAction {
    action: ZenActions,
    data: ZenMerchantActionData,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZenActions {
    Redirect,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenMerchantActionData {
    redirect_url: url::Url,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ZenPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ZenPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            ZenPaymentsResponse::ApiResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
            ZenPaymentsResponse::CheckoutResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, ApiResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: types::ResponseRouterData<F, ApiResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data_action = value.response.merchant_action.map(|merchant_action| {
            (
                services::RedirectForm::from((merchant_action.data.redirect_url, Method::Get)),
                merchant_action.action,
            )
        });
        let (redirection_data, action) = match redirection_data_action {
            Some((redirect_form, action)) => (Some(redirect_form), Some(action)),
            None => (None, None),
        };

        Ok(Self {
            status: enums::AttemptStatus::foreign_try_from((value.response.status, action))?,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(value.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..value.data
        })
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, CheckoutResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: types::ResponseRouterData<F, CheckoutResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = Some(services::RedirectForm::from((
            value.response.redirect_url,
            Method::Get,
        )));
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..value.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenRefundRequest {
    amount: String,
    transaction_id: String,
    currency: enums::Currency,
    merchant_transaction_id: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ZenRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?,
            transaction_id: item.request.connector_transaction_id.clone(),
            currency: item.request.currency,
            merchant_transaction_id: item.request.refund_id.clone(),
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Authorized,
    Accepted,
    #[default]
    Pending,
    Rejected,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Accepted => Self::Success,
            RefundStatus::Pending | RefundStatus::Authorized => Self::Pending,
            RefundStatus::Rejected => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
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
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenWebhookBody {
    #[serde(rename = "transactionId")]
    pub id: String,
    pub merchant_transaction_id: String,
    pub amount: String,
    pub currency: String,
    pub status: ZenPaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZenWebhookSignature {
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenWebhookObjectReference {
    #[serde(rename = "type")]
    pub transaction_type: ZenWebhookTxnType,
    pub transaction_id: String,
    pub merchant_transaction_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenWebhookEventType {
    #[serde(rename = "type")]
    pub transaction_type: ZenWebhookTxnType,
    pub transaction_id: String,
    pub status: ZenPaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZenWebhookTxnType {
    TrtPurchase,
    TrtRefund,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct ZenErrorResponse {
    pub error: ZenErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ZenErrorBody {
    pub message: String,
    pub code: String,
}
