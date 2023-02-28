use common_utils::pii::Email;
use masking::ExposeInterface;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{self, AddressDetailsData, CardData, RouterData},
    core::errors,
    pii::{self, Secret},
    services,
    types::{self, api, storage::enums},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Direct,
    Redirect,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Gateway {
    Amex,
    CreditCard,
    Discover,
    Maestro,
    MasterCard,
    Visa,
    Klarna,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Coupons {
    pub allow: Option<Vec<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Mistercash {
    pub mobile_pay_button_position: Option<String>,
    pub disable_mobile_pay_button: Option<String>,
    pub qr_only: Option<String>,
    pub qr_size: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Gateways {
    pub mistercash: Option<Mistercash>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Settings {
    pub coupons: Option<Coupons>,
    pub gateways: Option<Gateways>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentOptions {
    pub notification_url: Option<String>,
    pub notification_method: Option<String>,
    pub redirect_url: Option<String>,
    pub cancel_url: Option<String>,
    pub close_window: Option<bool>,
    pub settings: Option<Settings>,
    pub template_id: Option<String>,
    pub allowed_countries: Option<Vec<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Browser {
    pub javascript_enabled: Option<bool>,
    pub java_enabled: Option<bool>,
    pub cookies_enabled: Option<bool>,
    pub language: Option<String>,
    pub screen_color_depth: Option<i32>,
    pub screen_height: Option<i32>,
    pub screen_width: Option<i32>,
    pub time_zone: Option<i32>,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Customer {
    pub browser: Option<Browser>,
    pub locale: Option<String>,
    pub ip_address: Option<String>,
    pub forward_ip: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub gender: Option<String>,
    pub birthday: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub house_number: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub email: Option<Secret<String, Email>>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub reference: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GatewayInfo {
    pub card_number: Option<Secret<String, pii::CardNumber>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_expiry_date: Option<i32>,
    pub card_cvc: Option<Secret<String>>,
    pub flexible_3d: Option<bool>,
    pub moto: Option<bool>,
    pub term_url: Option<String>,
    pub email: Option<Secret<String, Email>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeliveryObject {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    house_number: Secret<String>,
    zip_code: Secret<String>,
    city: String,
    country: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DefaultObject {
    shipping_taxed: bool,
    rate: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TaxObject {
    pub default: DefaultObject,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CheckoutOptions {
    pub validate_cart: Option<bool>,
    pub tax_tables: TaxObject,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Item {
    pub name: String,
    pub unit_price: f64,
    pub description: Option<String>,
    pub quantity: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ShoppingCart {
    pub items: Vec<Item>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MultisafepayPaymentsRequest {
    #[serde(rename = "type")]
    pub payment_type: Type,
    pub gateway: Gateway,
    pub order_id: String,
    pub currency: String,
    pub amount: i64,
    pub description: String,
    pub payment_options: Option<PaymentOptions>,
    pub customer: Option<Customer>,
    pub gateway_info: Option<GatewayInfo>,
    pub delivery: Option<DeliveryObject>,
    pub checkout_options: Option<CheckoutOptions>,
    pub shopping_cart: Option<ShoppingCart>,
    pub items: Option<String>,
    pub recurring_model: Option<String>,
    pub recurring_id: Option<String>,
    pub capture: Option<String>,
    pub days_active: Option<i32>,
    pub seconds_active: Option<i32>,
    pub var1: Option<String>,
    pub var2: Option<String>,
    pub var3: Option<String>,
}

impl From<utils::CardIssuer> for Gateway {
    fn from(issuer: utils::CardIssuer) -> Self {
        match issuer {
            utils::CardIssuer::AmericanExpress => Self::Amex,
            utils::CardIssuer::Master => Self::MasterCard,
            utils::CardIssuer::Maestro => Self::Maestro,
            utils::CardIssuer::Visa => Self::Visa,
            utils::CardIssuer::Discover => Self::Discover,
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for MultisafepayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payment_type = match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref _ccard) => Type::Direct,
            api::PaymentMethodData::PayLater(ref _paylater) => Type::Redirect,
            _ => Type::Redirect,
        };

        let gateway = match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => Gateway::from(ccard.get_card_issuer()?),
            api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::KlarnaRedirect {
                    billing_email: _,
                    billing_country: _,
                },
            ) => Gateway::Klarna,
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };
        let description = item.get_description()?;
        let payment_options = PaymentOptions {
            notification_url: None,
            redirect_url: item.router_return_url.clone(),
            cancel_url: None,
            close_window: None,
            notification_method: None,
            settings: None,
            template_id: None,
            allowed_countries: None,
        };

        let customer = Customer {
            browser: None,
            locale: None,
            ip_address: None,
            forward_ip: None,
            first_name: None,
            last_name: None,
            gender: None,
            birthday: None,
            address1: None,
            address2: None,
            house_number: None,
            zip_code: None,
            city: None,
            state: None,
            country: None,
            phone: None,
            email: item.request.email.clone(),
            user_agent: None,
            referrer: None,
            reference: None,
        };

        let billing_address = item
            .get_billing()?
            .address
            .as_ref()
            .ok_or_else(utils::missing_field_err("billing.address"))?;
        let delivery = DeliveryObject {
            first_name: billing_address.get_first_name()?.to_owned(),
            last_name: billing_address.get_last_name()?.to_owned(),
            address1: billing_address.get_line1()?.to_owned(),
            house_number: billing_address.get_line2()?.to_owned(),
            zip_code: billing_address.get_zip()?.to_owned(),
            city: billing_address.get_city()?.to_owned(),
            country: billing_address.get_country()?.to_owned(),
        };

        let gateway_info = match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => GatewayInfo {
                card_number: Some(ccard.card_number.clone()),
                card_expiry_date: Some(
                    (format!(
                        "{}{}",
                        ccard.get_card_expiry_year_2_digit().expose(),
                        ccard.card_exp_month.clone().expose()
                    ))
                    .parse::<i32>()
                    .unwrap_or_default(),
                ),
                card_cvc: Some(ccard.card_cvc.clone()),
                card_holder_name: None,
                flexible_3d: None,
                moto: None,
                term_url: None,
                email: None,
            },
            api::PaymentMethodData::PayLater(ref paylater) => GatewayInfo {
                card_number: None,
                card_expiry_date: None,
                card_cvc: None,
                card_holder_name: None,
                flexible_3d: None,
                moto: None,
                term_url: None,
                email: Some(match paylater {
                    api_models::payments::PayLaterData::KlarnaRedirect {
                        billing_email,
                        billing_country: _,
                    } => billing_email.clone(),
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Only KlarnaRedirect is implemented".to_string(),
                    ))?,
                }),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };

        Ok(Self {
            payment_type,
            gateway,
            order_id: item.payment_id.to_string(),
            currency: item.request.currency.to_string(),
            amount: item.request.amount,
            description,
            payment_options: Some(payment_options),
            customer: Some(customer),
            delivery: Some(delivery),
            gateway_info: Some(gateway_info),
            checkout_options: None,
            shopping_cart: None,
            capture: None,
            items: None,
            recurring_model: if item.request.setup_future_usage
                == Some(enums::FutureUsage::OffSession)
            {
                Some("Unscheduled".to_string())
            } else {
                None
            },
            recurring_id: item
                .request
                .mandate_id
                .clone()
                .and_then(|mandate_ids| mandate_ids.connector_mandate_id),
            days_active: Some(30),
            seconds_active: Some(259200),
            var1: None,
            var2: None,
            var3: None,
        })
    }
}

// Auth Struct
pub struct MultisafepayAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for MultisafepayAuthType {
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
// PaymentsResponse
#[derive(Debug, Clone, Default, Eq, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MultisafepayPaymentStatus {
    Completed,
    Declined,
    #[default]
    Initialized,
}

impl From<MultisafepayPaymentStatus> for enums::AttemptStatus {
    fn from(item: MultisafepayPaymentStatus) -> Self {
        match item {
            MultisafepayPaymentStatus::Completed => Self::Charged,
            MultisafepayPaymentStatus::Declined => Self::Failure,
            MultisafepayPaymentStatus::Initialized => Self::AuthenticationPending,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Data {
    #[serde(rename = "type")]
    pub payment_type: Option<String>,
    pub order_id: String,
    pub currency: Option<String>,
    pub amount: Option<i64>,
    pub description: Option<String>,
    pub capture: Option<String>,
    pub payment_url: Option<Url>,
    pub status: Option<MultisafepayPaymentStatus>,
    pub error_code: Option<i32>,
    pub error_info: Option<String>,
    pub payment_details: Option<MultisafepayPaymentDetails>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]

pub struct MultisafepayPaymentDetails {
    pub account_holder_name: Option<String>,
    pub account_id: Option<String>,
    pub card_expiry_date: Option<i32>,
    pub external_transaction_id: Option<serde_json::Value>,
    pub last4: Option<serde_json::Value>,
    pub recurring_flow: Option<String>,
    pub recurring_id: Option<String>,
    pub recurring_model: Option<String>,
    #[serde(rename = "type")]
    pub payment_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultisafepayPaymentsResponse {
    pub success: bool,
    pub data: Data,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, MultisafepayPaymentsResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            MultisafepayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .data
            .payment_url
            .clone()
            .map(|url| services::RedirectForm::from((url, services::Method::Get)));

        let default_status = if item.response.success {
            MultisafepayPaymentStatus::Initialized
        } else {
            MultisafepayPaymentStatus::Declined
        };

        let status = item.response.data.status.unwrap_or(default_status);

        Ok(Self {
            status: enums::AttemptStatus::from(status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.order_id),
                redirection_data,
                mandate_reference: item
                    .response
                    .data
                    .payment_details
                    .and_then(|payment_details| payment_details.recurring_id),
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct MultisafepayRefundRequest {
    pub currency: storage_models::enums::Currency,
    pub amount: i64,
    pub description: Option<String>,
    pub refund_order_id: Option<String>,
    pub checkout_data: Option<ShoppingCart>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for MultisafepayRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            currency: item.request.currency,
            amount: item.request.amount,
            description: item.description.clone(),
            refund_order_id: Some(item.request.refund_id.clone()),
            checkout_data: None,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    pub transaction_id: i64,
    pub refund_id: i64,
    pub order_id: Option<String>,
    pub error_code: Option<i32>,
    pub error_info: Option<String>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub success: bool,
    pub data: RefundData,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_stat = if item.response.success {
            RefundStatus::Succeeded
        } else {
            RefundStatus::Failed
        };

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.data.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(refund_stat),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = if item.response.success {
            RefundStatus::Succeeded
        } else {
            RefundStatus::Failed
        };

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.data.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultisafepayErrorResponse {
    pub success: bool,
    pub error_code: i32,
    pub error_info: String,
}
