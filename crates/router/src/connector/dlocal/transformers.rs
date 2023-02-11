use std::ptr::eq;

use masking::PeekInterface;
use serde::{Deserialize, Serialize};
use storage_models::schema::payment_attempt::payment_method_id;
use crate::{core::errors,types::{self,api, storage::enums}};

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Payer {
    pub name : String,
    pub email: String,
    pub document: String,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Card {
    pub holder_name: String,
    pub number: String,
    pub cvv: String,
    pub expiration_month: i32,
    pub expiration_year: i32,
    pub capture: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: enums::Currency,
    pub country: Option<String>,
    pub payment_method_id: String,
    pub payment_method_flow: String,
    pub payer: Payer,
    pub card: Card,
    pub order_id: String,
    pub notification_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DlocalPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let should_capture = matches!(
                    item.request.capture_method,
                    Some(enums::CaptureMethod::Automatic) | None
                );
                let payment_request = Self {
                    amount: item.request.amount,
                    country: Some(get_currency(item.request.currency)),
                    currency : item.request.currency,
                    payment_method_id : "CARD".to_string(),
                    payment_method_flow : "DIRECT".to_string(),
                    payer : Payer {
                        name: ccard.card_holder_name.peek().clone(),
                        email: match &item.request.email{
                            Some (c) => c.peek().clone().to_string(),
                            None => "dummyEmail@gmail.com".to_string()
                        },
                        document: "08533195966".to_string()
                    },
                    card : Card {
                        holder_name: ccard.card_holder_name.peek().clone(),
                        number: ccard.card_number.peek().clone(),
                        cvv: ccard.card_cvc.peek().clone(),
                        expiration_month: ccard.card_exp_month.peek().clone().parse::<i32>().unwrap(),
                        expiration_year: ccard.card_exp_year.peek().clone().parse::<i32>().unwrap(),
                        capture: should_capture.to_string()
                    },
                    order_id : item.payment_id.clone(),
                    notification_url : match &item.return_url {
                        Some (val) => val.to_string(),
                        None => "http://wwww.sandbox.juspay.in/hackathon/H1005".to_string()
                    }
                    };
                println!("{:#?}",payment_request);
                Ok(payment_request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
        }

    }
}

fn get_currency(item: enums::Currency) -> String{
    match item{
        BRL => "BR".to_string(),
        _ => "IN".to_string()
    }
}

pub struct DlocalPaymentsSyncRequest {
    pub authz_id: String ,
}

impl TryFrom<&types::PaymentsSyncRouterData> for DlocalPaymentsSyncRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self,Self::Error> {
        Ok(Self {
            authz_id: (item.request.connector_transaction_id.get_connector_transaction_id().unwrap()),
        })
    }
}

pub struct DlocalPaymentsCancelRequest {
    pub cancel_id: String ,
}

impl TryFrom<&types::PaymentsCancelRouterData> for DlocalPaymentsCancelRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self,Self::Error> {
        Ok(Self {
            cancel_id: (item.request.connector_transaction_id.clone()),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsCaptureRequest {
    pub authorization_id: String ,
    pub amount: i64 ,
    pub currency: String ,
    pub order_id: String ,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for DlocalPaymentsCaptureRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self,Self::Error> {
        let amount_to_capture = match item.request.amount_to_capture {
            Some(val) => val,
            None => item.request.amount
        };
        Ok(Self {
            authorization_id: (item.request.connector_transaction_id.clone())
            , amount: (amount_to_capture)
            , currency: (item.request.currency.to_string())
            , order_id: (item.payment_id.clone())
        })
    }
}
//TODO: Fill the struct with respective fields
// Auth Struct
pub struct DlocalAuthType {
    pub(super) xLogin: String,
    pub(super) xTransKey: String,
    pub(super) secret : String,
}

impl TryFrom<&types::ConnectorAuthType> for DlocalAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey { api_key , key1, api_secret} = auth_type {
            Ok(Self { xLogin: (api_key.to_string()), xTransKey: (key1.to_string()), secret: (api_secret.to_string()) })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }

    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DlocalPaymentStatus {
    AUTHORIZED,
    PAID,
    VERIFIED,
    CANCELLED,
    #[default]
    PENDING,
}

impl From<DlocalPaymentStatus> for enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::AUTHORIZED => Self::Authorized,
            DlocalPaymentStatus::VERIFIED => Self::Authorized,
            DlocalPaymentStatus::PAID => Self::Charged,
            DlocalPaymentStatus::PENDING => Self::Pending,
            DlocalPaymentStatus::CANCELLED => Self::Voided
        }
    }
}

