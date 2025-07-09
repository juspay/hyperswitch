use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        payments::Void,
        refunds::{Execute, RSync},
    },
    router_request_types::{PaymentsCancelData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

//TODO: Fill the struct with respective fields
pub struct SilverflowRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for SilverflowRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Basic structures for Silverflow API
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amount {
    value: i64,
    currency: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Card {
    #[serde(rename = "number")]
    number: cards::CardNumber,
    #[serde(rename = "expiryMonth")]
    expiry_month: u8,
    #[serde(rename = "expiryYear")]
    expiry_year: u16,
    cvc: Secret<String>,
    #[serde(rename = "holderName")]
    holder_name: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct MerchantAcceptorResolver {
    #[serde(rename = "merchantAcceptorKey")]
    merchant_acceptor_key: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SilverflowPaymentsRequest {
    #[serde(rename = "merchantAcceptorResolver")]
    merchant_acceptor_resolver: MerchantAcceptorResolver,
    card: Card,
    amount: Amount,
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentType {
    intent: String,
    #[serde(rename = "cardEntry")]
    card_entry: String,
    order: String,
}

impl TryFrom<&SilverflowRouterData<&PaymentsAuthorizeRouterData>> for SilverflowPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SilverflowRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Convert MinorUnit to i64 for amount value
                let amount_value = item.router_data.request.minor_amount.get_amount_as_i64();

                let card = Card {
                    number: req_card.card_number,
                    expiry_month: req_card
                        .card_exp_month
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    expiry_year: req_card
                        .card_exp_year
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    cvc: req_card.card_cvc,
                    holder_name: req_card.card_holder_name,
                };

                Ok(Self {
                    merchant_acceptor_resolver: MerchantAcceptorResolver {
                        merchant_acceptor_key: "default".to_string(), // This should come from connector metadata
                    },
                    card,
                    amount: Amount {
                        value: amount_value,
                        currency: item.router_data.request.currency.to_string(),
                    },
                    payment_type: PaymentType {
                        intent: "purchase".to_string(),
                        card_entry: "e-commerce".to_string(),
                        order: "checkout".to_string(),
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct for HTTP Basic Authentication
pub struct SilverflowAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SilverflowAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.clone(),
                api_secret: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Payment Authorization Response Structures
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentStatus {
    pub authentication: String,
    pub authorization: String,
    pub clearing: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MerchantAcceptorRef {
    pub key: String,
    pub version: i32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardResponse {
    #[serde(rename = "maskedNumber")]
    pub masked_number: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Authentication {
    pub sca: SCA,
    pub cvc: String,
    pub avs: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SCA {
    pub compliance: String,
    #[serde(rename = "complianceReason")]
    pub compliance_reason: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<SCAResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SCAResult {
    pub version: String,
    #[serde(rename = "directoryServerTransId")]
    pub directory_server_trans_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthorizationIsoFields {
    #[serde(rename = "responseCode")]
    pub response_code: String,
    #[serde(rename = "responseCodeDescription")]
    pub response_code_description: String,
    #[serde(rename = "authorizationCode")]
    pub authorization_code: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "systemTraceAuditNumber")]
    pub system_trace_audit_number: String,
    #[serde(rename = "retrievalReferenceNumber")]
    pub retrieval_reference_number: String,
    pub eci: String,
    #[serde(rename = "networkSpecificFields")]
    pub network_specific_fields: NetworkSpecificFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkSpecificFields {
    #[serde(rename = "transactionIdentifier")]
    pub transaction_identifier: String,
    #[serde(rename = "cvv2ResultCode")]
    pub cvv2_result_code: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowPaymentsResponse {
    pub key: String,
    #[serde(rename = "merchantAcceptorRef")]
    pub merchant_acceptor_ref: MerchantAcceptorRef,
    pub card: CardResponse,
    pub amount: Amount,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    #[serde(rename = "clearingMode")]
    pub clearing_mode: String,
    pub status: PaymentStatus,
    pub authentication: Authentication,
    #[serde(rename = "localTransactionDateTime")]
    pub local_transaction_date_time: String,
    #[serde(rename = "fraudLiability")]
    pub fraud_liability: String,
    #[serde(rename = "authorizationIsoFields")]
    pub authorization_iso_fields: Option<AuthorizationIsoFields>,
    pub created: String,
    pub version: i32,
}

impl From<&PaymentStatus> for common_enums::AttemptStatus {
    fn from(status: &PaymentStatus) -> Self {
        match (status.authorization.as_str(), status.clearing.as_str()) {
            ("approved", "cleared") => Self::Charged,
            ("approved", "pending") => Self::Authorized,
            ("declined", _) => Self::Failure,
            ("failed", _) => Self::Failure,
            ("pending", _) => Self::Pending,
            _ => Self::Pending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SilverflowPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SilverflowPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(&item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.key.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item
                    .response
                    .authorization_iso_fields
                    .as_ref()
                    .map(|fields| {
                        fields
                            .network_specific_fields
                            .transaction_identifier
                            .clone()
                    }),
                connector_response_reference_id: Some(item.response.key.clone()),
                incremental_authorization_allowed: Some(false),
                charges: None,
            }),
            ..item.data
        })
    }
}

// CAPTURE:
// Type definition for CaptureRequest based on Silverflow API documentation
#[derive(Default, Debug, Serialize)]
pub struct SilverflowCaptureRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(rename = "closeCharge", skip_serializing_if = "Option::is_none")]
    pub close_charge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl TryFrom<&PaymentsCaptureRouterData> for SilverflowCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        // amount_to_capture is directly an i64, representing the amount in minor units
        let amount_to_capture = if item.request.amount_to_capture > 0 {
            Some(item.request.amount_to_capture)
        } else {
            None // If no amount specified, Silverflow will clear the full amount
        };

        Ok(Self {
            amount: amount_to_capture,
            close_charge: Some(true), // Default to closing charge after capture
            reference: Some(format!("capture-{}", item.payment_id)),
        })
    }
}

// Type definition for CaptureResponse based on Silverflow clearing action response
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowCaptureResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    #[serde(rename = "chargeKey")]
    pub charge_key: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub amount: Amount,
    pub created: String,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub version: i32,
}

impl From<&SilverflowCaptureResponse> for common_enums::AttemptStatus {
    fn from(response: &SilverflowCaptureResponse) -> Self {
        match response.status.as_str() {
            "completed" => Self::Charged,
            "pending" => Self::Pending,
            "failed" => Self::Failure,
            _ => Self::Pending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SilverflowCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SilverflowCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(&item.response),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.charge_key.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.key.clone()),
                incremental_authorization_allowed: Some(false),
                charges: None,
            }),
            ..item.data
        })
    }
}

// VOID/REVERSE:
// Type definition for Reverse Charge Request based on Silverflow API documentation
#[derive(Default, Debug, Serialize)]
pub struct SilverflowVoidRequest {
    #[serde(rename = "replacementAmount", skip_serializing_if = "Option::is_none")]
    pub replacement_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl TryFrom<&RouterData<Void, PaymentsCancelData, PaymentsResponseData>>
    for SilverflowVoidRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            replacement_amount: Some(0), // Default to 0 for full reversal
            reference: Some(format!("void-{}", item.payment_id)),
        })
    }
}

// Type definition for Void Status (only authorization, no clearing)
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoidStatus {
    pub authorization: String,
}

// Type definition for Reverse Charge Response based on Silverflow API documentation
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowVoidResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    #[serde(rename = "chargeKey")]
    pub charge_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "replacementAmount")]
    pub replacement_amount: Amount,
    pub status: VoidStatus,
    #[serde(rename = "authorizationResponse")]
    pub authorization_response: Option<AuthorizationResponse>,
    pub created: String,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub version: i32,
}

