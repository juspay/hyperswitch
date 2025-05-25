use common_enums::enums;
use common_utils::{ext_traits::ValueExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

// Common Spreedly structures
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Amount {
    pub value: StringMinorUnit,
    pub currency: String,
}

//TODO: Fill the struct with respective fields
pub struct SpreedlyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for SpreedlyRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Transaction structure for Spreedly payment requests
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SpreedlyTransaction {
    pub credit_card: SpreedlyCreditCard,
    pub amount: StringMinorUnit,
    pub currency_code: String,
}

// Payment request structure
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SpreedlyPaymentsRequest {
    pub transaction: SpreedlyTransaction,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SpreedlyCreditCard {
    number: cards::CardNumber,
    verification_value: Secret<String>,
    month: Secret<String>,
    year: Secret<String>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    complete: bool,
}

// Helper function to extract gateway token from connector metadata
pub fn get_gateway_token(
    connector_meta: &Option<common_utils::pii::SecretSerdeValue>,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let metadata = connector_meta
        .as_ref()
        .ok_or(errors::ConnectorError::InvalidConnectorConfig {
            config: "metadata",
        })?;
    
    let parsed_metadata = metadata
        .clone()
        .parse_value::<serde_json::Value>("ConnectorMetadata")
        .change_context(errors::ConnectorError::InvalidConnectorConfig {
            config: "metadata",
        })?;
    
    let gateway_token = parsed_metadata
        .get("gateway_token")
        .and_then(|token| token.as_str())
        .ok_or(errors::ConnectorError::InvalidConnectorConfig {
            config: "gateway_token",
        })?;
    
    Ok(gateway_token.to_string())
}

impl TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeRouterData>> for SpreedlyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Split cardholder name into first and last name
                let (first_name, last_name) = match req_card.card_holder_name.as_ref() {
                    Some(full_name) => {
                        let name = full_name.peek();
                        let parts: Vec<&str> = name.split_whitespace().collect();
                        match parts.len() {
                            0 => (None, None),
                            1 => (Some(Secret::new(parts[0].to_string())), None),
                            _ => {
                                let first = parts[0].to_string();
                                let last = parts[1..].join(" ");
                                (Some(Secret::new(first)), Some(Secret::new(last)))
                            }
                        }
                    }
                    None => (None, None),
                };
                
                let credit_card = SpreedlyCreditCard {
                    number: req_card.card_number,
                    verification_value: req_card.card_cvc,
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year,
                    first_name,
                    last_name,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                
                let transaction = SpreedlyTransaction {
                    credit_card,
                    amount: item.amount.clone(),
                    currency_code: item.router_data.request.currency.to_string(),
                };
                
                Ok(Self { transaction })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct SpreedlyAuthType {
    pub(super) environment_key: Secret<String>,
    pub(super) access_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                // Parse "environment_key:access_secret" format
                let api_key_str = api_key.peek();
                let parts: Vec<&str> = api_key_str.split(':').collect();
                
                if parts.len() != 2 {
                    return Err(errors::ConnectorError::FailedToObtainAuthType.into());
                }
                
                Ok(Self {
                    environment_key: Secret::new(parts[0].to_string()),
                    access_secret: Secret::new(parts[1].to_string()),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpreedlyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    Pending,
    Voided,
    Declined,
    Authorized,
}

impl From<SpreedlyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: SpreedlyPaymentStatus) -> Self {
        match item {
            SpreedlyPaymentStatus::Succeeded => Self::Charged,
            SpreedlyPaymentStatus::Failed => Self::Failure,
            SpreedlyPaymentStatus::Processing => Self::Authorizing,
            SpreedlyPaymentStatus::Pending => Self::Pending,
            SpreedlyPaymentStatus::Voided => Self::Voided,
            SpreedlyPaymentStatus::Declined => Self::Failure,
            SpreedlyPaymentStatus::Authorized => Self::Authorized,
        }
    }
}

// Transaction response structure
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyTransactionResponse {
    pub token: String,
    pub state: SpreedlyPaymentStatus,
    pub payment_method: Option<SpreedlyPaymentMethod>,
    pub amount: Option<StringMinorUnit>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyPaymentMethod {
    pub token: String,
    pub storage_state: String,
}

// Payment response structure with transaction field
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyPaymentsResponse {
    pub transaction: SpreedlyTransactionResponse,
}

impl<F, T> TryFrom<ResponseRouterData<F, SpreedlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SpreedlyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction.state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.token.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// CAPTURE :
// Type definition for CaptureRequest
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyCaptureRequest {
    pub transaction: SpreedlyCaptureTransaction,
}

#[derive(Default, Debug, Serialize)]
pub struct SpreedlyCaptureTransaction {
    pub amount: StringMinorUnit,
    pub currency_code: String,
}

impl TryFrom<&SpreedlyRouterData<&PaymentsCaptureRouterData>> for SpreedlyCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction = SpreedlyCaptureTransaction {
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
        };
        
        Ok(Self { transaction })
    }
}

// Type alias for capture response - reuses the same structure as payment response
pub type SpreedlyCaptureResponse = SpreedlyPaymentsResponse;

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    pub transaction: SpreedlyRefundTransaction,
}

#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundTransaction {
    pub amount: StringMinorUnit,
    pub currency_code: String,
}

impl<F> TryFrom<&SpreedlyRouterData<&RefundsRouterData<F>>> for SpreedlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SpreedlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let transaction = SpreedlyRefundTransaction {
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
        };
        
        Ok(Self { transaction })
    }
}

