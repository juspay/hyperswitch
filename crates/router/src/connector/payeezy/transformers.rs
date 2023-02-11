use serde::{Deserialize, Serialize};
use crate::{core::errors,pii::PeekInterface,types::{self,api, storage::enums}};
use api_models::{self, payments};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayeezyPaymentsRequest {
    pub merchant_ref: String,
    pub transaction_type: String,
    pub method: String,
    pub amount: String,
    pub currency_code: String,
    pub credit_card: CardDetails,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CardDetails {
    #[serde(rename = "type")]
    card_type: String,
    cardholder_name: String,
    card_number: String,
    exp_date: String,
    cvv: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PayeezyPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let card = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => CardDetails {
                card_type: "visa".to_string(),
                cardholder_name: "John Smith".to_string(),
                card_number: ccard.card_number.peek().to_string(),
                exp_date: format!("{}{}", ccard.card_exp_month.peek().to_string(), ccard.card_exp_year.peek().to_string()),
                cvv: ccard.card_cvc.peek().to_string()
            },
            _ => Err(errors::ConnectorError::NotImplemented(String::from("")))?
        };

        // let card = CardDetails {
        //     card_type: "visa".to_string(),
        //     cardholder_name: "John Smith".to_string(),
        //     card_number: "1234".to_string(),
        //     exp_date: "0425".to_string(),
        //     cvv: "1234".to_string(),
        // };
        Ok(Self {
            merchant_ref: item.payment_id.to_string(),
            transaction_type: "authorize".to_string(),
            method: "credit_card".to_string(),
            amount: item.request.amount.to_string(),
            currency_code: item.request.currency.to_string(),
            credit_card: card,
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PayeezyAuthType {
    pub(super) api_key: String,
    pub(super) token: String,
    pub(super) authorization: String,
}

impl TryFrom<&types::ConnectorAuthType> for PayeezyAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey { api_key, key1, api_secret } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                token: key1.to_string(),
                authorization: api_secret.to_string(),
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
    Failed,
    #[default]
    Processing,
}

impl From<PayeezyPaymentStatus> for enums::AttemptStatus {
    fn from(item: PayeezyPaymentStatus) -> Self {
        match item {
            PayeezyPaymentStatus::Approved => Self::Charged,
            PayeezyPaymentStatus::Failed => Self::Failure,
            PayeezyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyPaymentsResponse {
    transaction_status: PayeezyPaymentStatus,
    transaction_id: String,
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
pub struct PayeezyErrorResponse {}