//TODO: Fill the struct with respective fields
// {
//     "id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a",
//     "amount": 120,
//     "currency": "USD",
//     "payment_method_id": "CARD",
//     "payment_method_type": "CARD",
//     "payment_method_flow": "DIRECT",
//     "country": "BR",
//     "card": {
//         "holder_name": "Thiago Gabriel",
//         "expiration_month": 10,
//         "expiration_year": 2040,
//         "brand": "VI",
//         "last4": "1111"
//     },
//     "created_date": "2019-02-06T21:04:43.000+0000",
//     "approved_date": "2019-02-06T21:04:44.000+0000",
//     "status": "AUTHORIZED",
//     "status_detail": "The payment was authorized",
//     "status_code": "600",
//     "order_id": "657434343",
//     "notification_url": "http://merchant.com/notifications"
// }
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsResponse {
    status: DlocalPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsSyncResponse {
    status: DlocalPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, DlocalPaymentsSyncResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, DlocalPaymentsSyncResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsCaptureResponse {
    status: DlocalPaymentStatus,
    id: String,
}
impl<F,T> TryFrom<types::ResponseRouterData<F, DlocalPaymentsCaptureResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, DlocalPaymentsCaptureResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
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

pub struct DlocalPaymentsCancelResponse {
    status: DlocalPaymentStatus,
    id: String,
}
impl<F,T> TryFrom<types::ResponseRouterData<F, DlocalPaymentsCancelResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, DlocalPaymentsCancelResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
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
pub struct RefundRequest {
    pub amount: String,
    pub payment_id: String,
    pub currency: String,
    pub notification_url: String,
    pub id: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {

        let amount_to_refund = item.request.refund_amount.to_string();
        let return_url = match item.return_url.clone() {
            Some(val) => val,
            None => "https://google.com".to_string()
        };
        let payment_intent = item.request.connector_transaction_id.clone();
        Ok(Self {
            amount: amount_to_refund ,
            payment_id:payment_intent,
            currency:(item.request.currency.to_string()),
            id:item.request.refund_id.clone(),
            notification_url: return_url,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    SUCCESS,
    #[default]
    PENDING,
    REJECTED,
    CANCELLED,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::SUCCESS => Self::Success,
            RefundStatus::PENDING => Self::Pending,
            RefundStatus::REJECTED => Self::ManualReview, // Is rejected manual review?
            RefundStatus::CANCELLED => Self::Failure,
        }
    }
}


//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    // pub payment_id: String,
    // pub notification_url: String,
    // pub amount: f64,
    // pub currency: String,
    pub status: RefundStatus,
    // pub status_code: i32,
    // pub status_detail: String,
    // pub created_date: String,
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


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DlocalRefundsSyncRequest {
    pub refund_id: String ,
}

impl TryFrom<&types::RefundSyncRouterData> for DlocalRefundsSyncRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self,Self::Error> {
        let refund_id = match item.request.connector_refund_id.clone() {
            Some(val) => val,
            None => item.request.refund_id.clone(),
        };
        Ok(Self {
            refund_id: (refund_id),
        })
    }
}
impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DlocalErrorResponse {
    pub code :  i32,
    pub message : String,
    pub param : Option<String>,
}
