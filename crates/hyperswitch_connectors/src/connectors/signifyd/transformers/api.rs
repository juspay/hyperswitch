use api_models::webhooks::IncomingWebhookEvent;
use common_enums::{AttemptStatus, Currency, FraudCheckStatus, PaymentMethod};
use common_utils::{ext_traits::ValueExt, pii::Email};
use error_stack::{self, ResultExt};
pub use hyperswitch_domain_models::router_request_types::fraud_check::RefundMethod;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::Fulfillment,
    router_request_types::{
        fraud_check::{self, FraudCheckFulfillmentData, FrmFulfillmentRequest},
        ResponseId,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{
    types::{
        FrmCheckoutRouterData, FrmFulfillmentRouterData, FrmRecordReturnRouterData,
        FrmSaleRouterData, FrmTransactionRouterData, ResponseRouterData,
    },
    utils::{
        AddressDetailsData as _, FraudCheckCheckoutRequest, FraudCheckRecordReturnRequest as _,
        FraudCheckSaleRequest as _, FraudCheckTransactionRequest as _, RouterData as _,
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
    currency: Option<Currency>,
    total_shipping_cost: Option<i64>,
    confirmation_email: Option<Email>,
    confirmation_phone: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
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

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
pub enum FulfillmentMethod {
    Delivery,
    CounterPickup,
    CubsidePickup,
    LockerPickup,
    StandardShipping,
    ExpeditedShipping,
    GasPickup,
    ScheduledDelivery,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Products {
    item_name: String,
    item_price: i64,
    item_quantity: i32,
    item_id: Option<String>,
    item_category: Option<String>,
    item_sub_category: Option<String>,
    item_is_digital: Option<bool>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Shipments {
    destination: Destination,
    fulfillment_method: Option<FulfillmentMethod>,
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

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
pub enum CoverageRequests {
    Fraud, // use when you need a financial guarantee for Payment Fraud.
    Inr,   // use when you need a financial guarantee for Item Not Received.
    Snad, // use when you need a financial guarantee for fraud alleging items are Significantly Not As Described.
    All,  // use when you need a financial guarantee on all chargebacks.
    None, // use when you do not need a financial guarantee. Suggested actions in decision.checkpointAction are recommendations.
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsSaleRequest {
    order_id: String,
    purchase: Purchase,
    decision_delivery: DecisionDelivery,
    coverage_requests: Option<CoverageRequests>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SignifydFrmMetadata {
    pub total_shipping_cost: Option<i64>,
    pub fulfillment_method: Option<FulfillmentMethod>,
    pub coverage_request: Option<CoverageRequests>,
    pub order_channel: OrderChannel,
}

impl TryFrom<&FrmSaleRouterData> for SignifydPaymentsSaleRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmSaleRouterData) -> Result<Self, Self::Error> {
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| Products {
                item_name: order_detail.product_name.clone(),
                item_price: order_detail.amount.get_amount_as_i64(), // This should be changed to MinorUnit when we implement amount conversion for this connector. Additionally, the function get_amount_as_i64() should be avoided in the future.
                item_quantity: i32::from(order_detail.quantity),
                item_id: order_detail.product_id.clone(),
                item_category: order_detail.category.clone(),
                item_sub_category: order_detail.sub_category.clone(),
                item_is_digital: order_detail
                    .product_type
                    .as_ref()
                    .map(|product| product == &common_enums::ProductType::Digital),
            })
            .collect::<Vec<_>>();
        let metadata: SignifydFrmMetadata = item
            .frm_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "frm_metadata",
            })?
            .parse_value("Signifyd Frm Metadata")
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "frm_metadata",
            })?;
        let ship_address = item.get_shipping_address()?;
        let billing_address = item.get_billing()?;
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
        let order_channel = metadata.order_channel;
        let shipments = Shipments {
            destination,
            fulfillment_method: metadata.fulfillment_method,
        };
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price: item.request.amount,
            products,
            shipments,
            currency: item.request.currency,
            total_shipping_cost: metadata.total_shipping_cost,
            confirmation_email: item.request.email.clone(),
            confirmation_phone: billing_address
                .clone()
                .phone
                .and_then(|phone_data| phone_data.number),
        };
        Ok(Self {
            order_id: item.attempt_id.clone(),
            purchase,
            decision_delivery: DecisionDelivery::Sync, // Specify SYNC if you require the Response to contain a decision field. If you have registered for a webhook associated with this checkpoint, then the webhook will also be sent when SYNC is specified. If ASYNC_ONLY is specified, then the decision field in the response will be null, and you will require a Webhook integration to receive Signifyd's final decision
            coverage_requests: metadata.coverage_request,
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

impl From<SignifydPaymentStatus> for FraudCheckStatus {
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

impl<F, T> TryFrom<ResponseRouterData<F, SignifydPaymentsResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SignifydPaymentsResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                status: FraudCheckStatus::from(item.response.decision.checkpoint_action),
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
    payment_method: PaymentMethod,
    amount: i64,
    currency: Currency,
    gateway: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsTransactionRequest {
    order_id: String,
    checkout_id: String,
    transactions: Transactions,
}

impl From<AttemptStatus> for GatewayStatusCode {
    fn from(item: AttemptStatus) -> Self {
        match item {
            AttemptStatus::Pending => Self::Pending,
            AttemptStatus::Failure => Self::Failure,
            AttemptStatus::Charged => Self::Success,
            _ => Self::Pending,
        }
    }
}

impl TryFrom<&FrmTransactionRouterData> for SignifydPaymentsTransactionRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmTransactionRouterData) -> Result<Self, Self::Error> {
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
    coverage_requests: Option<CoverageRequests>,
}

impl TryFrom<&FrmCheckoutRouterData> for SignifydPaymentsCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmCheckoutRouterData) -> Result<Self, Self::Error> {
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| Products {
                item_name: order_detail.product_name.clone(),
                item_price: order_detail.amount.get_amount_as_i64(), // This should be changed to MinorUnit when we implement amount conversion for this connector. Additionally, the function get_amount_as_i64() should be avoided in the future.
                item_quantity: i32::from(order_detail.quantity),
                item_id: order_detail.product_id.clone(),
                item_category: order_detail.category.clone(),
                item_sub_category: order_detail.sub_category.clone(),
                item_is_digital: order_detail
                    .product_type
                    .as_ref()
                    .map(|product| product == &common_enums::ProductType::Digital),
            })
            .collect::<Vec<_>>();
        let metadata: SignifydFrmMetadata = item
            .frm_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "frm_metadata",
            })?
            .parse_value("Signifyd Frm Metadata")
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "frm_metadata",
            })?;
        let ship_address = item.get_shipping_address()?;
        let street_addr = ship_address.get_line1()?;
        let city_addr = ship_address.get_city()?;
        let zip_code_addr = ship_address.get_zip()?;
        let country_code_addr = ship_address.get_country()?;
        let _first_name_addr = ship_address.get_first_name()?;
        let _last_name_addr = ship_address.get_last_name()?;
        let billing_address = item.get_billing()?;
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
        let order_channel = metadata.order_channel;
        let shipments: Shipments = Shipments {
            destination,
            fulfillment_method: metadata.fulfillment_method,
        };
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price: item.request.amount,
            products,
            shipments,
            currency: item.request.currency,
            total_shipping_cost: metadata.total_shipping_cost,
            confirmation_email: item.request.email.clone(),
            confirmation_phone: billing_address
                .clone()
                .phone
                .and_then(|phone_data| phone_data.number),
        };
        Ok(Self {
            checkout_id: item.payment_id.clone(),
            order_id: item.attempt_id.clone(),
            purchase,
            coverage_requests: metadata.coverage_request,
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

impl TryFrom<&FrmFulfillmentRouterData> for FrmFulfillmentSignifydRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmFulfillmentRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            order_id: item.request.fulfillment_req.order_id.clone(),
            fulfillment_status: item
                .request
                .fulfillment_req
                .fulfillment_status
                .as_ref()
                .map(|fulfillment_status| FulfillmentStatus::from(&fulfillment_status.clone())),
            fulfillments: get_signifyd_fulfillments_from_frm_fulfillment_request(
                &item.request.fulfillment_req,
            ),
        })
    }
}

