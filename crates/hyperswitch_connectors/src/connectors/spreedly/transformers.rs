use common_enums::enums;
use common_utils::{ext_traits::ValueExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
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
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    name: Option<Secret<String>>,
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
                let credit_card = SpreedlyCreditCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    name: req_card.card_holder_name.clone(),
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&SpreedlyRouterData<&RefundsRouterData<F>>> for SpreedlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SpreedlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
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
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
