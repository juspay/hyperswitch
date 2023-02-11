use masking::Secret;
use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,api::{self, }, storage::enums},pii::{self, PeekInterface}};

#[derive(Eq, PartialEq, Serialize, Clone, Debug)]
pub struct PayeezyCard {
    #[serde(rename = "type")]
    pub card_type : String,
    pub cardholder_name : Secret<String>,
    pub card_number : Secret<String, pii::CardNumber>,
    pub exp_date : String,
    pub cvv : Secret<String>
}

#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum PayeezyPaymentMethod {
    PayeezyCard(PayeezyCard),
}

//TODO: Fill the struct with respective fields
#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
pub struct PayeezyPaymentsRequest {
    pub merchant_ref : String,
    pub transaction_type : String,
    pub method : PayeezyPaymentMethodType,
    pub amount : i64,
    pub currency_code : String,
    pub credit_card : PayeezyPaymentMethod
}

#[derive(Default, Debug, Serialize, Eq, PartialEq, Clone)]
pub enum PayeezyPaymentMethodType {
    #[default]
    #[serde(rename = "credit_card")]
    Card,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PayeezyPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.payment_method {
            storage_models::enums::PaymentMethodType::Card => get_card_specific_payment_data(item),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn get_card_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<PayeezyPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let merchant_ref = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
    let method = PayeezyPaymentMethodType::Card;
    let amount = item.request.amount;
    let transaction_type = String::from("authorize");
    let currency_code = item.request.currency.to_string();
    let credit_card = get_payment_method_data(item)?;
    Ok(PayeezyPaymentsRequest {
        merchant_ref,
        transaction_type,
        method,
        amount,
        currency_code,
        credit_card
    })
}

fn get_payment_method_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<PayeezyPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    match item.request.payment_method_data {
        api::PaymentMethod::Card(ref card) => {
            let payeezy_card = PayeezyCard {
                card_type: String::from("visa"),
                cardholder_name: card.card_holder_name.clone(),
                card_number: card.card_number.clone(),
                exp_date: format!("{}{}", card.card_exp_month.peek().clone(), card.card_exp_year.peek().clone()),
                cvv: card.card_cvc.clone(),
            };
            Ok(PayeezyPaymentMethod::PayeezyCard(payeezy_card))
        }
        _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PayeezyAuthType {
    pub(super) api_key: String,
    pub(super) api_secret: String,
    pub(super) merchant_token: String
}

impl TryFrom<&types::ConnectorAuthType> for PayeezyAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey { api_key, key1, api_secret } = item {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: api_secret.to_string(),
                merchant_token: key1.to_string()
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
pub enum PayeezyPaymentStatus {
    Approved,
    Declined,
    #[default]
    #[serde(rename = "Not Processed")]
    NotProcessed,
}

impl From<PayeezyPaymentStatus> for enums::AttemptStatus {
    fn from(item: PayeezyPaymentStatus) -> Self {
        match item {
            PayeezyPaymentStatus::Approved => Self::Authorized,
            PayeezyPaymentStatus::Declined => Self::AuthorizationFailed,
            PayeezyPaymentStatus::NotProcessed => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyPaymentsResponse {
    correlation_id: String,
    transaction_status: PayeezyPaymentStatus,
    validation_status : String,
    transaction_type : String,
    transaction_id : String,
    transaction_tag : String,
    method : String,
    amount : String,
    currency : String,
    bank_resp_code : String,
    bank_message : String,
    gateway_resp_code : String,
    gateway_message : String
}

impl<F,T> TryFrom<types::ResponseRouterData<F, PayeezyPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, PayeezyPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.transaction_status),
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
pub struct PayeezyRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PayeezyRefundRequest {
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
pub struct Message {
    pub code : String,
    pub description : String
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayeezyError {
    pub messages : Vec<Message>
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayeezyErrorResponse {
    pub transaction_status : String,
    #[serde(rename = "Error")]
    pub error : PayeezyError
}
