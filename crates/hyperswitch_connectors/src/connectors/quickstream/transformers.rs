use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::StringExt,
    types::StringMajorUnit,
};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

// QuickStream router data wrapper for amount conversion
pub struct QuickstreamRouterData<T> {
    pub amount: StringMajorUnit, // QuickStream accepts major currency units (dollars)
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for QuickstreamRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// QuickStream payment request structure based on actual API
#[derive(Debug, Serialize, PartialEq)]
pub struct QuickstreamPaymentsRequest {
    #[serde(rename = "transactionType")]
    pub transaction_type: String, // "PAYMENT"
    #[serde(rename = "creditCard", skip_serializing_if = "Option::is_none")]
    pub credit_card: Option<QuickstreamCreditCard>,
    #[serde(rename = "supplierBusinessCode")]
    pub supplier_business_code: Secret<String>,
    #[serde(rename = "principalAmount")]
    pub principal_amount: f64,
    pub currency: String, // "AUD"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>, // "INTERNET", "PHONE", "MAIL", "RECURRING"
    #[serde(rename = "ipAddress", skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<Secret<String>>,
    #[serde(rename = "customerReferenceNumber", skip_serializing_if = "Option::is_none")]
    pub customer_reference_number: Option<String>,
    #[serde(rename = "paymentReferenceNumber", skip_serializing_if = "Option::is_none")]
    pub payment_reference_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "storedCredentialData", skip_serializing_if = "Option::is_none")]
    pub stored_credential_data: Option<QuickstreamStoredCredentialData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct QuickstreamCreditCard {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Secret<String>,
    #[serde(rename = "cardNumber")]
    pub card_number: Secret<String>,
    #[serde(rename = "expiryDateMonth")]
    pub expiry_date_month: Secret<String>,
    #[serde(rename = "expiryDateYear")]
    pub expiry_date_year: Secret<String>,
    pub cvn: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct QuickstreamStoredCredentialData {
    #[serde(rename = "entryMode")]
    pub entry_mode: String, // "MANUAL", "STORED"
}

impl TryFrom<&QuickstreamRouterData<&PaymentsAuthorizeRouterData>> for QuickstreamPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &QuickstreamRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = item.router_data;
        
        // Convert amount from string to f64
        let principal_amount: f64 = item.amount.parse_value("f64").unwrap_or(0.0);
        
        // Get supplier business code from connector metadata or auth
        let supplier_business_code = get_supplier_business_code(router_data)?;
        
        match router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let credit_card = QuickstreamCreditCard {
                    cardholder_name: req_card.card_holder_name.unwrap_or(Secret::new("".to_string())),
                    card_number: req_card.card_number,
                    expiry_date_month: req_card.card_exp_month,
                    expiry_date_year: req_card.card_exp_year,
                    cvn: req_card.card_cvc,
                };
                
                Ok(Self {
                    transaction_type: "PAYMENT".to_string(),
                    credit_card: Some(credit_card),
                    supplier_business_code,
                    principal_amount,
                    currency: router_data.request.currency.to_string(),
                    eci: Some("INTERNET".to_string()), // Default to internet transaction
                    ip_address: router_data.request.get_optional_ip(),
                    customer_reference_number: router_data.request.connector_customer.clone(),
                    payment_reference_number: Some(router_data.connector_request_reference_id.clone()),
                    comment: None,
                    stored_credential_data: Some(QuickstreamStoredCredentialData {
                        entry_mode: "MANUAL".to_string(),
                    }),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Helper function to get supplier business code from connector metadata
fn get_supplier_business_code(
    router_data: &PaymentsAuthorizeRouterData,
) -> Result<Secret<String>, error_stack::Report<errors::ConnectorError>> {
    // Try to get from connector metadata first
    if let Some(metadata) = &router_data.connector_meta_data {
        if let Ok(metadata_obj) = metadata.parse_value::<serde_json::Value>("json") {
            if let Some(supplier_code) = metadata_obj.get("supplier_business_code") {
                if let Some(code_str) = supplier_code.as_str() {
                    return Ok(Secret::new(code_str.to_string()));
                }
            }
        }
    }
    
    // Default fallback - should be provided in merchant connector account metadata
    Err(errors::ConnectorError::MissingRequiredField { 
        field_name: "supplier_business_code" 
    }.into())
}

// QuickStream authentication structure
pub struct QuickstreamAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for QuickstreamAuthType {
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
// QuickStream payment response structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamPaymentsResponse {
    pub links: Vec<QuickstreamLink>,
    #[serde(rename = "receiptNumber")]
    pub receipt_number: String,
    #[serde(rename = "principalAmount")]
    pub principal_amount: QuickstreamAmount,
    #[serde(rename = "surchargeAmount")]
    pub surcharge_amount: QuickstreamAmount,
    #[serde(rename = "totalAmount")]
    pub total_amount: QuickstreamAmount,
    pub status: String, // "Approved", "Declined", "Error"
    #[serde(rename = "responseCode")]
    pub response_code: String,
    #[serde(rename = "responseDescription")]
    pub response_description: String,
    #[serde(rename = "summaryCode")]
    pub summary_code: String,
    #[serde(rename = "transactionType")]
    pub transaction_type: String,
    #[serde(rename = "fraudGuardResult")]
    pub fraud_guard_result: Option<String>,
    #[serde(rename = "transactionTime")]
    pub transaction_time: String,
    #[serde(rename = "settlementDate")]
    pub settlement_date: String,
    pub source: String,
    pub voidable: bool,
    pub refundable: bool,
    #[serde(rename = "creditCard")]
    pub credit_card: Option<QuickstreamCreditCardResponse>,
    #[serde(rename = "merchantAccount")]
    pub merchant_account: QuickstreamMerchantAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamAmount {
    pub currency: String,
    pub amount: f64,
    #[serde(rename = "displayAmount")]
    pub display_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamLink {
    pub rel: String,
    pub href: String,
    #[serde(rename = "requestMethod")]
    pub request_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamCreditCardResponse {
    #[serde(rename = "accountType")]
    pub account_type: String,
    #[serde(rename = "accountToken")]
    pub account_token: Option<String>,
    #[serde(rename = "customerId")]
    pub customer_id: Option<String>,
    #[serde(rename = "defaultAccount")]
    pub default_account: bool,
    #[serde(rename = "cardNumber")]
    pub card_number: String,
    #[serde(rename = "expiryDateMonth")]
    pub expiry_date_month: String,
    #[serde(rename = "expiryDateYear")]
    pub expiry_date_year: String,
    #[serde(rename = "cardScheme")]
    pub card_scheme: String,
    #[serde(rename = "cardholderName")]
    pub cardholder_name: String,
    #[serde(rename = "cardType")]
    pub card_type: String,
    #[serde(rename = "maskedCardNumber4Digits")]
    pub masked_card_number_4_digits: String,
    #[serde(rename = "walletProvider")]
    pub wallet_provider: Option<String>,
    #[serde(rename = "panType")]
    pub pan_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamMerchantAccount {
    #[serde(rename = "merchantId")]
    pub merchant_id: String,
    #[serde(rename = "merchantName")]
    pub merchant_name: String,
    #[serde(rename = "settlementBsb")]
    pub settlement_bsb: String,
    #[serde(rename = "settlementAccountNumber")]
    pub settlement_account_number: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "acquiringInstitution")]
    pub acquiring_institution: String,
    pub currency: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, QuickstreamPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, QuickstreamPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.status.as_str() {
            "Approved" => enums::AttemptStatus::Charged,
            "Declined" => enums::AttemptStatus::Failure,
            "Error" => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        };

        // For void/cancel operations, use Voided status when approved
        let attempt_status = if std::any::type_name::<T>().contains("PaymentsCancel") {
            match item.response.status.as_str() {
                "Approved" => enums::AttemptStatus::Voided,
                "Declined" => enums::AttemptStatus::VoidFailed,
                "Error" => enums::AttemptStatus::VoidFailed,
                _ => enums::AttemptStatus::Pending,
            }
        } else {
            status
        };

        Ok(Self {
            status: attempt_status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.receipt_number.clone()),
                redirection_data: Box::new(None), // QuickStream doesn't have redirect flows
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.receipt_number),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// QuickStream refund request structure
#[derive(Debug, Serialize)]
pub struct QuickstreamRefundRequest {
    #[serde(rename = "refundAmount", skip_serializing_if = "Option::is_none")]
    pub refund_amount: Option<f64>, // For partial refunds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl<F> TryFrom<&QuickstreamRouterData<&RefundsRouterData<F>>> for QuickstreamRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &QuickstreamRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let refund_amount: f64 = item.amount.parse_value("f64").unwrap_or(0.0);
        
        Ok(Self {
            refund_amount: Some(refund_amount),
            comment: None,
        })
    }
}

// QuickStream refund response structure (uses same format as payment response)
pub type QuickstreamRefundResponse = QuickstreamPaymentsResponse;
pub type RefundResponse = QuickstreamRefundResponse;

impl TryFrom<RefundsResponseRouterData<Execute, QuickstreamRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, QuickstreamRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            "Approved" => enums::RefundStatus::Success,
            "Declined" => enums::RefundStatus::Failure,
            "Error" => enums::RefundStatus::Failure,
            _ => enums::RefundStatus::Pending,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.receipt_number.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, QuickstreamRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, QuickstreamRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            "Approved" => enums::RefundStatus::Success,
            "Declined" => enums::RefundStatus::Failure,
            "Error" => enums::RefundStatus::Failure,
            _ => enums::RefundStatus::Pending,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.receipt_number.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

// QuickStream error response structure
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct QuickstreamErrorResponse {
    pub status: String,
    #[serde(rename = "responseCode")]
    pub code: String,
    #[serde(rename = "responseDescription")]
    pub message: String,
    #[serde(rename = "summaryCode")]
    pub summary_code: String,
    pub reason: Option<String>,
}
