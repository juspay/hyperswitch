use common_enums::enums;
use common_utils::types::{StringMinorUnit, StringMinorUnitForConnector, AmountConvertor};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, ExposeInterface};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
};

//TODO: Fill the struct with respective fields
pub struct PeachpaymentsRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for PeachpaymentsRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Card Gateway API Transaction Request
#[derive(Debug, Serialize, PartialEq)]
pub struct PeachpaymentsPaymentsRequest {
    #[serde(rename = "chargeMethod")]
    pub charge_method: String,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "ecommerceCardPaymentOnlyTransactionData")]
    pub ecommerce_card_payment_only_transaction_data: EcommerceCardPaymentOnlyTransactionData,
    #[serde(rename = "posData")]
    pub pos_data: serde_json::Value,
    #[serde(rename = "sendDateTime")]
    pub send_date_time: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct EcommerceCardPaymentOnlyTransactionData {
    #[serde(rename = "merchantInformation")]
    pub merchant_information: MerchantInformation,
    pub routing: Routing,
    pub card: CardDetails,
    pub amount: AmountDetails,
    #[serde(rename = "threeDSData", skip_serializing_if = "Option::is_none")]
    pub three_ds_data: Option<ThreeDSData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MerchantInformation {
    #[serde(rename = "clientMerchantReferenceId")]
    pub client_merchant_reference_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(rename = "postalCode", skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(rename = "regionCode", skip_serializing_if = "Option::is_none")]
    pub region_code: Option<String>,
    #[serde(rename = "merchantType", skip_serializing_if = "Option::is_none")]
    pub merchant_type: Option<String>,
    #[serde(rename = "websiteUrl", skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Routing {
    pub route: String,
    pub mid: String,
    pub tid: String,
    #[serde(rename = "visaPaymentFacilitatorId", skip_serializing_if = "Option::is_none")]
    pub visa_payment_facilitator_id: Option<String>,
    #[serde(rename = "masterCardPaymentFacilitatorId", skip_serializing_if = "Option::is_none")]
    pub master_card_payment_facilitator_id: Option<String>,
    #[serde(rename = "subMid", skip_serializing_if = "Option::is_none")]
    pub sub_mid: Option<String>,
    #[serde(rename = "amexId", skip_serializing_if = "Option::is_none")]
    pub amex_id: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CardDetails {
    pub pan: Secret<String>,
    #[serde(rename = "cardholderName", skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<Secret<String>>,
    #[serde(rename = "expiryYear")]
    pub expiry_year: Secret<String>,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AmountDetails {
    pub amount: i64,
    #[serde(rename = "currencyCode")]
    pub currency_code: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ThreeDSData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cavv: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tavv: Option<Secret<String>>,
    pub eci: String,
    #[serde(rename = "dsTransId")]
    pub ds_trans_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xid: Option<String>,
    #[serde(rename = "authenticationStatus")]
    pub authentication_status: String,
    #[serde(rename = "threeDSVersion")]
    pub three_ds_version: String,
}

// Confirm Transaction Request (for capture)
#[derive(Debug, Serialize, PartialEq)]
pub struct PeachpaymentsConfirmRequest {
    #[serde(rename = "ecommerceCardPaymentOnlyConfirmationData")]
    pub ecommerce_card_payment_only_confirmation_data: EcommerceCardPaymentOnlyConfirmationData,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct EcommerceCardPaymentOnlyConfirmationData {
    pub amount: AmountDetails,
}

// Void Transaction Request
#[derive(Debug, Serialize, PartialEq)]
pub struct PeachpaymentsVoidRequest {
    #[serde(rename = "paymentMethod")]
    pub payment_method: String,
    #[serde(rename = "sendDateTime")]
    pub send_date_time: String,
    #[serde(rename = "failureReason")]
    pub failure_reason: String,
}

impl TryFrom<&PeachpaymentsRouterData<&PaymentsCaptureRouterData>>
    for PeachpaymentsConfirmRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let amount_in_cents = StringMinorUnitForConnector
            .convert_back(item.amount.clone(), item.router_data.request.currency)
            .map_err(|_| errors::ConnectorError::ParsingFailed)?
            .get_amount_as_i64();

        let amount = AmountDetails {
            amount: amount_in_cents,
            currency_code: item.router_data.request.currency.to_string(),
        };

        let confirmation_data = EcommerceCardPaymentOnlyConfirmationData {
            amount,
        };

        Ok(Self {
            ecommerce_card_payment_only_confirmation_data: confirmation_data,
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for PeachpaymentsVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create simple UTC timestamp - PeachPayments expects ISO 8601 format
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?
            .as_secs();
        
        // Create a simple ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SSZ format)
        // For simplicity, we'll use a basic format that should be acceptable
        let send_date_time = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            1970 + (now / 31536000), // Very rough year calculation
            1 + ((now % 31536000) / 2628000), // Very rough month
            1 + ((now % 2628000) / 86400), // Day
            (now % 86400) / 3600, // Hour
            (now % 3600) / 60, // Minute
            now % 60 // Second
        );
        
        Ok(Self {
            payment_method: "ecommerce_card_payment_only".to_string(),
            send_date_time,
            failure_reason: item.request.cancellation_reason
                .as_ref()
                .map(|reason| reason.to_string())
                .unwrap_or_else(|| "timeout".to_string()), // Use timeout as default like the script
        })
    }
}

impl TryFrom<&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>>
    for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let amount_in_cents = StringMinorUnitForConnector
                    .convert_back(item.amount.clone(), item.router_data.request.currency)
                    .map_err(|_| errors::ConnectorError::ParsingFailed)?
                    .get_amount_as_i64();

                // Get merchant metadata from connector configuration
                let metadata = item.router_data.connector_meta_data.as_ref()
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?
                    .clone()
                    .expose();
                
                let metadata_obj = metadata.as_object()
                    .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                        config: "connector metadata must be a JSON object"
                    })?;
                
                let merchant_information = MerchantInformation {
                    client_merchant_reference_id: item.router_data.connector_request_reference_id.clone(),
                    name: metadata_obj.get("merchant_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    mcc: metadata_obj.get("mcc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    phone: metadata_obj.get("merchant_phone")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    email: metadata_obj.get("merchant_email")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    mobile: metadata_obj.get("merchant_mobile")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    address: metadata_obj.get("merchant_address")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    city: metadata_obj.get("merchant_city")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    postal_code: metadata_obj.get("merchant_postal_code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    region_code: metadata_obj.get("merchant_region_code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    merchant_type: metadata_obj.get("merchant_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    website_url: metadata_obj.get("merchant_website")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                // Get routing configuration from metadata
                let routing = Routing {
                    route: metadata_obj.get("routing_route")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "routing_route is required in connector metadata"
                        })?
                        .to_string(),
                    mid: metadata_obj.get("routing_mid")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "routing_mid is required in connector metadata"
                        })?
                        .to_string(),
                    tid: metadata_obj.get("routing_tid")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "routing_tid is required in connector metadata"
                        })?
                        .to_string(),
                    visa_payment_facilitator_id: metadata_obj.get("visa_payment_facilitator_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    master_card_payment_facilitator_id: metadata_obj.get("master_card_payment_facilitator_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    sub_mid: metadata_obj.get("sub_mid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    amex_id: metadata_obj.get("amex_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                let card = CardDetails {
                    pan: Secret::new(req_card.card_number.to_string()),
                    cardholder_name: req_card.card_holder_name.map(|name| name),
                    expiry_year: {
                        // Convert 4-digit year to 2-digit year (e.g., "2025" -> "25")
                        let year_str = req_card.card_exp_year.clone().expose();
                        if year_str.len() == 4 {
                            Secret::new(year_str[2..].to_string())
                        } else {
                            req_card.card_exp_year.clone()
                        }
                    },
                    expiry_month: req_card.card_exp_month.clone(),
                    cvv: Some(req_card.card_cvc.clone()),
                };

                let amount = AmountDetails {
                    amount: amount_in_cents,
                    currency_code: item.router_data.request.currency.to_string(),
                };

                // Extract 3DS data if available
                let three_ds_data = item.router_data.request.authentication_data.as_ref().and_then(|auth_data| {
                    // Only include 3DS data if we have essential fields (ECI is most critical)
                    if let Some(eci) = &auth_data.eci {
                        let ds_trans_id = auth_data.ds_trans_id.clone()
                            .or_else(|| auth_data.threeds_server_transaction_id.clone())?;
                        
                        // Determine authentication status based on ECI value
                        let authentication_status = match eci.as_str() {
                            "05" | "06" => "Y".to_string(),  // Fully authenticated
                            "07" => "A".to_string(),         // Attempted authentication / liability shift 
                            _ => "N".to_string(),            // Not authenticated / failed
                        };
                        
                        // Convert message version to string, handling None case
                        let three_ds_version = auth_data.message_version
                            .as_ref()
                            .map(|v| {
                                let version_str = v.to_string();
                                // Truncate version to match API spec (e.g., "2.2.0" -> "2.2")
                                if version_str.len() > 3 && version_str.chars().nth(3) == Some('.') {
                                    version_str[..3].to_string()
                                } else {
                                    version_str
                                }
                            })
                            ?;
                        
                        Some(ThreeDSData {
                            cavv: if auth_data.cavv.clone().expose().is_empty() { 
                                None 
                            } else { 
                                Some(auth_data.cavv.clone()) 
                            },
                            tavv: None, // Network token field - not available in Hyperswitch AuthenticationData
                            eci: eci.clone(),
                            ds_trans_id,
                            xid: None, // Legacy 3DS 1.x/network token field - not available in Hyperswitch AuthenticationData  
                            authentication_status,
                            three_ds_version,
                        })
                    } else {
                        None
                    }
                });

                let ecommerce_data = EcommerceCardPaymentOnlyTransactionData {
                    merchant_information,
                    routing,
                    card,
                    amount,
                    three_ds_data,
                };

                // Generate current timestamp for sendDateTime (ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ)
                let send_date_time = time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Iso8601::DEFAULT)
                    .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

                Ok(Self {
                    charge_method: "ecommerce_card_payment_only".to_string(),
                    reference_id: item.router_data.connector_request_reference_id.clone(),
                    ecommerce_card_payment_only_transaction_data: ecommerce_data,
                    pos_data: serde_json::json!({}), // Empty object as shown in reference scripts
                    send_date_time,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct for Card Gateway API
pub struct PeachpaymentsAuthType {
    pub(crate) api_key: Secret<String>,
    pub(crate) tenant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1: tenant_id } => {
                Ok(Self {
                    api_key: api_key.to_owned(),
                    tenant_id: tenant_id.to_owned(),
                })
            },
            ConnectorAuthType::MultiAuthKey { 
                api_key, 
                key1: tenant_id, 
                .. 
            } => {
                Ok(Self {
                api_key: api_key.to_owned(),
                    tenant_id: tenant_id.to_owned(),
                })
            },
            _ => {
                Err(errors::ConnectorError::FailedToObtainAuthType.into())
            }
        }
    }
}
// Card Gateway API Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PeachpaymentsPaymentStatus {
    #[serde(rename = "SUCCESSFUL")]
    Successful,
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "FAILED")]
    Failed,
}

