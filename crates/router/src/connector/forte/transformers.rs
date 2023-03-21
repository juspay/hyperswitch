use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{core::errors,types::{self,api, storage::enums}};

// Payment Request Flow
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct FortePaymentsRequest {
    authorization_amount: f64,
    billing_address:BillingAddress,
    card: ForteCard
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct BillingAddress{
    first_name:String,
    last_name:String,
}

#[derive(Default,Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    card_type: Option<api_models::enums::CardNetwork>,
    name_on_card: Secret<String>,
    account_number: Secret<String, common_utils::pii::CardNumber>,
    expire_month: Secret<String>,
    expire_year: Secret<String>,
    card_verification_value: Secret<String>,
}

// Implementing for PaymentsRequest
impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = ForteCard {
                    card_type: req_card.card_network,
                    name_on_card: req_card.card_holder_name,
                    account_number: req_card.card_number,
                    expire_month: req_card.card_exp_month,
                    expire_year: req_card.card_exp_year,
                    card_verification_value: req_card.card_cvc,
                };
                let buildadd=BillingAddress{
                    first_name:String::from("Test"),
                    last_name:String::from("Forte"),
                };
                Ok(Self {

                    authorization_amount: item.request.amount as f64,
                    billing_address:buildadd,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }    
    }
}

//Authentication Flow
pub struct ForteAuthType {
    pub(super) api_id: String,
    pub(super) api_key: String,
    
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_string(),
                api_id: key1.to_string()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Payment Status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum FortePaymentStatus {
    Authorized,
    Captured,
    Voided,
    #[default]
    Ready
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Voided => Self::Voided,
            FortePaymentStatus::Authorized => Self::Authorized,
            FortePaymentStatus::Captured => Self::Charged,
            FortePaymentStatus::Ready => Self::Started,
        }
    }
}

//Payments Response Flow
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    location_id: String,
    action: String,
    authorization_amount: f64,
    authorization_code: String,
    entered_by: String,
    billing_address: BillingAddress,
    card: ForteCard,
    response: Response,
    links: Links,
    status: FortePaymentStatus,
    response_type: String,
    response_code: String,
    transaction_id: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Links {
    pub disputes: String,
    pub settlements: String,
    #[serde(rename = "self")]
    pub self_data: String,
}

#[derive(Default, Debug,Clone, Serialize, Eq, PartialEq,Deserialize)]
pub struct Response {
    pub environment: String,
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String,
    pub avs_result: String,
    pub cvv_result: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}


// Response Flow

// Sync Response
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsSyncResponse {
    pub transaction_id: String,
    pub organization_id: String,
    pub location_id: String,
    pub status: String,
    pub action: String,
    pub authorization_amount: i64,
    pub authorization_code: String,
    pub entered_by:String,
    pub received_date: String,
    pub billing_address: BillingAddress,
    pub attempt_number:i64,
    pub biller_name:String,
    pub card: ForteCard,
    pub response: Response,
    pub links: Links,
}

// Capture Response
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsCaptureRequest {
    pub action: String,
    pub transaction_id: String,
    pub authorization_amount: f64,
    pub authorization_code: String,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsCaptureResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_amount: f64,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: Response2,
}


// PaymentVoid 
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsVoidRequest {
    pub action: String,
    pub transaction_id: String,
    pub authorization_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsVoidResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_amount: f64,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: Response2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response2 {
    pub environment: String,
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String,
}

// Refund _________________________________________________________________________________________________________

#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    pub authorization_amount: i64,
    pub original_transaction_id: Secret<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            authorization_amount: item.request.refund_amount,
            original_transaction_id: Secret::new(item.request.connector_transaction_id.clone()), 
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Complete,
    Failed,
    Declined,
    #[default]
    Settling,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Complete => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Settling => Self::Pending,
            RefundStatus::Declined => Self::TransactionFailure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    status: RefundStatus,
    response_type: String,
    response_code: String,
    transaction_id: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}