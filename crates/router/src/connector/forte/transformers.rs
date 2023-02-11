use serde::{Deserialize, Serialize};
use crate::{core::errors,pii::PeekInterface,types::{self,api, storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct FortePaymentsRequest {
    authorization_amount : f32,
    subtotal_amount : f32, 
    billing_address : BillingInfo,
    card : Card,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct BillingInfo {
    first_name: String,
    last_name: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Card {
    card_type: String,
    name_on_card: String,
    account_number: String,
    expire_month: String,
    expire_year: String,
    card_verification_value: String
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
// #[serde(rename_all = "camelCase")]
pub struct Card2 {
    card_type: String,
    name_on_card: String,
    last_4_account_number: String,
    masked_account_number: String,
    expire_month: i32,
    expire_year: i32
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        // todo!()
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let payment_request = Self {
                    authorization_amount: item.request.amount as f32,
                    subtotal_amount: item.request.amount as f32,
                    billing_address : BillingInfo { first_name: "saurav".to_string(), last_name: "cv".to_string() },
                    card: Card {
                        card_type: "visa".to_string(),
                        name_on_card: ccard.card_holder_name.peek().clone(),
                        account_number: ccard.card_number.peek().clone(),
                        expire_month: ccard.card_exp_month.peek().clone(),
                        expire_year: ccard.card_exp_year.peek().clone(),
                        card_verification_value : ccard.card_cvc.peek().clone(),
                    },
                };

                println!("something --> {payment_request:?}");
                let tmp = serde_json::to_string(&payment_request);
                println!("something2 --> {tmp:?}");
                Ok(payment_request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
    }
}
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        todo!()
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

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Succeeded => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Response {
    environment: String,
    response_type: String,
    response_code: String,
    response_desc: String,
    authorization_code: String,
    avs_result: String,
    cvv_result: String
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    // status: FortePaymentStatus,
    // id: String,
        transaction_id: String,
        location_id: String,
        action: String,
        authorization_amount: f32,
        entered_by: String,
        billing_address: BillingInfo,
        card: Card2,
        response: Response
      }

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(enums::AttemptStatus::Authorized),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId("1234".to_string()),
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
