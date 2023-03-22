use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, RouterData},
    core::errors,
    pii::Secret,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalPrice {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    pub customer_id: String,
    pub customer_name: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CoinbasePaymentsRequest {
    pub name: Secret<String>,
    pub description: String,
    pub pricing_type: String,
    pub local_price: LocalPrice,
    pub metadata: Metadata,
    pub redirect_url: String,
    pub cancel_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CoinbasePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        get_crypto_specific_payment_data(_item)
        // Err(errors::ConnectorError::NotImplemented(
        //     "try_from PaymentsAuthorizeRouterData".to_string(),
        // )
        // .into())
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct CoinbaseAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for CoinbaseAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
        // Err(errors::ConnectorError::NotImplemented("try_from ConnectorAuthType".to_string()).into())
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoinbasePaymentStatus {
    #[serde(rename = "NEW")]
    New,
    #[default]
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "COMPLETED")]
    Completed,
    #[serde(rename = "EXPIRED")]
    Expired,
    #[serde(rename = "UNRESOLVED")]
    Unresolved,
    #[serde(rename = "RESOLVED")]
    Resolved,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "PENDING REFUND")]
    PendingRefund,
    #[serde(rename = "REFUNDED")]
    Refunded,
}

impl From<CoinbasePaymentStatus> for enums::AttemptStatus {
    fn from(item: CoinbasePaymentStatus) -> Self {
        match item {
            CoinbasePaymentStatus::New => Self::Charged,
            CoinbasePaymentStatus::Pending => Self::Authorizing,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    status: CoinbasePaymentStatus,
    time: String,
    pub payment: Option<TimelinePayment>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoinbasePaymentResponseData {
    id: String,
    hosted_url: String,
    timeline: Vec<Timeline>,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoinbasePaymentsResponse {
    // status: CoinbasePaymentStatus,
    // id: String,
    data: CoinbasePaymentResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, CoinbasePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CoinbasePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let redirection_data = services::RedirectForm {
            endpoint: item.response.data.hosted_url.to_string(),
            method: services::Method::Get,
            form_fields,
        };
        // let len = item.response.data.timeline.clone().len();
        // let my_status = item.response.data.timeline[len - 1].status;
        Ok(Self {
            // my_status,
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.id),
                redirection_data: Some(redirection_data),
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CoinbaseRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CoinbaseRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from RefundsRouterData".to_string()).into())
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
pub struct CoinbaseConnectorMeta {
    pub pricing_type: String,
}

fn get_crypto_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CoinbasePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let billing_address = item
        .get_billing()?
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    let name = billing_address.get_first_name()?.to_owned();
    let description = item.get_description()?;
    let connector_meta: CoinbaseConnectorMeta =
        utils::to_connector_meta_from_secret(item.connector_meta_data.clone())?;
    let pricing_type = connector_meta.pricing_type;
    let local_price = get_local_price(item);
    let metadata = get_metadata(item);
    let redirect_url = item.return_url.as_ref().unwrap().to_string();
    let cancel_url = item.return_url.as_ref().unwrap().to_string();

    Ok(CoinbasePaymentsRequest {
        name,
        description,
        pricing_type,
        local_price,
        metadata,
        redirect_url,
        cancel_url,
    })
}

fn get_local_price(item: &types::PaymentsAuthorizeRouterData) -> LocalPrice {
    LocalPrice {
        amount: format!("{:?}", item.request.amount),
        currency: item.request.currency.to_string(),
    }
}

fn get_metadata(_item: &types::PaymentsAuthorizeRouterData) -> Metadata {
    Metadata {
        customer_id: "112".to_string(),
        customer_name: "John".to_string(),
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseWebhookDetails {
    pub attempt_number: i64,
    pub event: Event,
    pub id: String,
    pub scheduled_for: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub api_version: String,
    pub created_at: String,
    pub data: Data,
    pub id: String,
    pub resource: String,
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "charge:confirmed")]
    ChargeConfirmed,
    #[serde(rename = "charge:created")]
    ChargeCreated,
    #[serde(rename = "charge:pending")]
    ChargePending,
    #[serde(rename = "charge:failed")]
    ChargeFailed,
    #[serde(rename = "charge:resolved")]
    ChargeResolved,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub id: String,
    pub code: String,
    pub name: String,
    pub utxo: bool,
    pub pricing: HashMap<String, OverpaymentAbsoluteThreshold>,
    pub fee_rate: f64,
    pub logo_url: String,
    pub metadata: Metadata,
    pub payments: Vec<PaymentElement>,
    pub resource: String,
    pub timeline: Vec<Timeline>,
    pub addresses: Addresses,
    pub pwcb_only: bool,
    pub cancel_url: String,
    pub created_at: String,
    pub expires_at: String,
    pub hosted_url: String,
    pub brand_color: String,
    pub description: String,
    pub confirmed_at: String,
    pub fees_settled: bool,
    pub pricing_type: String,
    pub redirect_url: String,
    pub support_email: String,
    pub brand_logo_url: String,
    pub exchange_rates: ExchangeRates,
    pub offchain_eligible: bool,
    pub organization_name: String,
    pub payment_threshold: PaymentThreshold,
    pub local_exchange_rates: LocalExchangeRates,
    pub coinbase_managed_merchant: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Addresses {
    pub dai: String,
    pub usdc: String,
    pub pusdc: String,
    pub pweth: String,
    pub tether: String,
    pub apecoin: String,
    pub bitcoin: String,
    pub polygon: String,
    pub dogecoin: String,
    pub ethereum: String,
    pub litecoin: String,
    pub shibainu: String,
    pub bitcoincash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeRates {
    #[serde(rename = "APE-USD")]
    pub ape_usd: String,
    #[serde(rename = "BCH-USD")]
    pub bch_usd: String,
    #[serde(rename = "BTC-USD")]
    pub btc_usd: String,
    #[serde(rename = "DAI-USD")]
    pub dai_usd: String,
    #[serde(rename = "ETH-USD")]
    pub eth_usd: String,
    #[serde(rename = "LTC-USD")]
    pub ltc_usd: String,
    #[serde(rename = "DOGE-USD")]
    pub doge_usd: String,
    #[serde(rename = "SHIB-USD")]
    pub shib_usd: String,
    #[serde(rename = "USDC-USD")]
    pub usdc_usd: String,
    #[serde(rename = "USDT-USD")]
    pub usdt_usd: String,
    #[serde(rename = "PUSDC-USD")]
    pub pusdc_usd: String,
    #[serde(rename = "PWETH-USD")]
    pub pweth_usd: String,
    #[serde(rename = "PMATIC-USD")]
    pub pmatic_usd: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalExchangeRates {
    #[serde(rename = "APE-INR")]
    pub ape_inr: String,
    #[serde(rename = "BCH-INR")]
    pub bch_inr: String,
    #[serde(rename = "BTC-INR")]
    pub btc_inr: String,
    #[serde(rename = "DAI-INR")]
    pub dai_inr: String,
    #[serde(rename = "ETH-INR")]
    pub eth_inr: String,
    #[serde(rename = "LTC-INR")]
    pub ltc_inr: String,
    #[serde(rename = "DOGE-INR")]
    pub doge_inr: String,
    #[serde(rename = "SHIB-INR")]
    pub shib_inr: String,
    #[serde(rename = "USDC-INR")]
    pub usdc_inr: String,
    #[serde(rename = "USDT-INR")]
    pub usdt_inr: String,
    #[serde(rename = "PUSDC-INR")]
    pub pusdc_inr: String,
    #[serde(rename = "PWETH-INR")]
    pub pweth_inr: String,
    #[serde(rename = "PMATIC-INR")]
    pub pmatic_inr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentThreshold {
    pub overpayment_absolute_threshold: OverpaymentAbsoluteThreshold,
    pub overpayment_relative_threshold: String,
    pub underpayment_absolute_threshold: OverpaymentAbsoluteThreshold,
    pub underpayment_relative_threshold: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverpaymentAbsoluteThreshold {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentElement {
    pub net: CoinbaseProcessingFee,
    pub block: Block,
    pub value: CoinbaseProcessingFee,
    pub status: String,
    pub network: String,
    pub deposited: Deposited,
    pub payment_id: String,
    pub detected_at: String,
    pub transaction_id: String,
    pub coinbase_processing_fee: CoinbaseProcessingFee,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub hash: String,
    pub height: i64,
    pub confirmations: i64,
    pub confirmations_required: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseProcessingFee {
    pub local: Option<OverpaymentAbsoluteThreshold>,
    pub crypto: OverpaymentAbsoluteThreshold,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deposited {
    pub amount: Amount,
    pub status: String,
    pub destination: String,
    pub exchange_rate: Option<serde_json::Value>,
    pub autoconversion_status: String,
    pub autoconversion_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Amount {
    pub net: CoinbaseProcessingFee,
    pub gross: CoinbaseProcessingFee,
    pub coinbase_fee: CoinbaseProcessingFee,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelinePayment {
    pub value: OverpaymentAbsoluteThreshold,
    pub network: String,
    pub transaction_id: String,
}