// Type definition for Refund Response

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SpreedlyRefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    Pending,
}

impl From<SpreedlyRefundStatus> for enums::RefundStatus {
    fn from(item: SpreedlyRefundStatus) -> Self {
        match item {
            SpreedlyRefundStatus::Succeeded => Self::Success,
            SpreedlyRefundStatus::Failed => Self::Failure,
            SpreedlyRefundStatus::Processing => Self::Pending,
            SpreedlyRefundStatus::Pending => Self::Pending,
        }
    }
}

// Refund transaction response structure
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyRefundTransactionResponse {
    pub token: String,
    pub state: SpreedlyRefundStatus,
    pub amount: Option<StringMinorUnit>,
}

// Refund response structure with transaction field
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyRefundResponse {
    pub transaction: SpreedlyRefundTransactionResponse,
}

impl TryFrom<RefundsResponseRouterData<Execute, SpreedlyRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, SpreedlyRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.token.clone(),
                refund_status: enums::RefundStatus::from(item.response.transaction.state.clone()),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, SpreedlyRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, SpreedlyRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.token.clone(),
                refund_status: enums::RefundStatus::from(item.response.transaction.state.clone()),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyErrorResponse {
    pub errors: Vec<SpreedlyError>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyError {
    pub attribute: Option<String>,
    pub key: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperswitch_domain_models::router_data::ConnectorAuthType;
    use masking::Secret;

    #[test]
    fn test_spreedly_auth_type_parsing() {
        // Test successful parsing
        let auth_type = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("env_key_123:secret_456".to_string()),
        };
        
        let result = SpreedlyAuthType::try_from(&auth_type);
        assert!(result.is_ok());
        
        let auth = result.unwrap();
        assert_eq!(auth.environment_key.peek(), "env_key_123");
        assert_eq!(auth.access_secret.peek(), "secret_456");
    }
    
    #[test]
    fn test_spreedly_auth_type_parsing_failure() {
        // Test parsing failure - no colon separator
        let auth_type = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("invalid_format".to_string()),
        };
        
        let result = SpreedlyAuthType::try_from(&auth_type);
        assert!(result.is_err());
        
        // Test parsing failure - wrong auth type
        let auth_type = ConnectorAuthType::BodyKey {
            api_key: Secret::new("key".to_string()),
            key1: Secret::new("value".to_string()),
        };
        
        let result = SpreedlyAuthType::try_from(&auth_type);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_payment_status_mapping() {
        use common_enums::AttemptStatus;
        
        // Test all payment status mappings
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Succeeded),
            AttemptStatus::Charged
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Failed),
            AttemptStatus::Failure
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Processing),
            AttemptStatus::Authorizing
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Pending),
            AttemptStatus::Pending
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Voided),
            AttemptStatus::Voided
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Declined),
            AttemptStatus::Failure
        );
        assert_eq!(
            AttemptStatus::from(SpreedlyPaymentStatus::Authorized),
            AttemptStatus::Authorized
        );
    }
    
    #[test]
    fn test_refund_status_mapping() {
        // Test all refund status mappings
        assert_eq!(
            enums::RefundStatus::from(SpreedlyRefundStatus::Succeeded),
            enums::RefundStatus::Success
        );
        assert_eq!(
            enums::RefundStatus::from(SpreedlyRefundStatus::Failed),
            enums::RefundStatus::Failure
        );
        assert_eq!(
            enums::RefundStatus::from(SpreedlyRefundStatus::Processing),
            enums::RefundStatus::Pending
        );
        assert_eq!(
            enums::RefundStatus::from(SpreedlyRefundStatus::Pending),
            enums::RefundStatus::Pending
        );
    }
    
    #[test]
    fn test_amount_conversion() {
        use common_utils::types::MinorUnit;
        
        // Test SpreedlyRouterData conversion
        let minor_amount = MinorUnit::new(1099); // $10.99
        let test_data = "test_data";
        
        // Convert to StringMinorUnit for testing
        let string_minor_unit = StringMinorUnit::new(minor_amount.get_amount_as_i64());
        let router_data = SpreedlyRouterData::from((string_minor_unit.clone(), test_data));
        
        assert_eq!(router_data.amount, string_minor_unit);
        assert_eq!(router_data.router_data, &test_data);
        
        // Test Amount struct
        let amount = Amount {
            value: StringMinorUnit::new(2500), // $25.00
            currency: "USD".to_string(),
        };
        
        assert_eq!(amount.value, StringMinorUnit::new(2500));
        assert_eq!(amount.currency, "USD");
    }
    
    #[test]
    fn test_gateway_token_extraction() {
        use common_utils::pii::SecretSerdeValue;
        
        // Test successful extraction
        let metadata_json = serde_json::json!({
            "gateway_token": "test_gateway_token_123"
        });
        let metadata = Some(SecretSerdeValue::from(metadata_json.to_string()));
        
        let result = get_gateway_token(&metadata);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_gateway_token_123");
        
        // Test missing gateway_token
        let metadata_json = serde_json::json!({
            "other_field": "value"
        });
        let metadata = Some(SecretSerdeValue::from(metadata_json.to_string()));
        
        let result = get_gateway_token(&metadata);
        assert!(result.is_err());
        
        // Test None metadata
        let result = get_gateway_token(&None);
        assert!(result.is_err());
    }
}
