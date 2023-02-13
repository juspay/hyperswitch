use std::fmt::format;

use api_models::payments::Card;
use common_utils::pii::Email;
use error_stack::ResultExt;
use masking::{PeekInterface, ExposeInterface};
use regex::Error;
use reqwest::redirect::Attempt;
use serde::{Deserialize, Serialize};
use storage_models::schema::address::first_name;
use url::Url;
use uuid::Uuid;
use crate::{core::errors,pii::{self, Secret},services,types::{self,api, storage::enums}, utils::OptionExt};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Gateway {
    Amex,
    Creditcard,
    Maestro,
    Mastercard,
    Visa,
    Klarna,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Coupons {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Mistercash {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile_pay_button_position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_mobile_pay_button: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_only: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_size: Option<String>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Gateways {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mistercash: Option<Mistercash>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupons: Option<Coupons>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateways: Option<Gateways>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_method: Option<String>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_window: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // pub template: Option<Template>,
    pub allowed_countries: Option<Vec<String>>

}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Browser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_color_depth: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Customer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<Browser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<masking::Secret<String, Email>>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GatewayInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_holder_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_expiry_date: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_cvc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flexible_3d: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeliveryObject {
    first_name: String,
    last_name: String,
    address1: String,
    house_number: String,
    zip_code: String,
    city: String,
    country: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Item {
    pub name: String, 
    pub unit_price: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub quantity: i64,
} 

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ShoppingCart {
    pub items: Vec<Item>
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DefaultObject {
    shipping_taxed: bool,
    rate: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TaxObject {
    pub default: DefaultObject
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CheckoutOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_cart: Option<bool>,
    pub tax_tables: TaxObject
}

//TODO: Fill the struct with respective fields
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MultisafepayPaymentsRequest {
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
    pub gateway: Gateway,
    pub order_id: String,
    pub currency: String,
    pub amount:i64,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_options: Option<PaymentOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Customer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_info: Option<GatewayInfo>,
    pub delivery: Option<DeliveryObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_options: Option<CheckoutOptions>,
    pub shopping_cart: Option<ShoppingCart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<String>,
    // pub affiliate: Option<Affiliate>,
    // pub second_chance: Option<SecondChance>,
    // pub customer_verification: Option<CustomerVerification>,
    // pub custom_info: Option<CustomInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_active: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds_active: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var3: Option<String>, 
    // pub plugin: Option<Plugin>
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for MultisafepayPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let payment_options = Some(PaymentOptions {
            notification_url: None,
            redirect_url: _item.return_url.clone(),
            cancel_url: None, 
            close_window: None,
            notification_method: None,
            settings: None,
            template_id: None,
            allowed_countries: None
        });

        let customer = Some(Customer {
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
            email: _item.request.email.clone(),
            user_agent: None,
            referrer: None,
            reference: None
        });

        let description = match _item.description.clone() {
            Some(desc) => desc,
            None => String::from("Default Description"),
        };
        
        let defaultDelivery = DeliveryObject {
            first_name: String::from("default"),
            last_name: String::from("default"),
            address1: String::from("default"),
            house_number: String::from("default"),
            zip_code: String::from("default"),
            city: String::from("default"),
            country: String::from("default")
        };

        let delivery = match _item.address.billing.clone() {
            Some(addr) => match addr.address {
                Some(addrs) => DeliveryObject {
                    first_name: addrs.first_name.unwrap_or(Secret::new("default".to_string())).expose(),
                    last_name: addrs.last_name.unwrap_or(Secret::new("default".to_string())).expose(),
                    address1: addrs.line1.unwrap_or(Secret::new("default".to_string())).expose(),
                    house_number: addrs.line2.unwrap_or(Secret::new("default".to_string())).expose(),
                    zip_code: addrs.zip.unwrap_or(Secret::new("default".to_string())).expose(),
                    city: addrs.city.unwrap_or("default".to_string()),
                    country: addrs.country.unwrap_or("default".to_string()),
                },
                None => defaultDelivery
            },
            None => defaultDelivery
        };

        let gateway_info = match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                GatewayInfo {
                    card_number: Some(ccard.card_number.clone().expose()),
                    card_expiry_date: Some((format!("{}{}", ccard.card_exp_year.clone().expose(), ccard.card_exp_month.clone().expose())).parse::<i32>().unwrap()),
                    card_cvc: Some(ccard.card_cvc.clone().expose()),
                    card_holder_name: None,
                    flexible_3d: None,
                    moto: None,
                    term_url: None,
                    email: None,
                }
            },
            api::PaymentMethod::PayLater(ref paylater) => {
                GatewayInfo {
                    card_number: None,
                    card_expiry_date: None,
                    card_cvc: None,
                    card_holder_name: None,
                    flexible_3d: None,
                    moto: None,
                    term_url: None,
                    email: Some(match paylater {
                        api_models::payments::PayLaterData::KlarnaRedirect { 
                            issuer_name, 
                            billing_email, 
                            billing_country 
                        } => billing_email.clone(),
                        _ => Err(errors::ConnectorError::NotImplemented(
                            "Only KlarnaRedirect is implemented".to_string(),
                        ))?
                    }),
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not Implemented".to_string(),
            ))?,
        };

        let _type = match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => String::from("direct"),
            api::PaymentMethod::PayLater(ref paylater) => String::from("redirect"),
            _ => String::from("redirect"),
        };

        let gateway = match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Gateway::Amex,
            api::PaymentMethod::PayLater(ref paylater) => Gateway::Klarna,
            _ => Gateway::Amex,
        };

        println!("Printing Gatwaya ---> {:?}",gateway_info);
        Ok(Self {
            _type: Some(_type),
            gateway,
            order_id: _item.payment_id.to_string(),
            currency: _item.request.currency.to_string(),
            amount: _item.request.amount.clone(),
            description: description.clone(),
            payment_options,
            customer,
            delivery: Some(delivery),
            gateway_info: Some(gateway_info),
            checkout_options: Some(CheckoutOptions { 
                validate_cart: Some(false),
                tax_tables: TaxObject {
                    default: DefaultObject {
                        shipping_taxed: false,
                        rate: 0.0,
                    }
                }
            }),
            shopping_cart: Some(ShoppingCart {
                items: vec!(Item {
                    name: String::from("Item"),
                    unit_price: _item.request.amount.clone() / 100,
                    description: Some(description.clone()),
                    quantity: 1,
                })
            }),
            capture: None,
            items: None,
            recurring_model: None,
            recurring_id: None,
            days_active: Some(30),
            seconds_active: Some(259200),
            var1: None,
            var2: None,
            var3: None
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct MultisafepayAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for MultisafepayAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
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
            MultisafepayPaymentStatus::Initialized => Self::AuthenticationPending,
            MultisafepayPaymentStatus::Completed => Self::Charged,
            MultisafepayPaymentStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Data {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub order_id: Option<String>,
    pub currency: Option<String>,
    pub amount: Option<i64>,
    pub description: Option<String>,
    pub capture: Option<String>,
    pub payment_url: Option<Url>,
    pub status: Option<MultisafepayPaymentStatus>,
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultisafepayPaymentsResponse {
    pub success: bool,
    pub data: Data,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, MultisafepayPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, MultisafepayPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        println!("payment URL check {:?}", item.response.data.payment_url);
        println!("order_id check {:?}", item.response.data.order_id);
        
        let redirection_data = match item.response.data.payment_url.clone() {
            Some(url) => Some({
                    let mut base_url = url.clone();
                    base_url.set_query(None);
                    services::RedirectForm {
                        url: base_url.to_string(),
                        method: services::Method::Get,
                        form_fields: std::collections::HashMap::from_iter(
                            url
                                .query_pairs()
                                .map(|(k, v)| (k.to_string(), v.to_string())),
                        ),
                    }
                }),
            None => None,
        };
        
        let default_status = if (item.response.success) { MultisafepayPaymentStatus::Initialized } else { MultisafepayPaymentStatus::Declined };

        let status = item.response.data.status.unwrap_or(default_status);

        Ok(Self {
            status: enums::AttemptStatus::from(status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.order_id.unwrap_or(Uuid::new_v4().to_string())),
                redirection_data,
                redirect: item.response.data.payment_url.is_some(),
                mandate_reference: None,
                connector_metadata: None,
            }),
            amount_captured: Some(item.response.data.amount.unwrap_or(0)),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct MultisafepayRefundRequest {
    pub currency: String,
    pub amount: i64,
    pub description: Option<String>,
    pub refund_order_id: Option<String>,
    pub checkout_data: ShoppingCart,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for MultisafepayRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            currency: _item.request.currency.to_string(),
            amount: _item.request.amount,
            description: _item.description.clone(),
            refund_order_id: Some(Uuid::new_v4().to_string()),
            checkout_data: ShoppingCart {
                items: vec!(Item {
                    name: String::from("Item"),
                    unit_price: _item.request.amount.clone() / 100,
                    description: None,
                    quantity: 1,
                })
            }
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
            //TODO: Review mapping
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    pub transaction_id: i64,
    pub refund_id: i64,
    pub order_id: Option<String>,
}
//TODO: Fill the struct with respective fields
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
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_refund_id = match _item.response.data.order_id.clone() {
            Some(ord_id) => ord_id,
            None => _item.response.data.refund_id.to_string(),
        };
        let refund_stat = if (_item.response.success) { RefundStatus::Succeeded } else { RefundStatus::Failed };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id,
                refund_status: enums::RefundStatus::from(refund_stat),
            }),
            .._item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        let refund_stat = if (_item.response.success) { RefundStatus::Succeeded } else { RefundStatus::Failed };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: _item.response.data.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(refund_stat),
            }),
            .._item.data
        })
    }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultisafepayErrorResponse {}
