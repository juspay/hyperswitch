use common_enums::{enums, FraudCheckStatus};
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    address::Address,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        fraud_check::{Checkout, Sale, Transaction},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        fraud_check::{FraudCheckCheckoutData, FraudCheckSaleData, FraudCheckTransactionData},
        ResponseId,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        FrmCheckoutRouterData, FrmSaleRouterData, FrmTransactionRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{self, FraudCheckCheckoutRequest, FraudCheckSaleRequest, RouterData as _},
};

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SiftFraudCheckRequest {
    #[serde(rename = "$user_id")]
    user_id: String,
    #[serde(rename = "$session_id")]
    session_id: String,
    #[serde(rename = "$order_id")]
    order_id: String,
    #[serde(rename = "$user_email")]
    user_email: Secret<String>,
    #[serde(rename = "$amount")]
    amount: u64,
    #[serde(rename = "$currency_code")]
    currency_code: String,
    #[serde(rename = "$billing_address")]
    billing_address: SiftAddress,
    #[serde(rename = "$payment_methods")]
    payment_methods: Vec<PaymentMethods>,
    #[serde(rename = "$ordered_from")]
    ordered_from: OrderedFrom,
    #[serde(rename = "$shipping_address")]
    shipping_address: SiftAddress,
    #[serde(rename = "$site_country")]
    site_country: String,
    #[serde(rename = "$site_domain")]
    site_domain: String,
    #[serde(rename = "$items")]
    items: Vec<Item>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaymentMethods {
    #[serde(rename = "$payment_type")]
    payment_type: String,
    #[serde(rename = "$payment_gateway")]
    payment_gateway: String,
    #[serde(rename = "$card_bin")]
    card_bin: Secret<String>,
    #[serde(rename = "$card_last4")]
    card_last4: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct OrderedFrom {
    #[serde(rename = "$browser")]
    browser: Browser,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Browser {
    #[serde(rename = "$user_agent")]
    user_agent: String,
    #[serde(rename = "$accept_language")]
    accept_language: String,
    #[serde(rename = "$content_language")]
    content_language: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Item {
    #[serde(rename = "$item_id")]
    item_id: String,
    #[serde(rename = "$product_title")]
    product_title: String,
    #[serde(rename = "$price")]
    price: u64,
    #[serde(rename = "$currency_code")]
    currency_code: String,
    #[serde(rename = "$quantity")]
    quantity: i64,
    #[serde(rename = "$sku")]
    sku: String,
    #[serde(rename = "$brand")]
    brand: String,
    #[serde(rename = "$manufacturer")]
    manufacturer: String,
    #[serde(rename = "$category")]
    category: String,
    #[serde(rename = "$tags")]
    tags: Vec<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SiftAddress {
    #[serde(rename = "$name")]
    name: Secret<String>,
    #[serde(rename = "$phone")]
    phone: Secret<String>,
    #[serde(rename = "$address_1")]
    address_1: Secret<String>,
    #[serde(rename = "$address_2")]
    address_2: Secret<String>,
    #[serde(rename = "$city")]
    city: String,
    #[serde(rename = "$region")]
    region: Secret<String>,
    #[serde(rename = "$country")]
    country: String,
    #[serde(rename = "$zipcode")]
    zipcode: Secret<String>,
}

impl TryFrom<&Address> for SiftAddress {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &Address) -> Result<Self, Self::Error> {
        let address = value.address.as_ref();
        Ok(Self {
            name: address
                .and_then(|address| address.get_optional_full_name())
                .unwrap_or_default(),
            phone: value
                .phone
                .as_ref()
                .and_then(|phone| phone.number.clone())
                .unwrap_or_else(|| Secret::new("".to_string())),
            address_1: address
                .and_then(|address| address.line1.as_ref())
                .map_or(Secret::new("".to_string()), |line1| line1.to_owned()),
            address_2: address
                .and_then(|address| address.line2.as_ref())
                .map_or(Secret::new("".to_string()), |line2| line2.to_owned()),
            city: address
                .and_then(|address| address.city.as_ref())
                .map_or("".to_string(), |city| city.to_owned()),
            region: address
                .and_then(|address| address.state.as_ref())
                .map_or(Secret::new("".to_string()), |state| state.to_owned()),
            country: address
                .and_then(|address| address.country.as_ref())
                .map_or("".to_string(), |country| country.to_string()),
            zipcode: address
                .and_then(|address| address.zip.as_ref())
                .map_or(Secret::new("".to_string()), |zip| zip.to_owned()),
        })
    }
}

// impl TryFrom<&FrmCheckoutRouterData> for SiftFraudCheckRequest {
//     type Error = error_stack::Report<ConnectorError>;
//     fn try_from(item: &FrmCheckoutRouterData) -> Result<Self, Self::Error> {
//         let products = item
//             .request
//             .get_order_details()?
//             .iter()
//             .map(|order_detail| Products {
//                 item_name: order_detail.product_name.clone(),
//                 item_price: order_detail.amount.get_amount_as_i64(), // This should be changed to MinorUnit when we implement amount conversion for this connector. Additionally, the function get_amount_as_i64() should be avoided in the future.
//                 item_quantity: i32::from(order_detail.quantity),
//                 item_id: order_detail.product_id.clone(),
//                 item_category: order_detail.category.clone(),
//                 item_sub_category: order_detail.sub_category.clone(),
//                 item_is_digital: order_detail
//                     .product_type
//                     .as_ref()
//                     .map(|product| (product == &common_enums::ProductType::Digital)),
//             })
//             .collect::<Vec<_>>();
//         let metadata: SignifydFrmMetadata = item
//             .frm_metadata
//             .clone()
//             .ok_or(ConnectorError::MissingRequiredField {
//                 field_name: "frm_metadata",
//             })?
//             .parse_value("Signifyd Frm Metadata")
//             .change_context(ConnectorError::InvalidDataFormat {
//                 field_name: "frm_metadata",
//             })?;
//         let ship_address = item.get_shipping_address()?;
//         let street_addr = ship_address.get_line1()?;
//         let city_addr = ship_address.get_city()?;
//         let zip_code_addr = ship_address.get_zip()?;
//         let country_code_addr = ship_address.get_country()?;
//         let _first_name_addr = ship_address.get_first_name()?;
//         let _last_name_addr = ship_address.get_last_name()?;
//         let billing_address = item.get_billing()?;
//         let address: Address = Address {
//             street_address: street_addr.clone(),
//             unit: None,
//             postal_code: zip_code_addr.clone(),
//             city: city_addr.clone(),
//             province_code: zip_code_addr.clone(),
//             country_code: country_code_addr.to_owned(),
//         };
//         let destination: Destination = Destination {
//             full_name: ship_address.get_full_name().unwrap_or_default(),
//             organization: None,
//             email: None,
//             address,
//         };
//         let created_at = common_utils::date_time::now();
//         let order_channel = metadata.order_channel;
//         let shipments: Shipments = Shipments {
//             destination,
//             fulfillment_method: metadata.fulfillment_method,
//         };
//         let purchase = Purchase {
//             created_at,
//             order_channel,
//             total_price: item.request.amount,
//             products,
//             shipments,
//             currency: item.request.currency,
//             total_shipping_cost: metadata.total_shipping_cost,
//             confirmation_email: item.request.email.clone(),
//             confirmation_phone: billing_address
//                 .clone()
//                 .phone
//                 .and_then(|phone_data| phone_data.number),
//         };
//         Ok(Self {
//             checkout_id: item.payment_id.clone(),
//             order_id: item.attempt_id.clone(),
//             purchase,
//             coverage_requests: metadata.coverage_request,
//         })
//     }
// }

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct SiftAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SiftAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiftFraudCheckResponse {
    status: i64,
    error_message: String,
    scores: Scores,
    entity_type: String,
    entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scores {
    payment_abuse: PaymentAbuse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAbuse {
    status: i64,
    error_message: String,
    time: i64,
    score: f64,
    reasons: Vec<Reasons>,
    percentiles: Percentiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reasons {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentiles {
    last_7_days: f64,
    last_1_days: f64,
    last_10_days: f64,
    last_5_days: f64,
}

// impl From<f64> for FraudCheckStatus {
//     fn from(score: f64) -> Self {
//         match score {
//             s if s > 0.5 => Self::Fraud,
//             s if s > 0.3 => Self::ManualReview,
//             _ => Self::Legit,
//         }
//     }
// }

pub struct SiftScore(pub f64);

impl From<SiftScore> for FraudCheckStatus {
    fn from(score: SiftScore) -> Self {
        match score.0 {
            s if s > 0.5 => Self::Fraud,
            s if s > 0.3 => Self::ManualReview,
            _ => Self::Legit,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SiftFraudCheckResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, SiftFraudCheckResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        let score = item.response.scores.payment_abuse.score;

        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.entity_id),
                status: FraudCheckStatus::from(SiftScore(score)),
                connector_metadata: None,
                score: Some(score as i32),
                reason: Some(serde_json::json!({
                    "error_message": item.response.error_message,
                    "reasons": item.response.scores.payment_abuse.reasons,
                })),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SiftErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

// impl TryFrom<&FrmSaleRouterData> for SiftFraudCheckRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &FrmSaleRouterData) -> Result<Self, Self::Error> {
//         let request = &item.request;
//         let currency_code = request.currency.to_string();
//         let amount = item.get_amount_as_u64()?;
//         let shipping_address = item
//             .get_optional_shipping()
//             .map(SiftAddress::try_from)
//             .transpose()?
//             .unwrap_or_default();
//         Ok(Self {
//             user_id: request.customer_id.clone().unwrap_or_default(),
//             session_id: item.attempt_id.clone(),
//             order_id: item.payment_id.to_string(),
//             user_email: request
//                 .email
//                 .clone()
//                 .unwrap_or_default()
//                 .expose()
//                 .to_string()
//                 .into(),
//             amount,
//             currency_code: currency_code.to_string(),
//             billing_address: SiftAddress::try_from(item.get_billing()?)?,
//             payment_methods: vec![],
//             ordered_from: OrderedFrom {
//                 browser: Browser {
//                     user_agent: "".to_string(),
//                     accept_language: "".to_string(),
//                     content_language: "".to_string(),
//                 },
//             },
//             shipping_address,
//             site_country: "".to_string(),
//             site_domain: "".to_string(),
//             items: request
//                 .get_order_details()?
//                 .iter()
//                 .map(|order| {
//                     Ok(Item {
//                         item_id: order.product_id.clone().unwrap_or_default(),
//                         product_title: order.product_name.clone(),
//                         price: order.amount.get_amount_as_u64()?,
//                         currency_code: currency_code.to_string(),
//                         quantity: order.quantity.into(),
//                         sku: order.product_id.clone().unwrap_or_default(),
//                         brand: "".to_string(),
//                         manufacturer: "".to_string(),
//                         category: order.category.clone().unwrap_or_default(),
//                         tags: vec![],
//                     })
//                 })
//                 .collect::<Result<Vec<_>, _>>()?,
//         })
//     }
// }

// impl TryFrom<&FrmTransactionRouterData> for SiftFraudCheckRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &FrmTransactionRouterData) -> Result<Self, Self::Error> {
//         let request = &item.request;
//         let currency_code = request.currency?.to_string();
//         let amount = item.get_amount_as_u64()?;
//         let shipping_address = item
//             .get_optional_shipping()
//             .map(SiftAddress::try_from)
//             .transpose()?
//             .unwrap_or_default();
//         Ok(Self {
//             user_id: request.customer_id.clone().unwrap_or_default(),
//             session_id: item.attempt_id.clone(),
//             order_id: item.payment_id.to_string(),
//             user_email: item
//                 .get_billing()?
//                 .email
//                 .clone()
//                 .unwrap_or_default()
//                 .expose()
//                 .to_string()
//                 .into(),
//             amount,
//             currency_code,
//             billing_address: SiftAddress::try_from(item.get_billing()?)?,
//             payment_methods: vec![],
//             ordered_from: OrderedFrom {
//                 browser: Browser {
//                     user_agent: "".to_string(),
//                     accept_language: "".to_string(),
//                     content_language: "".to_string(),
//                 },
//             },
//             shipping_address,
//             site_country: "".to_string(),
//             site_domain: "".to_string(),
//             items: vec![],
//         })
//     }
// }