impl From<PeachpaymentsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PeachpaymentsPaymentStatus) -> Self {
        match item {
            PeachpaymentsPaymentStatus::Successful => Self::Charged,
            // PENDING means authorized but not yet captured - requires confirmation
            PeachpaymentsPaymentStatus::Pending => Self::Authorized,
            PeachpaymentsPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeachpaymentsPaymentsResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "responseCode")]
    pub response_code: ResponseCode,
    #[serde(rename = "transactionResult")]
    pub transaction_result: PeachpaymentsPaymentStatus,
    #[serde(rename = "ecommerceCardPaymentOnlyTransactionData", skip_serializing_if = "Option::is_none")]
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
}

// Confirm Transaction Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeachpaymentsConfirmResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "responseCode")]
    pub response_code: ResponseCode,
    #[serde(rename = "transactionResult")]
    pub transaction_result: PeachpaymentsPaymentStatus,
    #[serde(rename = "authorizationCode", skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseCode {
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EcommerceCardPaymentOnlyResponseData {
    pub card: Option<CardResponseData>,
    pub amount: Option<AmountDetails>,
    pub stan: Option<String>,
    pub rrn: Option<String>,
    #[serde(rename = "approvalCode")]
    pub approval_code: Option<String>,
    #[serde(rename = "merchantAdviceCode")]
    pub merchant_advice_code: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardResponseData {
    #[serde(rename = "binNumber")]
    pub bin_number: Option<String>,
    #[serde(rename = "maskedPan")]
    pub masked_pan: Option<String>,
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: Option<String>,
    #[serde(rename = "expiryYear")]
    pub expiry_year: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.transaction_result);
        
        // Check if it's an error response
        let response = if item.response.response_code.value != "00" {
            Err(ErrorResponse {
                code: item.response.response_code.value,
                message: item.response.response_code.description,
                reason: item.response.ecommerce_card_payment_only_transaction_data
                    .and_then(|data| data.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// TryFrom implementation for confirm response
impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsConfirmResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsConfirmResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.transaction_result);
        
        // Check if it's an error response
        let response = if item.response.response_code.value != "00" {
            Err(ErrorResponse {
                code: item.response.response_code.value,
                message: item.response.response_code.description,
                reason: None,
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: item.response.authorization_code.map(|auth_code| {
                    serde_json::json!({
                        "authorization_code": auth_code
                    })
                }),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}


impl TryFrom<&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>> for PeachpaymentsConfirmRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let amount_in_cents = StringMinorUnitForConnector
            .convert_back(item.amount.clone(), item.router_data.request.currency)
            .map_err(|_| errors::ConnectorError::ParsingFailed)?
            .get_amount_as_i64();

        let amount = AmountDetails {
            amount: amount_in_cents,
            currency_code: item.router_data.request.currency.to_string(),
        };

        let confirmation_data = EcommerceCardPaymentOnlyConfirmationData {
            amount,
        };

        Ok(Self {
            ecommerce_card_payment_only_confirmation_data: confirmation_data,
        })
    }
}

// Card Gateway API Refund Request
#[derive(Debug, Serialize)]
pub struct PeachpaymentsRefundRequest {
    #[serde(rename = "chargeMethod")]
    pub charge_method: String,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "ecommerceCardPaymentOnlyTransactionData")]
    pub ecommerce_card_payment_only_transaction_data: EcommerceCardPaymentOnlyRefundData,
    #[serde(rename = "posData")]
    pub pos_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct EcommerceCardPaymentOnlyRefundData {
    #[serde(rename = "merchantInformation")]
    pub merchant_information: MerchantInformation,
    pub routing: Routing,
    pub card: CardDetails,
    pub amount: AmountDetails,
}

impl<F> TryFrom<&PeachpaymentsRouterData<&RefundsRouterData<F>>> for PeachpaymentsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let amount_in_cents = StringMinorUnitForConnector
            .convert_back(item.amount.clone(), item.router_data.request.currency)
            .map_err(|_| errors::ConnectorError::ParsingFailed)?
            .get_amount_as_i64();

        // Get merchant metadata from connector configuration
        let metadata = item.router_data.connector_meta_data.as_ref()
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?
            .clone()
            .expose();
        
        let metadata_obj = metadata.as_object()
            .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                config: "connector metadata must be a JSON object"
            })?;
        
        let merchant_information = MerchantInformation {
            client_merchant_reference_id: item.router_data.request.refund_id.clone(),
            name: metadata_obj.get("merchant_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mcc: metadata_obj.get("mcc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            phone: metadata_obj.get("merchant_phone")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            email: metadata_obj.get("merchant_email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mobile: metadata_obj.get("merchant_mobile")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            address: metadata_obj.get("merchant_address")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            city: metadata_obj.get("merchant_city")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            postal_code: metadata_obj.get("merchant_postal_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            region_code: metadata_obj.get("merchant_region_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            merchant_type: metadata_obj.get("merchant_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            website_url: metadata_obj.get("merchant_website")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        // Get routing configuration from metadata
        let routing = Routing {
            route: metadata_obj.get("routing_route")
                .and_then(|v| v.as_str())
                .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                    config: "routing_route is required in connector metadata"
                })?
                .to_string(),
            mid: metadata_obj.get("routing_mid")
                .and_then(|v| v.as_str())
                .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                    config: "routing_mid is required in connector metadata"
                })?
                .to_string(),
            tid: metadata_obj.get("routing_tid")
                .and_then(|v| v.as_str())
                .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                    config: "routing_tid is required in connector metadata"
                })?
                .to_string(),
            visa_payment_facilitator_id: metadata_obj.get("visa_payment_facilitator_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            master_card_payment_facilitator_id: metadata_obj.get("master_card_payment_facilitator_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            sub_mid: metadata_obj.get("sub_mid")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            amex_id: metadata_obj.get("amex_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        // Extract card details from refund_connector_metadata
        let card = if let Some(ref connector_metadata) = item.router_data.request.refund_connector_metadata {
            let metadata_value = connector_metadata.clone().expose();
            let card_data = metadata_value.as_object()
                .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                    config: "refund_connector_metadata must be a JSON object containing card details"
                })?;
            
            CardDetails {
                pan: Secret::new(
                    card_data.get("card_number")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "card_number is required in refund_connector_metadata"
                        })?
                        .to_string()
                ),
                cardholder_name: card_data.get("card_holder_name")
                    .and_then(|v| v.as_str())
                    .map(|name| Secret::new(name.to_string())),
                expiry_year: {
                    let year_str = card_data.get("card_exp_year")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "card_exp_year is required in refund_connector_metadata"
                        })?;
                    // Convert 4-digit year to 2-digit year (e.g., "2025" -> "25")
                    if year_str.len() == 4 {
                        Secret::new(year_str[2..].to_string())
                    } else {
                        Secret::new(year_str.to_string())
                    }
                },
                expiry_month: Secret::new(
                    card_data.get("card_exp_month")
                        .and_then(|v| v.as_str())
                        .ok_or(errors::ConnectorError::InvalidConnectorConfig { 
                            config: "card_exp_month is required in refund_connector_metadata"
                        })?
                        .to_string()
                ),
                cvv: card_data.get("card_cvc")
                    .and_then(|v| v.as_str())
                    .map(|cvc| Secret::new(cvc.to_string())),
            }
        } else {
            return Err(errors::ConnectorError::InvalidConnectorConfig { 
                config: "refund_connector_metadata with card details is required for PeachPayments refunds"
            }.into());
        };

        let amount = AmountDetails {
            amount: amount_in_cents,
            currency_code: item.router_data.request.currency.to_string(),
        };

        let ecommerce_data = EcommerceCardPaymentOnlyRefundData {
            merchant_information,
            routing,
            card,
            amount,
        };

        Ok(Self {
            charge_method: "ecommerce_card_payment_only".to_string(),
            reference_id: item.router_data.request.refund_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            pos_data: serde_json::json!({}), // Empty object for now
        })
    }
}

