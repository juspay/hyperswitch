use std::collections::HashMap;

use common_utils::pii;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, PaymentsAuthorizeRequestData, RouterData},
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
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CoinbasePaymentsRequest {
    pub name: Option<Secret<String>>,
    pub description: Option<String>,
    pub pricing_type: String,
    pub local_price: LocalPrice,
    pub redirect_url: String,
    pub cancel_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CoinbasePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        get_crypto_specific_payment_data(item)
    }
}

// Auth Struct
pub struct CoinbaseAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CoinbaseAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CoinbasePaymentStatus {
    New,
    #[default]
    Pending,
    Completed,
    Expired,
    Unresolved,
    Resolved,
    Canceled,
    #[serde(rename = "PENDING REFUND")]
    PendingRefund,
    Refunded,
}

impl From<CoinbasePaymentStatus> for enums::AttemptStatus {
    fn from(item: CoinbasePaymentStatus) -> Self {
        match item {
            CoinbasePaymentStatus::Completed | CoinbasePaymentStatus::Resolved => Self::Charged,
            CoinbasePaymentStatus::Expired => Self::Failure,
            CoinbasePaymentStatus::New => Self::AuthenticationPending,
            CoinbasePaymentStatus::Unresolved => Self::Unresolved,
            CoinbasePaymentStatus::Canceled => Self::Voided,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum UnResolvedContext {
    Underpaid,
    Overpaid,
    Delayed,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    status: CoinbasePaymentStatus,
    context: Option<UnResolvedContext>,
    time: String,
    pub payment: Option<TimelinePayment>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
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
        let redirection_data = services::RedirectForm::Form {
            endpoint: item.response.data.hosted_url.to_string(),
            method: services::Method::Get,
            form_fields,
        };
        let timeline = item
            .response
            .data
            .timeline
            .last()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
            .clone();
        let connector_id = types::ResponseId::ConnectorTransactionId(item.response.data.id.clone());
        let attempt_status = timeline.status.clone();
        let response_data = timeline.context.map_or(
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: connector_id.clone(),
                redirection_data: Some(redirection_data),
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.data.id.clone()),
                incremental_authorization_allowed: None,
            }),
            |context| {
                Ok(types::PaymentsResponseData::TransactionUnresolvedResponse{
                resource_id: connector_id,
                reason: Some(api::enums::UnresolvedResponseReason {
                code: context.to_string(),
                message: "Please check the transaction in coinbase dashboard and resolve manually"
                    .to_string(),
                }),
                connector_response_reference_id: Some(item.response.data.id),
            })
            },
        );
        Ok(Self {
            status: enums::AttemptStatus::from(attempt_status),
            response: response_data,
            ..item.data
        })
    }
}

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
        }
    }
}

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

#[derive(Debug, Deserialize)]
pub struct CoinbaseErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseErrorResponse {
    pub error: CoinbaseErrorData,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
pub struct CoinbaseConnectorMeta {
    pub pricing_type: String,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for CoinbaseConnectorMeta {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        utils::to_connector_meta_from_secret(meta_data.clone()).change_context(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "`pricing_type` not present in `CoinbaseConnectorMeta`",
            },
        )
    }
}

fn get_crypto_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CoinbasePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let billing_address = item
        .get_billing()
        .ok()
        .and_then(|billing_address| billing_address.address.as_ref());
    let name =
        billing_address.and_then(|add| add.get_first_name().ok().map(|name| name.to_owned()));
    let description = item.get_description().ok();
    let connector_meta = CoinbaseConnectorMeta::try_from(&item.connector_meta_data)?;
    let pricing_type = connector_meta.pricing_type;
    let local_price = get_local_price(item);
    let redirect_url = item.request.get_return_url()?;
    let cancel_url = item.request.get_return_url()?;

    Ok(CoinbasePaymentsRequest {
        name,
        description,
        pricing_type,
        local_price,
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
    pub data: CoinbasePaymentResponseData,
    pub id: String,
    pub resource: String,
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "charge:confirmed")]
    Confirmed,
    #[serde(rename = "charge:created")]
    Created,
    #[serde(rename = "charge:pending")]
    Pending,
    #[serde(rename = "charge:failed")]
    Failed,
    #[serde(rename = "charge:resolved")]
    Resolved,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CoinbasePaymentResponseData {
    pub id: String,
    pub code: String,
    pub name: Option<String>,
    pub utxo: bool,
    pub pricing: HashMap<String, OverpaymentAbsoluteThreshold>,
    pub fee_rate: f64,
    pub logo_url: String,
    pub metadata: Option<Metadata>,
    pub payments: Vec<PaymentElement>,
    pub resource: String,
    pub timeline: Vec<Timeline>,
    pub pwcb_only: bool,
    pub cancel_url: String,
    pub created_at: String,
    pub expires_at: String,
    pub hosted_url: String,
    pub brand_color: String,
    pub description: Option<String>,
    pub confirmed_at: Option<String>,
    pub fees_settled: bool,
    pub pricing_type: String,
    pub redirect_url: String,
    pub support_email: String,
    pub brand_logo_url: String,
    pub offchain_eligible: bool,
    pub organization_name: String,
    pub payment_threshold: PaymentThreshold,
    pub coinbase_managed_merchant: bool,
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct PaymentThreshold {
    pub overpayment_absolute_threshold: OverpaymentAbsoluteThreshold,
    pub overpayment_relative_threshold: String,
    pub underpayment_absolute_threshold: OverpaymentAbsoluteThreshold,
    pub underpayment_relative_threshold: String,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, PartialEq, Eq)]
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
    pub hash: Option<String>,
    pub height: Option<i64>,
    pub confirmations: Option<i64>,
    pub confirmations_required: Option<i64>,
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