impl From<&fraud_check::FulfillmentStatus> for FulfillmentStatus {
    fn from(status: &fraud_check::FulfillmentStatus) -> Self {
        match status {
            fraud_check::FulfillmentStatus::PARTIAL => Self::PARTIAL,
            fraud_check::FulfillmentStatus::COMPLETE => Self::COMPLETE,
            fraud_check::FulfillmentStatus::REPLACEMENT => Self::REPLACEMENT,
            fraud_check::FulfillmentStatus::CANCELED => Self::CANCELED,
        }
    }
}
pub(crate) fn get_signifyd_fulfillments_from_frm_fulfillment_request(
    fulfillment_req: &FrmFulfillmentRequest,
) -> Vec<Fulfillments> {
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

impl From<fraud_check::Product> for Product {
    fn from(product: fraud_check::Product) -> Self {
        Self {
            item_name: product.item_name,
            item_quantity: product.item_quantity,
            item_id: product.item_id,
        }
    }
}

impl From<fraud_check::Destination> for Destination {
    fn from(destination: fraud_check::Destination) -> Self {
        Self {
            full_name: destination.full_name,
            organization: destination.organization,
            email: destination.email,
            address: Address::from(destination.address),
        }
    }
}

impl From<fraud_check::Address> for Address {
    fn from(address: fraud_check::Address) -> Self {
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
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    > for RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Fulfillment,
            FrmFulfillmentSignifydApiResponse,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::FulfillmentResponse {
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
    currency: Currency,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsRecordReturnResponse {
    return_id: String,
    order_id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, SignifydPaymentsRecordReturnResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            SignifydPaymentsRecordReturnResponse,
            T,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::RecordReturnResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                return_id: Some(item.response.return_id.to_string()),
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&FrmRecordReturnRouterData> for SignifydPaymentsRecordReturnRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmRecordReturnRouterData) -> Result<Self, Self::Error> {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignifydWebhookBody {
    pub order_id: String,
    pub review_disposition: ReviewDisposition,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewDisposition {
    Fraudulent,
    Good,
}

impl From<ReviewDisposition> for IncomingWebhookEvent {
    fn from(value: ReviewDisposition) -> Self {
        match value {
            ReviewDisposition::Fraudulent => Self::FrmRejected,
            ReviewDisposition::Good => Self::FrmApproved,
        }
    }
}