impl From<&SilverflowVoidResponse> for common_enums::AttemptStatus {
    fn from(response: &SilverflowVoidResponse) -> Self {
        match response.status.authorization.as_str() {
            "approved" => Self::Voided,
            "declined" | "failed" => Self::VoidFailed,
            "pending" => Self::Pending,
            _ => Self::Pending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SilverflowVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SilverflowVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(&item.response),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.charge_key.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.key.clone()),
                incremental_authorization_allowed: Some(false),
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for DynamicDescriptor
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DynamicDescriptor {
    #[serde(rename = "merchantName")]
    pub merchant_name: String,
    #[serde(rename = "merchantCity")]
    pub merchant_city: String,
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct SilverflowRefundRequest {
    #[serde(rename = "refundAmount")]
    pub refund_amount: i64,
    pub reference: String,
}

impl<F> TryFrom<&SilverflowRouterData<&RefundsRouterData<F>>> for SilverflowRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SilverflowRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let refund_amount_value = item
            .router_data
            .request
            .minor_refund_amount
            .get_amount_as_i64();

        Ok(Self {
            refund_amount: refund_amount_value,
            reference: format!("refund-{}", item.router_data.request.refund_id),
        })
    }
}

// Type definition for Authorization Response
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AuthorizationResponse {
    pub network: String,
    #[serde(rename = "responseCode")]
    pub response_code: String,
    #[serde(rename = "responseCodeDescription")]
    pub response_code_description: String,
}

// Type definition for Refund Status
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RefundStatus {
    pub authorization: String,
}

impl From<&RefundStatus> for enums::RefundStatus {
    fn from(item: &RefundStatus) -> Self {
        match item.authorization.as_str() {
            "approved" => Self::Success,
            "declined" | "failed" => Self::Failure,
            "pending" => Self::Pending,
            _ => Self::Pending,
        }
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefundResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    #[serde(rename = "chargeKey")]
    pub charge_key: String,
    pub reference: String,
    pub amount: Amount,
    #[serde(rename = "status")]
    pub status: RefundStatus,
    #[serde(rename = "clearAfter")]
    pub clear_after: Option<String>,
    #[serde(rename = "authorizationResponse")]
    pub authorization_response: Option<AuthorizationResponse>,
    pub created: String,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub version: i32,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.key.clone(),
                refund_status: enums::RefundStatus::from(&item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.key.clone(),
                refund_status: enums::RefundStatus::from(&item.response.status),
            }),
            ..item.data
        })
    }
}

