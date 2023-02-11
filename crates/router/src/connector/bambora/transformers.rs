use std::fmt::Debug;
// use common_utils::{pii};
// use error_stack::{report, IntoReport};
// use masking::{Secret};
use serde::{Deserialize, Serialize};
use crate::{core::errors,pii::PeekInterface,types::{self,api, storage::enums}};

#[derive(Debug, Serialize, Deserialize)]
enum PaymentMethod {
    InputCardDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InputCardDetails {
    name: String,
    number: String,
    expiry_month: String,
    expiry_year: String,
    cvd: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Custom {
    ref1: String,
    ref2: String,
    ref3: String,
    ref4: String,
    ref5: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Card {
    card_type: String,
    last_four: String,
    card_bin: String,
    address_match: i64,
    postal_result: i64,
    avs_result: String,
    cvd_result: String,
    cavv_result: Option<String>,
    avs: Option<Avs>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Avs {
    id: String,
    message: String,
    processed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Link {
    rel: String,
    href: String,
    method: String,
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Deserialize)]
pub struct BamboraPaymentsRequest {
    amount: i64,
    payment_method: String,
    card: InputCardDetails,
    order_number: String
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let card_details:InputCardDetails = match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => InputCardDetails{
                name:ccard.card_holder_name.peek().clone(),
                number:ccard.card_number.peek().clone(),
                expiry_month:ccard.card_exp_month.peek().clone(),
                expiry_year: ccard.card_exp_year.peek().clone(),
                cvd:ccard.card_cvc.peek().clone(),
            },
            _ => Err(errors::ConnectorError::NotImplemented("payment method".into()))?,
        };

        let payment_method_name = match _item.payment_method {
            storage_models::enums::PaymentMethodType::Card => String::from("card"),
            _ => Err(errors::ConnectorError::NotImplemented("payment method".into()))?,
        };

        println!("{:?}", _item);

        Ok(Self {
            amount: _item.request.amount,
            payment_method: payment_method_name,
            card: card_details,
            order_number: _item.payment_id.to_string(),
        })
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
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
        // todo!()
        // Err
        // Err(errors::ConnectorError::NotImplemented("payment method".into())).into_report()
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BamboraPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<BamboraPaymentStatus> for enums::AttemptStatus {
    fn from(item: BamboraPaymentStatus) -> Self {
        match item {
            BamboraPaymentStatus::Succeeded => Self::Charged,
            BamboraPaymentStatus::Failed => Self::Failure,
            BamboraPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Deserialize)]
pub struct BamboraPaymentsResponse {
    id: String,
    authorizing_merchant_id: i64,
    approved: String,
    message_id: String,
    message: String,
    auth_code: String,
    created: String,
    order_number: String,
    #[serde(rename="type")]
    flow_type: String,
    payment_method: String,
    risk_score: f64,
    amount: f64,
    custom: Custom,
    card: Card,
    links: Vec<Link>
}

impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Charged,
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
pub struct BamboraRefundRequest {
    order_number: String,
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: _item.request.amount,
            order_number: _item.payment_id.to_string(),
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
#[derive(Debug, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    authorizing_merchant_id: i64,
    approved: String,
    message_id: String,
    message: String,
    auth_code: String,
    created: String,
    order_number: String,
    #[serde(rename="type")]
    flow_type: String,
    payment_method: String,
    risk_score: f64,
    amount: f64,
    custom: Custom,
    card: Option<Card>,
    links: Vec<Link>
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: _item.response.id,
                refund_status: enums::RefundStatus::Success,
            }),
            .._item.data
        })
        // Err(errors::ConnectorError::NotImplemented("payment method".into())).into_report()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: _item.response.id,
                refund_status: enums::RefundStatus::Success,
            }),
            .._item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BamboraErrorResponse {}
