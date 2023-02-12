use serde::{Deserialize, Serialize};
use masking::PeekInterface;

use crate::{core::errors,types::{self,api, storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize)]
pub struct BamboraPaymentsRequest {
    pub amount : i64,
    pub payment_method : String,
    pub card : Source
}

#[derive(Debug, Serialize)]
pub struct CardSource {
    pub name: Option<String>,
    pub number: Option<String>,
    pub expiry_month: Option<String>,
    pub expiry_year: Option<String>,
    pub cvd: Option<String>
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Source {
    Card(CardSource)
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let amount = _item.request.amount;
                let card = Source::Card(CardSource {
                        name : Some(ccard.card_holder_name.peek().clone()),
                        number: Some(ccard.card_number.peek().clone()),
                        expiry_month: Some(ccard.card_exp_month.peek().clone()),
                        expiry_year: Some(ccard.card_exp_year.peek().clone()),
                        cvd: Some(ccard.card_cvc.peek().clone()),
                    });
                let payment_method = "card".to_string();
                Ok(Self {
                    amount,
                    payment_method,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, ..} = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
// #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
// #[serde(rename_all = "lowercase")]
// pub enum BamboraPaymentStatus {
//     NotApproved
//     Approved,
//     #[default]
//     Processing,
// }

// impl From<BamboraPaymentStatus> for enums::AttemptStatus {
//     fn from(item: BamboraPaymentStatus) -> Self {
//         match item {
//             BamboraPaymentStatus::Succeeded => Self::Charged,
//             BamboraPaymentStatus::Failed => Self::Failure,
//             BamboraPaymentStatus::Processing => Self::Authorizing,
//         }
//     }
// }

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SI {
    String(String),
    Int(i64)
} 

fn get_srt_from_si(s: SI) -> String {
    match s {
        SI::String(str) => str,
        SI::Int(i) => i.to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BamboraPaymentsResponse {

    id: SI, // Bombora is giving different response in payment create and payment sync
    approved: SI,
    message_id: SI
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct Links {
    pub rel: String,
    pub href: String,
    pub method : String
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CardSourceData {
    #[serde(rename = "type")]
    pub card_type: Option<String>,
    pub last_four: Option<String>,
    pub card_bin: Option<String>,
    pub address_match: i64,
    pub postal_result: i64,
    pub avs_result: Option<String>,
    pub cvd_result: Option<String>,
    pub avs: AVSType
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AVSResponse {
    #[serde(rename = "type")]
    pub id: Option<String>,
    pub message: Option<String>,
    pub processed: Option<bool>
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AVSType {
    AVS(AVSResponse)
}



#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CardResponse {
    Card(CardSourceData)
}


#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CustomSource {
    #[serde(rename = "type")]
    pub ref1: Option<String>,
    pub ref2: Option<String>,
    pub ref3: Option<String>,
    pub ref4: Option<String>,
    pub ref5: Option<String>
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Reference {
    Custom(CustomSource)
}


impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: match (get_srt_from_si(item.response.approved).as_str(), get_srt_from_si(item.response.message_id).as_str()) {
                ("0", _) => enums::AttemptStatus::Failure,
                (_, "0") => enums::AttemptStatus::Authorizing,
                (_, _) => enums::AttemptStatus::Charged
            },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(match item.response.id { SI::String(s) => s, SI::Int(i) => i.to_string()}),
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
pub struct BamboraRefundRequest {
    amount: i64,
    order_number: String
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
            order_number: item.request.refund_id.clone()
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BamboraRefundStatus {
    #[default]
    ZERO,
    ONE,
}

impl From<BamboraRefundStatus> for enums::RefundStatus {
    fn from(item: BamboraRefundStatus) -> Self {
        match item {
            BamboraRefundStatus::ONE => Self::Success,
            BamboraRefundStatus::ZERO => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    approved: BamboraRefundStatus,
    id: String,
    status: String,
    authorizing_merchant_id: String,
    description: String,
    message_id: i64,
    message: String
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.approved);
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
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BamboraErrorResponse {
    pub code: i8,
    pub category: i8,
    pub message: String,
    references: String,
    details: Vec<Detail>,
    validation: CardValidation
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct Detail {
    pub field: String,
    pub message: String
}


#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Validator {
    pub id: String,
    pub approved: String,
    pub message_id: i8,
    pub message: Option<String>,
    pub auth_code: Option<String>,
    pub trans_date: Option<String>,
    pub order_number: Option<String>,
    pub typeb: Option<String>,
    pub amount: Option<String>,
    pub cnd_id: Option<String>


}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum CardValidation {
    Validation(Validator)
}