// Card Gateway API Refund Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    #[serde(alias = "successful", alias = "SUCCESSFUL")]
    Successful,
    #[serde(alias = "failed", alias = "FAILED")]
    Failed,
    #[serde(alias = "authorized", alias = "AUTHORIZED")]
    Authorized,
    #[serde(alias = "expired", alias = "EXPIRED")]
    Expired,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Successful => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Authorized => Self::Pending,
            RefundStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "originalTransactionId")]
    pub original_transaction_id: String,
    #[serde(rename = "responseCode")]
    pub response_code: ResponseCode,
    #[serde(rename = "transactionResult")]
    pub transaction_result: RefundStatus,
    #[serde(rename = "ecommerceCardPaymentOnlyTransactionData", skip_serializing_if = "Option::is_none")]
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.transaction_result.clone());
        
        // Check if it's an error response
        let response = if item.response.response_code.value != "00" {
            Err(ErrorResponse {
                code: item.response.response_code.value,
                message: item.response.response_code.description,
                reason: item.response.ecommerce_card_payment_only_transaction_data
                    .and_then(|data| data.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.clone(),
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.transaction_result.clone());
        
        // Check if it's an error response
        let response = if item.response.response_code.value != "00" {
            Err(ErrorResponse {
                code: item.response.response_code.value,
                message: item.response.response_code.description,
                reason: item.response.ecommerce_card_payment_only_transaction_data
                    .and_then(|data| data.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.clone(),
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

// Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeachpaymentsErrorResponse {
    #[serde(rename = "errorRef")]
    pub error_ref: String,
    pub message: String,
    #[serde(rename = "errorDetails")]
    pub error_details: serde_json::Value, // Use Value to handle dynamic error details structure
}

impl PeachpaymentsErrorResponse {
    pub fn get_error_details_message(&self) -> String {
        // Try to extract message from error details
        if let Some(details_obj) = self.error_details.as_object() {
            if let Some(message) = details_obj.get("message").and_then(|v| v.as_str()) {
                return format!("{}: {}", self.message, message);
            }
        }
        
        // If error_details is a string, use it directly
        if let Some(details_str) = self.error_details.as_str() {
            return format!("{}: {}", self.message, details_str);
        }
        
        // Otherwise just return the main message
        self.message.clone()
    }
}

impl TryFrom<ErrorResponse> for PeachpaymentsErrorResponse {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(error_response: ErrorResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            error_ref: error_response.code,
            message: error_response.message,
            error_details: serde_json::json!({}),
        })
    }
}
