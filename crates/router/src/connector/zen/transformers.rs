use std::net::IpAddr;

use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    pii,
    services::{self, Method},
    types::{self, api, storage::enums, transformers::ForeignTryFrom},
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

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenPaymentsRequest {
    merchant_transaction_id: String,
    payment_channel: ZenPaymentChannels,
    amount: String,
    currency: enums::Currency,
    payment_specific_data: ZenPaymentData,
    customer: ZenCustomerDetails,
    custom_ipn_url: String,
    items: Vec<ZenItemObject>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZenPaymentChannels {
    PclCard,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenCustomerDetails {
    email: Secret<String, pii::Email>,
    ip: IpAddr,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenPaymentData {
    browser_details: ZenBrowserDetails,
    #[serde(rename = "type")]
    payment_type: ZenPaymentTypes,
    card: ZenCardDetails,
    descriptor: String,
    return_verify_url: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq, frunk::LabelledGeneric)]
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

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ZenPaymentTypes {
    Onetime,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenCardDetails {
    number: Secret<String, pii::CardNumber>,
    expiry_date: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenItemObject {
    name: String,
    price: String,
    quantity: u16,
    line_amount_total: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for ZenPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let browser_info = item.request.get_browser_info()?;
        let order_details = item.request.get_order_details()?;

        let window_size = match (browser_info.screen_height, browser_info.screen_width) {
            (250, 400) => "01",
            (390, 400) => "02",
            (500, 600) => "03",
            (600, 400) => "04",
            _ => "05",
        }
        .to_string();
        let browser_details = ZenBrowserDetails {
            color_depth: browser_info.color_depth.to_string(),
            java_enabled: browser_info.java_enabled,
            lang: browser_info.language,
            screen_height: browser_info.screen_height.to_string(),
            screen_width: browser_info.screen_width.to_string(),
            timezone: browser_info.time_zone.to_string(),
            accept_header: browser_info.accept_header,
            window_size,
            user_agent: browser_info.user_agent,
        };
        let (payment_specific_data, payment_channel) =
            match item.request.payment_method_data.clone() {
                api::PaymentMethodData::Card(ccard) => Ok((
                    ZenPaymentData {
                        browser_details,
                        //Connector Specific for cards
                        payment_type: ZenPaymentTypes::Onetime,
                        card: ZenCardDetails {
                            number: ccard.card_number.clone(),
                            expiry_date: Secret::new(format!(
                                "{}{}",
                                ccard.card_exp_month.clone().expose(),
                                ccard.get_card_expiry_year_2_digit().expose()
                            )),
                            cvv: ccard.card_cvc,
                        },
                        descriptor: item.get_description()?.chars().take(24).collect(),
                        return_verify_url: item.request.router_return_url.clone(),
                    },
                    //Connector Specific for cards
                    ZenPaymentChannels::PclCard,
                )),
                _ => Err(errors::ConnectorError::NotImplemented(
                    "payment method".to_string(),
                )),
            }?;
        Ok(Self {
            merchant_transaction_id: item.payment_id.clone(),
            payment_channel,
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            payment_specific_data,
            customer: ZenCustomerDetails {
                email: item.request.get_email()?,
                ip: browser_info.ip_address.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info.ip_address",
                    },
                )?,
            },
            custom_ipn_url: item.request.get_webhook_url()?,
            items: vec![ZenItemObject {
                name: order_details.product_name.clone(),
                price: "65.40".to_string(),
                quantity: order_details.quantity,
                line_amount_total: "65.40".to_string(),
            }],
        })
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, strum::Display)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenPaymentsResponse {
    status: ZenPaymentStatus,
    id: String,
    redirect_url: Option<String>,
    #[serde(rename = "type")]
    transaction_type: String,
    merchant_action: Option<ZenMerchantAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenMerchantAction {
    action: ZenActions,
    data: ZenMerchantActionData,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZenActions {
    Redirect,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        let (redirection_data, action) = item
            .response
            .merchant_action
            .map(|merchant_action| {
                (
                    services::RedirectForm::from((merchant_action.data.redirect_url, Method::Get)),
                    merchant_action.action,
                )
            })
            .unzip();
        Ok(Self {
            status: enums::AttemptStatus::foreign_try_from((item.response.status, action))?,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
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

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    #[serde(rename = "type")]
    transaction_type: String,
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZenWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: ZenWebhookTxnType,
    pub transaction_id: String,
    pub merchant_transaction_id: String,
    pub amount: String,
    pub currency: String,
    pub status: ZenPaymentStatus,
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZenWebhookTxnType {
    TrtPurchase,
    TrtRefund,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ZenErrorResponse {
    pub error: ZenErrorBody,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ZenErrorBody {
    pub message: String,
    pub code: String,
}
