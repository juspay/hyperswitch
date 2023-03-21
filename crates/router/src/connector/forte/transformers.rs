use serde::{Deserialize, Serialize};
use masking::{Secret, PeekInterface};
use crate::{connector::utils::{AddressDetailsData, RouterData},core::errors,types::{self,api, storage::enums}};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    authorization_amount: i64,
    card: ForteCard,
    billing_address : BillingAddress
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    // card_type: Option<api_models::enums::CardNetwork>,
    card_type:String,
    name_on_card: Secret<String>,
    account_number: Secret<String, common_utils::pii::CardNumber>,
    expire_month: Secret<String>,
    expire_year: Secret<String>,
    card_verification_value: Secret<String>,
}

#[derive(Default, Debug,Clone, Serialize, Eq, PartialEq)]
pub struct BillingAddress {
    pub first_name:String,
    pub last_name : String
}



impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let address = item.get_billing_address()?;
        let first_name = address
        .first_name
        .clone()
        .map_or("".to_string(), |first_name| first_name.peek().to_string());
        let last_name = address
        .last_name
        .clone()

.map_or("".to_string(), |last_name| last_name.peek().to_string());
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = ForteCard {
                    // card_type: req_card.card_network,
                    card_type:String::from("visa"),
                    name_on_card: req_card.card_holder_name,
                    account_number: req_card.card_number,
                    expire_month: req_card.card_exp_month,
                    expire_year: req_card.card_exp_year,
                    card_verification_value: req_card.card_cvc,
                };
                Ok(Self {
                    authorization_amount: item.request.amount,
                    card,billing_address:BillingAddress{first_name:first_name,last_name:last_name}
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }    
    }
}


pub struct ForteAuthType {
    pub(super) api_key: String,
    pub(super) api_id: String,
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        println!("{:?}",auth_type);
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_string(),
                api_id: key1.to_string()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum FortePaymentStatus {
    #[serde(rename = "A01")]
    Authorized,
    #[serde(rename = "A02")]
    Complete,
    #[serde(rename = "A03")]
    Failed,
    #[serde(rename = "A04")]
    Voided,
    #[serde(rename = "A05")]
    Declined,
    #[serde(rename = "A06")]
    #[default]
    Settling,
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Voided => Self::Voided,
            FortePaymentStatus::Authorized => Self::Authorized,
            FortePaymentStatus::Complete => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Declined => Self::RouterDeclined,
            FortePaymentStatus::Settling => Self::Authorizing,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: String,
    pub authorization_amount: i64,
    pub authorization_code: String,
    pub entered_by: String,
    //pub billing_address: BillingAddress,
    pub card: ResponseCard,
    pub response: Response,
    pub links: Links,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCard {
    pub name_on_card: String,
    #[serde(rename = "last_4_account_number")]
    pub last_4__account_number: String,
    pub masked_account_number: String,
    pub expire_month: i64,
    pub expire_year: i64,
    pub card_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    pub disputes: String,
    pub settlements: String,
    #[serde(rename = "self")]
    pub links_self: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub environment: String,
    pub response_type: String,
    pub response_code: FortePaymentStatus,
    pub response_desc: String,
    pub authorization_code: String,
    pub avs_result: String,
    pub cvv_result: String,
}



impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response.response_code),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.authorization_code),
                redirection_data: None,
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

//TODO: Fill the struct with respective fields
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// #[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
// pub struct ForteErrorResponse {
//     pub code: Option<String>,
//     pub message: String,
// }