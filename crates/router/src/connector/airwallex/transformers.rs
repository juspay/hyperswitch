use masking::PeekInterface;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{core::errors,types::{self,api, storage::enums}};
use time::{PrimitiveDateTime};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexIntentRequest {
    request_id: String,
    amount: i64,
    currency: enums::Currency,
    merchant_order_id: String,
}
impl TryFrom<&types::PaymentsPreAuthorizeRouterData> for AirwallexIntentRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsPreAuthorizeRouterData) -> Result<Self,Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: item.request.amount,
            currency: item.request.currency,
            merchant_order_id: Uuid::new_v4().to_string(),
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsRequest {
    request_id: String,
    payment_method: AirwallexPaymentMethod,
    payment_method_options: Option<AirwallexPaymentOptions>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentOptions{
    auto_capture: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum AirwallexPaymentMethod {
    Card(AirwallexCard)
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCard{
    card: AirwallexCardDetails,
    #[serde(rename="type")]
    payment_method_type: AirwallexPaymentType,
}
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCardDetails{
    expiry_month: String,
    expiry_year: String,
    number: String,
    cvc: String,
    
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all="snake_case")]
pub enum AirwallexPaymentType {
    Card,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for AirwallexPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethod::Card(ccard) => Ok(AirwallexPaymentMethod::Card(AirwallexCard { 
                card: AirwallexCardDetails{
                    number: ccard.card_number.peek().to_string(),
                    expiry_month: ccard.card_exp_month.peek().to_string(),
                    expiry_year: ccard.card_exp_year.peek().to_string(),
                    cvc: ccard.card_cvc.peek().to_string(),
                },
                payment_method_type: AirwallexPaymentType::Card
            })),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        let payment_method_options = Some(AirwallexPaymentOptions{
            auto_capture: matches!(
                item.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            ),
        });
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            payment_method,
            payment_method_options,
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AirwallexAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for AirwallexAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Deserialize)]
pub struct AirwallexAuthUpdateResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    expires_at: PrimitiveDateTime,
    token: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        let expires = (common_utils::date_time::now() - item.response.expires_at).whole_seconds();
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.token,
                expires
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCaptureRequest {
    request_id: String,
    amount: Option<i64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AirwallexPaymentsCaptureRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self,Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: item.request.amount_to_capture,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCancelRequest {
    request_id: String,
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for AirwallexPaymentsCancelRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self,Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}


// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresPaymentMethod,
    RequiresCustomerAction,
    RequiresCapture,
    Cancelled,
}

impl From<AirwallexPaymentStatus> for enums::AttemptStatus {
    fn from(item: AirwallexPaymentStatus) -> Self {
        match item {
            AirwallexPaymentStatus::Succeeded => Self::Charged,
            AirwallexPaymentStatus::Failed => Self::Failure,
            AirwallexPaymentStatus::Pending => Self::Pending,
            AirwallexPaymentStatus::RequiresPaymentMethod => Self::PaymentMethodAwaited,
            AirwallexPaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            AirwallexPaymentStatus::RequiresCapture => Self::Authorized,
            AirwallexPaymentStatus::Cancelled => Self::Voided,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirwallexPaymentsResponse {
    status: AirwallexPaymentStatus,
    id: String,
    amount: Option<i64>,
    payment_consent_id: Option<String>,

}

impl<F,T> TryFrom<types::ResponseRouterData<F, AirwallexPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, AirwallexPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            reference_id: Some(item.response.id.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
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
pub struct AirwallexRefundRequest {
    request_id: String,
    amount: Option<i64>,
    reason: Option<String>,
    payment_intent_id: String

}

impl<F> TryFrom<&types::RefundsRouterData<F>> for AirwallexRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(item.request.refund_amount),
            reason: item.request.reason.clone(),
            payment_intent_id: item.request.connector_transaction_id.clone(),
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
    Received,
    Accepted,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Received | RefundStatus::Accepted => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    acquirer_reference_number: String,
    amount: i64,
    id: String,
    status: RefundStatus,

}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AirwallexErrorResponse {
   pub code: String,
   pub message: String,
   pub details: Option<Vec<String>>,
   pub source: Option<String>
}