#[cfg(test)]
mod tests {
    use std::{marker::PhantomData, str::FromStr};

    use common_utils::pii;

    use super::*;
    use crate::core::errors::ConnectorError;

    fn construct_payment_router_data(
        connector_meta_data: Option<pii::SecretSerdeValue>,
    ) -> types::PaymentsAuthorizeRouterData {
        let connector_auth_type = types::ConnectorAuthType::HeaderKey {
            api_key: Secret::new("api_key".to_string()),
        };

        types::RouterData {
            flow: PhantomData,
            merchant_id: String::from("Coinbase"),
            customer_id: Some(String::from("Coinbase")),
            connector: "Coinbase".to_string(),
            payment_id: uuid::Uuid::new_v4().to_string(),
            attempt_id: uuid::Uuid::new_v4().to_string(),
            status: Default::default(),
            auth_type: enums::AuthenticationType::NoThreeDs,
            payment_method: enums::PaymentMethod::Card,
            connector_auth_type,
            description: Some("This is a test".to_string()),
            return_url: None,
            request: types::PaymentsAuthorizeData {
                amount: 1000,
                currency: enums::Currency::USD,
                payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
                    card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
                    card_exp_month: Secret::new("10".to_string()),
                    card_exp_year: Secret::new("2025".to_string()),
                    card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
                    card_cvc: Secret::new("999".to_string()),
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    card_issuing_country: None,
                    bank_code: None,
                    nick_name: Some(masking::Secret::new("nick_name".into())),
                }),
                confirm: true,
                statement_descriptor_suffix: None,
                statement_descriptor: None,
                setup_future_usage: None,
                mandate_id: None,
                off_session: None,
                setup_mandate_details: None,
                capture_method: None,
                browser_info: None,
                order_details: None,
                order_category: None,
                email: None,
                session_token: None,
                enrolled_for_3ds: false,
                related_transaction_id: None,
                payment_experience: None,
                payment_method_type: None,
                router_return_url: Some("router_return_url".to_string()),
                webhook_url: None,
                complete_authorize_url: None,
                customer_id: None,
                surcharge_details: None,
                request_incremental_authorization: false,
            },
            response: Err(Default::default()),
            payment_method_id: None,
            address: Default::default(),
            connector_meta_data,
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            connector_customer: None,
            recurring_mandate_payment_data: None,

            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            payment_method_balance: None,
            connector_api_version: None,
            connector_http_status_code: None,
            apple_pay_flow: None,
            external_latency: None,
            frm_metadata: None,
        }
    }

    #[test]
    fn coinbase_payments_request_try_from_works() {
        // `connector_meta_data` as `None` - should fail
        assert_eq!(
            CoinbasePaymentsRequest::try_from(&construct_payment_router_data(None))
                .unwrap_err()
                .current_context(),
            &ConnectorError::InvalidConnectorConfig {
                config: "`pricing_type` not present in `CoinbaseConnectorMeta`"
            },
        );

        // `connector_meta_data` as empty json - should fail
        assert_eq!(
            CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
                serde_json::json!({})
            ))))
            .unwrap_err()
            .current_context(),
            &ConnectorError::InvalidConnectorConfig {
                config: "`pricing_type` not present in `CoinbaseConnectorMeta`"
            },
        );

        // `connector_meta_data` as json with missing `pricing_type`  - should fail
        assert_eq!(
            CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
                serde_json::json!({ "wrong_type" : "blah" })
            ))))
            .unwrap_err()
            .current_context(),
            &ConnectorError::InvalidConnectorConfig {
                config: "`pricing_type` not present in `CoinbaseConnectorMeta`"
            },
        );

        // `connector_meta_data` as json with correct `pricing_type`  - ok
        assert_eq!(
            CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
                serde_json::json!({ "pricing_type" : "blah" })
            ))))
            .unwrap(),
            CoinbasePaymentsRequest {
                name: None,
                description: Some("This is a test".to_string()),
                pricing_type: "blah".to_string(),
                local_price: LocalPrice {
                    amount: "1000".to_string(),
                    currency: "USD".to_string()
                },
                redirect_url: "router_return_url".to_string(),
                cancel_url: "router_return_url".to_string(),
            }
        );
    }
}
