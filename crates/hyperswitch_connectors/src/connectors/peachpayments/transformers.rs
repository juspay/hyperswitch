use common_enums::enums;
use common_utils::types::{StringMinorUnit, StringMinorUnitForConnector, AmountConvertor};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
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
    #[serde(rename = "paymentMethod")]
    pub payment_method: String,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "ecommerceCardPaymentOnlyTransactionData")]
    pub ecommerce_card_payment_only_transaction_data: EcommerceCardPaymentOnlyTransactionData,
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
    pub name: String,
    pub mcc: String,
    pub phone: String,
    pub email: String,
    pub mobile: String,
    pub address: String,
    pub city: String,
    #[serde(rename = "postalCode")]
    pub postal_code: String,
    #[serde(rename = "regionCode")]
    pub region_code: String,
    #[serde(rename = "merchantType")]
    pub merchant_type: String,
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
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CardDetails {
    pub pan: Secret<String>,
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Secret<String>,
    #[serde(rename = "expiryYear")]
    pub expiry_year: Secret<String>,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AmountDetails {
    pub amount: i64,
    #[serde(rename = "currencyCode")]
    pub currency_code: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ThreeDSData {
    pub cavv: Secret<String>,
    pub eci: String,
    #[serde(rename = "dsTransId")]
    pub ds_trans_id: String,
    #[serde(rename = "authenticationStatus")]
    pub authentication_status: String,
    #[serde(rename = "threeDSVersion")]
    pub     three_ds_version: String,
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
                        .unwrap_or("Default Merchant")
                        .to_string(),
                    mcc: metadata_obj.get("mcc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("5411")
                        .to_string(),
                    phone: metadata_obj.get("merchant_phone")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1234567890")
                        .to_string(),
                    email: metadata_obj.get("merchant_email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("merchant@example.com")
                        .to_string(),
                    mobile: metadata_obj.get("merchant_mobile")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1234567890")
                        .to_string(),
                    address: metadata_obj.get("merchant_address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("123 Main Street")
                        .to_string(),
                    city: metadata_obj.get("merchant_city")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Default City")
                        .to_string(),
                    postal_code: metadata_obj.get("merchant_postal_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("12345")
                        .to_string(),
                    region_code: metadata_obj.get("merchant_region_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("WC")
                        .to_string(),
                    merchant_type: metadata_obj.get("merchant_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("iso")
                        .to_string(),
                    website_url: metadata_obj.get("merchant_website")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                // Get routing configuration from metadata
                let routing = Routing {
                    route: metadata_obj.get("routing_route")
                        .and_then(|v| v.as_str())
                        .unwrap_or("exipay_emulator")
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
                };

                let card = CardDetails {
                    pan: Secret::new(req_card.card_number.to_string()),
                    cardholder_name: req_card.card_holder_name.unwrap_or_default(),
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
                    cvv: req_card.card_cvc.clone(),
                };

                let amount = AmountDetails {
                    amount: amount_in_cents,
                    currency_code: item.router_data.request.currency.to_string(),
                };

                // Extract 3DS data if available
                let three_ds_data = item.router_data.request.authentication_data.as_ref().and_then(|auth_data| {
                    // Only include 3DS data if we have the essential fields
                    if auth_data.eci.is_some() || auth_data.ds_trans_id.is_some() {
                        Some(ThreeDSData {
                            cavv: auth_data.cavv.clone(),
                            eci: auth_data.eci.clone().unwrap_or_else(|| "07".to_string()), // Default ECI for 3DS authenticated
                            ds_trans_id: auth_data.ds_trans_id.clone()
                                .or_else(|| auth_data.threeds_server_transaction_id.clone())
                                .unwrap_or_default(),
                            authentication_status: if auth_data.eci.is_some() { "Y".to_string() } else { "A".to_string() },
                            three_ds_version: auth_data.message_version
                                .as_ref()
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "2.0".to_string()),
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

                Ok(Self {
                    payment_method: "ecommerce_card_payment_only".to_string(),
                    reference_id: item.router_data.connector_request_reference_id.clone(),
                    ecommerce_card_payment_only_transaction_data: ecommerce_data,
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
#[serde(rename_all = "lowercase")]
pub enum PeachpaymentsPaymentStatus {
    Successful,
    Authorized,
    Failed,
    Expired,
    Refunded,
}

impl From<PeachpaymentsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PeachpaymentsPaymentStatus) -> Self {
        match item {
            PeachpaymentsPaymentStatus::Successful => Self::Charged,
            PeachpaymentsPaymentStatus::Authorized => Self::Authorized,
            PeachpaymentsPaymentStatus::Failed => Self::Failure,
            PeachpaymentsPaymentStatus::Expired => Self::Failure,
            PeachpaymentsPaymentStatus::Refunded => Self::Voided,
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
        let response = if item.response.response_code.value != "00" && item.response.response_code.value != "08" {
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

// Card Gateway API Refund Request
#[derive(Debug, Serialize)]
pub struct PeachpaymentsRefundRequest {
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
                .unwrap_or("Default Merchant")
                .to_string(),
            mcc: metadata_obj.get("mcc")
                .and_then(|v| v.as_str())
                .unwrap_or("5411")
                .to_string(),
            phone: metadata_obj.get("merchant_phone")
                .and_then(|v| v.as_str())
                .unwrap_or("1234567890")
                .to_string(),
            email: metadata_obj.get("merchant_email")
                .and_then(|v| v.as_str())
                .unwrap_or("merchant@example.com")
                .to_string(),
            mobile: metadata_obj.get("merchant_mobile")
                .and_then(|v| v.as_str())
                .unwrap_or("1234567890")
                .to_string(),
            address: metadata_obj.get("merchant_address")
                .and_then(|v| v.as_str())
                .unwrap_or("123 Main Street")
                .to_string(),
            city: metadata_obj.get("merchant_city")
                .and_then(|v| v.as_str())
                .unwrap_or("Default City")
                .to_string(),
            postal_code: metadata_obj.get("merchant_postal_code")
                .and_then(|v| v.as_str())
                .unwrap_or("12345")
                .to_string(),
            region_code: metadata_obj.get("merchant_region_code")
                .and_then(|v| v.as_str())
                .unwrap_or("WC")
                .to_string(),
            merchant_type: metadata_obj.get("merchant_type")
                .and_then(|v| v.as_str())
                .unwrap_or("iso")
                .to_string(),
            website_url: metadata_obj.get("merchant_website")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        // Get routing configuration from metadata
        let routing = Routing {
            route: metadata_obj.get("routing_route")
                .and_then(|v| v.as_str())
                .unwrap_or("exipay_emulator")
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
                cardholder_name: Secret::new(
                    card_data.get("card_holder_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Card Holder")
                        .to_string()
                ),
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
                cvv: Secret::new(
                    card_data.get("card_cvc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("999")  // CVV might not be stored for security reasons
                        .to_string()
                ),
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
