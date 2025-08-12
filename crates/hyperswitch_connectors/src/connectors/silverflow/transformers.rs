use common_enums::enums;
use common_utils::types::MinorUnit;
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

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CardData},
};

//TODO: Fill the struct with respective fields
pub struct SilverflowRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for SilverflowRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
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
    value: MinorUnit,
    currency: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    holder_name: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAcceptorResolver {
    merchant_acceptor_key: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowPaymentsRequest {
    merchant_acceptor_resolver: MerchantAcceptorResolver,
    card: Card,
    amount: Amount,
    #[serde(rename = "type")]
    payment_type: PaymentType,
    clearing_mode: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentType {
    intent: String,
    card_entry: String,
    order: String,
}

impl TryFrom<&SilverflowRouterData<&PaymentsAuthorizeRouterData>> for SilverflowPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SilverflowRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // Check if 3DS is being requested - Silverflow doesn't support 3DS
        if matches!(
            item.router_data.auth_type,
            enums::AuthenticationType::ThreeDs
        ) {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("silverflow"),
            )
            .into());
        }

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Extract merchant acceptor key from connector auth
                let auth = SilverflowAuthType::try_from(&item.router_data.connector_auth_type)?;

                let card = Card {
                    number: req_card.card_number.clone(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.card_exp_year.clone(),
                    cvc: req_card.card_cvc.clone(),
                    holder_name: req_card.get_cardholder_name().ok(),
                };

                // Determine clearing mode based on capture method
                let clearing_mode = match item.router_data.request.capture_method {
                    Some(enums::CaptureMethod::Manual) => "manual".to_string(),
                    Some(enums::CaptureMethod::Automatic) | None => "auto".to_string(),
                    Some(enums::CaptureMethod::ManualMultiple)
                    | Some(enums::CaptureMethod::Scheduled)
                    | Some(enums::CaptureMethod::SequentialAutomatic) => {
                        return Err(errors::ConnectorError::NotSupported {
                            message: "Capture method not supported by Silverflow".to_string(),
                            connector: "Silverflow",
                        }
                        .into());
                    }
                };

                Ok(Self {
                    merchant_acceptor_resolver: MerchantAcceptorResolver {
                        merchant_acceptor_key: auth.merchant_acceptor_key.expose(),
                    },
                    card,
                    amount: Amount {
                        value: item.amount,
                        currency: item.router_data.request.currency.to_string(),
                    },
                    payment_type: PaymentType {
                        intent: "purchase".to_string(),
                        card_entry: "e-commerce".to_string(),
                        order: "checkout".to_string(),
                    },
                    clearing_mode,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("silverflow"),
            )
            .into()),
        }
    }
}

// Auth Struct for HTTP Basic Authentication
pub struct SilverflowAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
    pub(super) merchant_acceptor_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SilverflowAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.clone(),
                api_secret: api_secret.clone(),
                merchant_acceptor_key: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Enum for Silverflow payment authorization status
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowAuthorizationStatus {
    Approved,
    Declined,
    Failed,
    #[default]
    Pending,
}

// Enum for Silverflow payment clearing status
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowClearingStatus {
    Cleared,
    #[default]
    Pending,
    Failed,
}

