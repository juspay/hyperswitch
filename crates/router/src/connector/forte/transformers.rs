use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{connector::utils::PaymentsAuthorizeRequestData,core::errors,types::{self,api, storage::enums}};
use crate::types::PaymentsCaptureRouterData;
use crate::types::PaymentsAuthorizeRouterData;
use api_models::enums::PaymentMethod::Card;
use crate::connector::utils::missing_field_err;


//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FortePaymentsRequest {
    pub authorization_amount: i64,
    pub subtotal_amount: i64,
    pub billing_address: BillingAddress,
    pub card: ForteCard
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BillingAddress{
    pub first_name: String,
    pub last_name: String
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    pub card_type: String,
    pub name_on_card: String,
    pub account_number: String,
    pub expire_month: String,
    pub expire_year: String,
    pub card_verification_value: String
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCardResponse {
    pub card_type: String,
    pub name_on_card: String,
    pub account_number: String,
    pub expire_month: String,
    pub expire_year: String,
    pub last_4_account_number:String,
    pub masked_account_number:String,
}


impl TryFrom<&PaymentsCaptureRouterData> for PaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(value: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let amount = value.request.amount as i64;
        Ok(Self {
            action: String::from("capture"),
            transaction_id: value.request.connector_transaction_id.clone(),
            authorization_amount: amount,
            authorization_code: String::from("OSF381"),
        })
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for FortePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(value: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match value.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ref ccard) => {
            let request = FortePaymentsRequest{
                billing_address: BillingAddress{
                    first_name: String,
                    last_name: String,
                },
                card: ForteCard{
                    card_type: String::from("visa"),
                    name_on_card: ccard.card_holder_name.clone(),
                    account_number: ccard.card_number.clone(),
                    expire_month: ccard.card_exp_month.clone(),
                    expire_year: ccard.card_exp_year.clone(),
                    card_verification_value: ccard.card_cvc.clone(),
                },
                authorization_amount: value.request.amount as i64,
                subtotal_amount: value.request.amount as i64,
            };
            Ok(request)
        }
        _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
    }
}
}

impl TryFrom<&types::PaymentsCancelRouterData> for PaymentsVoidRequest{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            action:String::from("void"),
            transaction_id:item.request.connector_transaction_id.clone(),
            authorization_code: String::from("33717372"),
        })
    }
}


// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FortePaymentStatus {
    Captured,
    Failed,
    #[default]
    Processing,
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Captured => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Processing => Self::Authorizing,
        }
    }
}


#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: String,
    pub authorization_amount: i64,
    pub authorization_code: String,
    pub entered_by: String,
    pub billing_address: BillingAddress,
    pub card: ForteCardResponse,
    pub response: Response,
    pub links: Links,
}

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
    pub card: ForteCardResponse,
    pub response: Response,
    pub links: Links,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsCaptureRequest {
    pub action: String,
    pub transaction_id: String,
    pub authorization_amount: i64,
    pub authorization_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentsCaptureResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_amount: i64,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: Response_another,
}

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
    pub authorization_amount: i64,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: Response_another,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String,
    pub cvv_result: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response_another {
    pub environment: String,
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Links {
    pub disputes: String,
    pub settlements: String,
    #[serde(rename = "self")]
    pub self_data: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self, Self::Error> {
        let status_msg=String::from(item.response.response.response_desc);
        Ok(Self {
            status: if status_msg == "TEST APPROVED" {  enums::AttemptStatus::Authorized} else { enums::AttemptStatus::Pending },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}


// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    pub amount: i64
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.amount,
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


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus
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
                connector_refund_id: item.response.id.to_string(),
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
                connector_refund_id: item.response.id.to_string(),
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
