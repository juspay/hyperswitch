use bigdecimal::ToPrimitive;
use common_utils::pii::Email;
use error_stack;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{
    connector::utils::{
        AddressDetailsData, FraudCheckCheckoutRequest, FraudCheckRecordReturnRequest,
        FraudCheckSaleRequest, FraudCheckTransactionRequest, RouterData,
    },
    core::{errors, fraud_check::types as core_types},
    types::{
        self, api::Fulfillment, fraud_check as frm_types, storage::enums as storage_enums,
        ResponseId, ResponseRouterData,
    },
};

#[allow(dead_code)]
#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionDelivery {
    Sync,
    AsyncOnly,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    order_channel: OrderChannel,
    total_price: i64,
    products: Vec<Products>,
    shipments: Shipments,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderChannel {
    Web,
    Phone,
    MobileApp,
    Social,
    Marketplace,
    InStoreKiosk,
    ScanAndGo,
    SmartTv,
    Mit,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Products {
    item_name: String,
    item_price: i64,
    item_quantity: i32,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct Shipments {
    destination: Destination,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    full_name: Secret<String>,
    organization: Option<String>,
    email: Option<Email>,
    address: Address,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    street_address: Secret<String>,
    unit: Option<Secret<String>>,
    postal_code: Secret<String>,
    city: String,
    province_code: Secret<String>,
    country_code: common_enums::CountryAlpha2,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsSaleRequest {
    order_id: String,
    purchase: Purchase,
    decision_delivery: DecisionDelivery,
}

impl TryFrom<&frm_types::FrmSaleRouterData> for SignifydPaymentsSaleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &frm_types::FrmSaleRouterData) -> Result<Self, Self::Error> {
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| Products {
                item_name: order_detail.product_name.clone(),
                item_price: order_detail.amount,
                item_quantity: i32::from(order_detail.quantity),
            })
            .collect::<Vec<_>>();
        let ship_address = item.get_shipping_address()?;
        let street_addr = ship_address.get_line1()?;
        let city_addr = ship_address.get_city()?;
        let zip_code_addr = ship_address.get_zip()?;
        let country_code_addr = ship_address.get_country()?;
        let _first_name_addr = ship_address.get_first_name()?;
        let _last_name_addr = ship_address.get_last_name()?;
        let address: Address = Address {
            street_address: street_addr.clone(),
            unit: None,
            postal_code: zip_code_addr.clone(),
            city: city_addr.clone(),
            province_code: zip_code_addr.clone(),
            country_code: country_code_addr.to_owned(),
        };
        let destination: Destination = Destination {
            full_name: ship_address.get_full_name().unwrap_or_default(),
            organization: None,
            email: None,
            address,
        };

        let created_at = common_utils::date_time::now();
        let order_channel = OrderChannel::Web;
        let shipments = Shipments { destination };
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price: item.request.amount,
            products,
            shipments,
        };
        Ok(Self {
            order_id: item.attempt_id.clone(),
            purchase,
            decision_delivery: DecisionDelivery::Sync,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    checkpoint_action: SignifydPaymentStatus,
    checkpoint_action_reason: Option<String>,
    checkpoint_action_policy: Option<String>,
    score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignifydPaymentStatus {
    Accept,
    Challenge,
    Credit,
    Hold,
    Reject,
}

impl From<SignifydPaymentStatus> for storage_enums::FraudCheckStatus {
    fn from(item: SignifydPaymentStatus) -> Self {
        match item {
            SignifydPaymentStatus::Accept => Self::Legit,
            SignifydPaymentStatus::Reject => Self::Fraud,
            SignifydPaymentStatus::Hold => Self::ManualReview,
            SignifydPaymentStatus::Challenge | SignifydPaymentStatus::Credit => Self::Pending,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsResponse {
    signifyd_id: i64,
    order_id: String,
    decision: Decision,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, SignifydPaymentsResponse, T, frm_types::FraudCheckResponseData>>
    for types::RouterData<F, T, frm_types::FraudCheckResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SignifydPaymentsResponse, T, frm_types::FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(frm_types::FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                status: storage_enums::FraudCheckStatus::from(
                    item.response.decision.checkpoint_action,
                ),
                connector_metadata: None,
                score: item.response.decision.score.and_then(|data| data.to_i32()),
                reason: item
                    .response
                    .decision
                    .checkpoint_action_reason
                    .map(serde_json::Value::from),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SignifydErrorResponse {
    pub messages: Vec<String>,
    pub errors: serde_json::Value,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transactions {
    transaction_id: String,
    gateway_status_code: String,
    payment_method: storage_enums::PaymentMethod,
    amount: i64,
    currency: storage_enums::Currency,
    gateway: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsTransactionRequest {
    order_id: String,
    checkout_id: String,
    transactions: Transactions,
}

impl From<storage_enums::AttemptStatus> for GatewayStatusCode {
    fn from(item: storage_enums::AttemptStatus) -> Self {
        match item {
            storage_enums::AttemptStatus::Pending => Self::Pending,
            storage_enums::AttemptStatus::Failure => Self::Failure,
            storage_enums::AttemptStatus::Charged => Self::Success,
            _ => Self::Pending,
        }
    }
}

impl TryFrom<&frm_types::FrmTransactionRouterData> for SignifydPaymentsTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &frm_types::FrmTransactionRouterData) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;
        let transactions = Transactions {
            amount: item.request.amount,
            transaction_id: item.clone().payment_id,
            gateway_status_code: GatewayStatusCode::from(item.status).to_string(),
            payment_method: item.payment_method,
            currency,
            gateway: item.request.connector.clone(),
        };
        Ok(Self {
            order_id: item.attempt_id.clone(),
            checkout_id: item.payment_id.clone(),
            transactions,
        })
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayStatusCode {
    Success,
    Failure,
    #[default]
    Pending,
    Error,
    Cancelled,
    Expired,
    SoftDecline,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsCheckoutRequest {
    checkout_id: String,
    order_id: String,
    purchase: Purchase,
}

impl TryFrom<&frm_types::FrmCheckoutRouterData> for SignifydPaymentsCheckoutRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &frm_types::FrmCheckoutRouterData) -> Result<Self, Self::Error> {
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| Products {
                item_name: order_detail.product_name.clone(),
                item_price: order_detail.amount,
                item_quantity: i32::from(order_detail.quantity),
            })
            .collect::<Vec<_>>();
        let ship_address = item.get_shipping_address()?;
        let street_addr = ship_address.get_line1()?;
        let city_addr = ship_address.get_city()?;
        let zip_code_addr = ship_address.get_zip()?;
        let country_code_addr = ship_address.get_country()?;
        let _first_name_addr = ship_address.get_first_name()?;
        let _last_name_addr = ship_address.get_last_name()?;
        let address: Address = Address {
            street_address: street_addr.clone(),
            unit: None,
            postal_code: zip_code_addr.clone(),
            city: city_addr.clone(),
            province_code: zip_code_addr.clone(),
            country_code: country_code_addr.to_owned(),
        };
        let destination: Destination = Destination {
            full_name: ship_address.get_full_name().unwrap_or_default(),
            organization: None,
            email: None,
            address,
        };
        let created_at = common_utils::date_time::now();
        let order_channel = OrderChannel::Web;
        let shipments: Shipments = Shipments { destination };
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price: item.request.amount,
            products,
            shipments,
        };
        Ok(Self {
            checkout_id: item.payment_id.clone(),
            order_id: item.attempt_id.clone(),
            purchase,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydRequest {
    pub order_id: String,
    pub fulfillment_status: Option<FulfillmentStatus>,
    pub fulfillments: Vec<Fulfillments>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub enum FulfillmentStatus {
    PARTIAL,
    COMPLETE,
    REPLACEMENT,
    CANCELED,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct Fulfillments {
    pub shipment_id: String,
    pub products: Option<Vec<Product>>,
    pub destination: Destination,
    pub fulfillment_method: Option<String>,
    pub carrier: Option<String>,
    pub shipment_status: Option<String>,
    pub tracking_urls: Option<Vec<String>>,
    pub tracking_numbers: Option<Vec<String>>,
    pub shipped_at: Option<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub item_name: String,
    pub item_quantity: i64,
    pub item_id: String,
}

impl TryFrom<&frm_types::FrmFulfillmentRouterData> for FrmFulfillmentSignifydRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &frm_types::FrmFulfillmentRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            order_id: item.request.fulfillment_req.order_id.clone(),
            fulfillment_status: item
                .request
                .fulfillment_req
                .fulfillment_status
                .as_ref()
                .map(|fulfillment_status| FulfillmentStatus::from(&fulfillment_status.clone())),
            fulfillments: Vec::<Fulfillments>::from(&item.request.fulfillment_req),
        })
    }
}

impl From<&core_types::FulfillmentStatus> for FulfillmentStatus {
    fn from(status: &core_types::FulfillmentStatus) -> Self {
        match status {
            core_types::FulfillmentStatus::PARTIAL => Self::PARTIAL,
            core_types::FulfillmentStatus::COMPLETE => Self::COMPLETE,
            core_types::FulfillmentStatus::REPLACEMENT => Self::REPLACEMENT,
            core_types::FulfillmentStatus::CANCELED => Self::CANCELED,
        }
    }
}

impl From<&core_types::FrmFulfillmentRequest> for Vec<Fulfillments> {
    fn from(fulfillment_req: &core_types::FrmFulfillmentRequest) -> Self {
        fulfillment_req
            .fulfillments
            .iter()
            .map(|fulfillment| Fulfillments {
                shipment_id: fulfillment.shipment_id.clone(),
                products: fulfillment
                    .products
                    .as_ref()
                    .map(|products| products.iter().map(|p| Product::from(p.clone())).collect()),
                destination: Destination::from(fulfillment.destination.clone()),
                tracking_urls: fulfillment_req.tracking_urls.clone(),
                tracking_numbers: fulfillment_req.tracking_numbers.clone(),
                fulfillment_method: fulfillment_req.fulfillment_method.clone(),
                carrier: fulfillment_req.carrier.clone(),
                shipment_status: fulfillment_req.shipment_status.clone(),
                shipped_at: fulfillment_req.shipped_at.clone(),
            })
            .collect()
    }
}

impl From<core_types::Product> for Product {
    fn from(product: core_types::Product) -> Self {
        Self {
            item_name: product.item_name,
            item_quantity: product.item_quantity,
            item_id: product.item_id,
        }
    }
}

impl From<core_types::Destination> for Destination {
    fn from(destination: core_types::Destination) -> Self {
        Self {
            full_name: destination.full_name,
            organization: destination.organization,
            email: destination.email,
            address: Address::from(destination.address),
        }
    }
}

impl From<core_types::Address> for Address {
    fn from(address: core_types::Address) -> Self {
        Self {
            street_address: address.street_address,
            unit: address.unit,
            postal_code: address.postal_code,
            city: address.city,
            province_code: address.province_code,
            country_code: address.country_code,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiResponse {
    pub order_id: String,
    pub shipment_ids: Vec<String>,
}

impl
    TryFrom<
        ResponseRouterData<
            Fulfillment,
            FrmFulfillmentSignifydApiResponse,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        >,
    >
    for types::RouterData<
        Fulfillment,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Fulfillment,
            FrmFulfillmentSignifydApiResponse,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(frm_types::FraudCheckResponseData::FulfillmentResponse {
                order_id: item.response.order_id,
                shipment_ids: item.response.shipment_ids,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydRefund {
    method: RefundMethod,
    amount: String,
    currency: storage_enums::Currency,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsRecordReturnRequest {
    order_id: String,
    return_id: String,
    refund_transaction_id: Option<String>,
    refund: SignifydRefund,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundMethod {
    StoreCredit,
    OriginalPaymentInstrument,
    NewPaymentInstrument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsRecordReturnResponse {
    return_id: String,
    order_id: String,
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            SignifydPaymentsRecordReturnResponse,
            T,
            frm_types::FraudCheckResponseData,
        >,
    > for types::RouterData<F, T, frm_types::FraudCheckResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            SignifydPaymentsRecordReturnResponse,
            T,
            frm_types::FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(frm_types::FraudCheckResponseData::RecordReturnResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                return_id: Some(item.response.return_id.to_string()),
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&frm_types::FrmRecordReturnRouterData> for SignifydPaymentsRecordReturnRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &frm_types::FrmRecordReturnRouterData) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;
        let refund = SignifydRefund {
            method: item.request.refund_method.clone(),
            amount: item.request.amount.to_string(),
            currency,
        };
        Ok(Self {
            return_id: uuid::Uuid::new_v4().to_string(),
            refund_transaction_id: item.request.refund_transaction_id.clone(),
            refund,
            order_id: item.attempt_id.clone(),
        })
    }
}