// Payment Authorization Response Structures
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentStatus {
    pub authentication: String,
    pub authorization: SilverflowAuthorizationStatus,
    pub clearing: SilverflowClearingStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MerchantAcceptorRef {
    pub key: String,
    pub version: i32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardResponse {
    pub masked_number: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Authentication {
    pub sca: SCA,
    pub cvc: Secret<String>,
    pub avs: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SCA {
    pub compliance: String,
    pub compliance_reason: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<SCAResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SCAResult {
    pub version: String,
    pub directory_server_trans_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationIsoFields {
    pub response_code: String,
    pub response_code_description: String,
    pub authorization_code: String,
    pub network_code: String,
    pub system_trace_audit_number: Secret<String>,
    pub retrieval_reference_number: String,
    pub eci: String,
    pub network_specific_fields: NetworkSpecificFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSpecificFields {
    pub transaction_identifier: String,
    pub cvv2_result_code: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowPaymentsResponse {
    pub key: String,
    pub merchant_acceptor_ref: MerchantAcceptorRef,
    pub card: CardResponse,
    pub amount: Amount,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub clearing_mode: String,
    pub status: PaymentStatus,
    pub authentication: Authentication,
    pub local_transaction_date_time: String,
    pub fraud_liability: String,
    pub authorization_iso_fields: Option<AuthorizationIsoFields>,
    pub created: String,
    pub version: i32,
}

impl From<&PaymentStatus> for common_enums::AttemptStatus {
    fn from(status: &PaymentStatus) -> Self {
        match (&status.authorization, &status.clearing) {
            (SilverflowAuthorizationStatus::Approved, SilverflowClearingStatus::Cleared) => {
                Self::Charged
            }
            (SilverflowAuthorizationStatus::Approved, SilverflowClearingStatus::Pending) => {
                Self::Authorized
            }
            (SilverflowAuthorizationStatus::Approved, SilverflowClearingStatus::Failed) => {
                Self::Failure
            }
            (SilverflowAuthorizationStatus::Declined, _) => Self::Failure,
            (SilverflowAuthorizationStatus::Failed, _) => Self::Failure,
            (SilverflowAuthorizationStatus::Pending, _) => Self::Pending,
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
#[serde(rename_all = "camelCase")]
pub struct SilverflowCaptureRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_charge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl TryFrom<&PaymentsCaptureRouterData> for SilverflowCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        // amount_to_capture is directly an i64, representing the amount in minor units
        let amount_to_capture = Some(item.request.amount_to_capture);

        Ok(Self {
            amount: amount_to_capture,
            close_charge: Some(true), // Default to closing charge after capture
            reference: Some(format!("capture-{}", item.payment_id)),
        })
    }
}

// Enum for Silverflow capture status
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowCaptureStatus {
    Completed,
    #[default]
    Pending,
    Failed,
}

// Type definition for CaptureResponse based on Silverflow clearing action response
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowCaptureResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    pub charge_key: String,
    pub status: SilverflowCaptureStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub amount: Amount,
    pub created: String,
    pub last_modified: String,
    pub version: i32,
}

impl From<&SilverflowCaptureResponse> for common_enums::AttemptStatus {
    fn from(response: &SilverflowCaptureResponse) -> Self {
        match response.status {
            SilverflowCaptureStatus::Completed => Self::Charged,
            SilverflowCaptureStatus::Pending => Self::Pending,
            SilverflowCaptureStatus::Failed => Self::Failure,
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
#[serde(rename_all = "camelCase")]
pub struct SilverflowVoidRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
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

// Enum for Silverflow void authorization status
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowVoidAuthorizationStatus {
    Approved,
    Declined,
    Failed,
    #[default]
    Pending,
}

// Type definition for Void Status (only authorization, no clearing)
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoidStatus {
    pub authorization: SilverflowVoidAuthorizationStatus,
}

// Type definition for Reverse Charge Response based on Silverflow API documentation
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowVoidResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    pub charge_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub replacement_amount: Amount,
    pub status: VoidStatus,
    pub authorization_response: Option<AuthorizationResponse>,
    pub created: String,
    pub last_modified: String,
    pub version: i32,
}

impl From<&SilverflowVoidResponse> for common_enums::AttemptStatus {
    fn from(response: &SilverflowVoidResponse) -> Self {
        match response.status.authorization {
            SilverflowVoidAuthorizationStatus::Approved => Self::Voided,
            SilverflowVoidAuthorizationStatus::Declined
            | SilverflowVoidAuthorizationStatus::Failed => Self::VoidFailed,
            SilverflowVoidAuthorizationStatus::Pending => Self::Pending,
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
#[serde(rename_all = "camelCase")]
pub struct DynamicDescriptor {
    pub merchant_name: String,
    pub merchant_city: String,
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct SilverflowRefundRequest {
    #[serde(rename = "refundAmount")]
    pub refund_amount: MinorUnit,
    pub reference: String,
}

impl<F> TryFrom<&SilverflowRouterData<&RefundsRouterData<F>>> for SilverflowRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SilverflowRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            refund_amount: item.amount,
            reference: format!("refund-{}", item.router_data.request.refund_id),
        })
    }
}

// Type definition for Authorization Response
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationResponse {
    pub network: String,

    pub response_code: String,

    pub response_code_description: String,
}

// Enum for Silverflow refund authorization status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowRefundAuthorizationStatus {
    Approved,
    Declined,
    Failed,
    Pending,
}

// Enum for Silverflow refund status
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SilverflowRefundStatus {
    Success,
    Failure,
    #[default]
    Pending,
}

impl From<&SilverflowRefundStatus> for enums::RefundStatus {
    fn from(item: &SilverflowRefundStatus) -> Self {
        match item {
            SilverflowRefundStatus::Success => Self::Success,
            SilverflowRefundStatus::Failure => Self::Failure,
            SilverflowRefundStatus::Pending => Self::Pending,
        }
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub key: String,
    pub charge_key: String,
    pub reference: String,
    pub amount: Amount,
    pub status: SilverflowRefundStatus,
    pub clear_after: Option<String>,
    pub authorization_response: Option<AuthorizationResponse>,
    pub created: String,
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
#[serde(rename_all = "camelCase")]
pub struct SilverflowTokenizationRequest {
    pub reference: String,

    pub card_data: SilverflowCardData,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowCardData {
    pub number: String,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: String,
    pub holder_name: String,
}

impl TryFrom<&PaymentsAuthorizeRouterData> for SilverflowTokenizationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card_data = SilverflowCardData {
                    number: req_card.card_number.peek().to_string(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.card_exp_year.clone(),
                    cvc: req_card.card_cvc.clone().expose(),
                    holder_name: req_card
                        .get_cardholder_name()
                        .unwrap_or(Secret::new("".to_string()))
                        .expose(),
                };

                Ok(Self {
                    reference: format!("CUSTOMER_ID_{}", item.payment_id),
                    card_data,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("silverflow"),
            )
            .into()),
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
                let card_data = SilverflowCardData {
                    number: req_card.card_number.peek().to_string(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.card_exp_year.clone(),
                    cvc: req_card.card_cvc.clone().expose(),
                    holder_name: req_card
                        .get_cardholder_name()
                        .unwrap_or(Secret::new("".to_string()))
                        .expose(),
                };

                Ok(Self {
                    reference: format!("CUSTOMER_ID_{}", item.payment_id),
                    card_data,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("silverflow"),
            )
            .into()),
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
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.key,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardDetails {
    pub masked_card_number: String,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub card_brand: String,
}

// WEBHOOKS:
// Type definition for Webhook Event structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowWebhookEvent {
    pub event_type: String,
    pub event_data: SilverflowWebhookEventData,
    pub event_id: String,
    pub created: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilverflowWebhookEventData {
    pub charge_key: Option<String>,
    pub refund_key: Option<String>,
    pub status: Option<PaymentStatus>,
    pub amount: Option<Amount>,
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
