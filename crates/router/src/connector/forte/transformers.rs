use serde::{Deserialize, Serialize};
use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};
//Types Start
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FortePaymentsRequest {
    #[serde(rename = "authorization_amount")]
    pub authorization_amount: f64,
    #[serde(rename = "subtotal_amount")]
    pub subtotal_amount: f64,
    #[serde(rename = "billing_address")]
    pub billing_address: BillingAddress,
    pub card: Card,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "last_name")]
    pub last_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    #[serde(rename = "card_type")]
    pub card_type: String,
    #[serde(rename = "name_on_card")]
    pub name_on_card: String,
    #[serde(rename = "account_number")]
    pub account_number: String,
    #[serde(rename = "expire_month")]
    pub expire_month: String,
    #[serde(rename = "expire_year")]
    pub expire_year: String,
    #[serde(rename = "card_verification_value")]
    pub card_verification_value: String,
}

//Res Types Start
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FortePaymentsResponse {
    #[serde(rename = "transaction_id")]
    pub transaction_id: String,
    #[serde(rename = "location_id")]
    pub location_id: String,
    pub action: String,
    #[serde(rename = "authorization_amount")]
    pub authorization_amount: f64,
    #[serde(rename = "entered_by")]
    pub entered_by: String,
    #[serde(rename = "billing_address")]
    pub billing_address: BillingAddress,
    pub card: Card,
    pub response: Response,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub environment: String,
    #[serde(rename = "response_type")]
    pub response_type: String,
    #[serde(rename = "response_code")]
    pub response_code: String,
    #[serde(rename = "response_desc")]
    pub response_desc: String,
    #[serde(rename = "authorization_code")]
    pub authorization_code: String,
    #[serde(rename = "avs_result")]
    pub avs_result: String,
    #[serde(rename = "cvv_result")]
    pub cvv_result: String,
}

//Res Types end

//TransactionId Types start
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionByIdResponse {
    #[serde(rename = "transaction_id")]
    pub transaction_id: String,
    #[serde(rename = "organization_id")]
    pub organization_id: String,
    #[serde(rename = "location_id")]
    pub location_id: String,
    pub status: String,
    pub action: String,
    #[serde(rename = "authorization_amount")]
    pub authorization_amount: i64,
    #[serde(rename = "authorization_code")]
    pub authorization_code: String,
    #[serde(rename = "received_date")]
    pub received_date: String,
    #[serde(rename = "billing_address")]
    pub billing_address: BillingAddress,
    pub card: Card,
    pub response: Response,
    pub links: Links,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalAddress {
    #[serde(rename = "street_line1")]
    pub street_line1: String,
    #[serde(rename = "street_line2")]
    pub street_line2: String,
    pub locality: String,
    pub region: String,
    pub country: String,
    #[serde(rename = "postal_code")]
    pub postal_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub disputes: String,
    pub settlements: String,
    #[serde(rename = "self")]
    pub self_field: String,
}

//TransactionId Types end
//Types End
impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {

        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let request  =  FortePaymentsRequest{
                    billing_address : BillingAddress { first_name: ccard.card_holder_name.peek().clone(), last_name: ccard.card_holder_name.peek().clone() },
                    card: Card {
                        card_type               : String::from("visa"),
                        name_on_card            : ccard.card_holder_name.peek().clone(),
                        account_number          : ccard.card_number.peek().clone(),
                        expire_month            : ccard.card_exp_month.peek().clone(),
                        expire_year             : ccard.card_exp_year.peek().clone(),
                        card_verification_value : ccard.card_cvc.peek().clone(),
                    },
                    authorization_amount: item.request.amount as f64,
                    subtotal_amount: item.request.amount as f64,
                };
                Ok(request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
    }
}
}


// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
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
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FortePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        let status_string = String::from(item.response.response.response_desc);
        Ok(Self {
            status: if status_string == "APPROVAL" {  enums::AttemptStatus::Authorized} else { enums::AttemptStatus::Pending },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transaction_id),
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
pub struct ForteRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
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
pub struct ForteErrorResponse {}