// TOKENIZATION:
// Type definition for TokenizationRequest
#[derive(Default, Debug, Serialize)]
pub struct SilverflowTokenizationRequest {
    pub reference: String,
    #[serde(rename = "cardData")]
    pub card_data: CardData,
}

#[derive(Default, Debug, Serialize)]
pub struct CardData {
    pub number: String,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: u8,
    #[serde(rename = "expiryYear")]
    pub expiry_year: u16,
    pub cvc: String,
    #[serde(rename = "holderName")]
    pub holder_name: String,
}

impl TryFrom<&PaymentsAuthorizeRouterData> for SilverflowTokenizationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card_data = CardData {
                    number: req_card.card_number.peek().to_string(),
                    expiry_month: req_card
                        .card_exp_month
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    expiry_year: req_card
                        .card_exp_year
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    cvc: req_card.card_cvc.expose(),
                    holder_name: req_card
                        .card_holder_name
                        .map(|name| name.expose())
                        .unwrap_or_default(),
                };

                Ok(Self {
                    reference: format!("CUSTOMER_ID_{}", item.payment_id),
                    card_data,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Add TryFrom implementation for direct tokenization router data
impl
    TryFrom<
        &RouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    > for SilverflowTokenizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card_data = CardData {
                    number: req_card.card_number.peek().to_string(),
                    expiry_month: req_card
                        .card_exp_month
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    expiry_year: req_card
                        .card_exp_year
                        .expose()
                        .parse()
                        .map_err(|_| errors::ConnectorError::ParsingFailed)?,
                    cvc: req_card.card_cvc.expose(),
                    holder_name: req_card
                        .card_holder_name
                        .map(|name| name.expose())
                        .unwrap_or_default(),
                };

                Ok(Self {
                    reference: format!("CUSTOMER_ID_{}", item.payment_id),
                    card_data,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Type definition for TokenizationResponse
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowTokenizationResponse {
    pub key: String,
    #[serde(rename = "agentKey")]
    pub agent_key: String,
    pub last4: String,
    pub status: String,
    pub reference: String,
    #[serde(rename = "cardInfo")]
    pub card_info: Vec<CardInfo>,
    pub created: String,
    #[serde(rename = "cvcPresent")]
    pub cvc_present: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardInfo {
    #[serde(rename = "infoSource")]
    pub info_source: String,
    pub network: String,
    #[serde(rename = "primaryNetwork")]
    pub primary_network: bool,
}

impl<F, T> TryFrom<ResponseRouterData<F, SilverflowTokenizationResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SilverflowTokenizationResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::Pending,
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.key,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenizedCardDetails {
    #[serde(rename = "maskedCardNumber")]
    pub masked_card_number: String,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: u8,
    #[serde(rename = "expiryYear")]
    pub expiry_year: u16,
    #[serde(rename = "cardBrand")]
    pub card_brand: String,
}

// WEBHOOKS:
// Type definition for Webhook Event structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowWebhookEvent {
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "eventData")]
    pub event_data: SilverflowWebhookEventData,
    #[serde(rename = "eventId")]
    pub event_id: String,
    pub created: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilverflowWebhookEventData {
    #[serde(rename = "chargeKey")]
    pub charge_key: Option<String>,
    #[serde(rename = "refundKey")]
    pub refund_key: Option<String>,
    pub status: Option<PaymentStatus>,
    pub amount: Option<Amount>,
    #[serde(rename = "transactionReference")]
    pub transaction_reference: Option<String>,
}

// Error Response Structures based on Silverflow API format
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetails {
    pub field: String,
    pub issue: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SilverflowErrorResponse {
    pub error: SilverflowError,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SilverflowError {
    pub code: String,
    pub message: String,
    pub details: Option<ErrorDetails>,
}
