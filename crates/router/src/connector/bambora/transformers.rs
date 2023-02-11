

use error_stack;

use serde::{Deserialize, Serialize};

use crate::{
   
 
    core::errors,
    pii::{self, Secret},

    types::{
        self,
        api::{self},
        storage::enums as storage_enums,
    },
};

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BamboraPaymentMethod {
    // #[serde(rename(serialize = "card"))]
    BamboraCard(Card),
    
}




#[derive(Debug, Serialize)]
pub struct Card {
    name:Secret<String>,
    number: Secret<String, pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
 
    cvd: Option<Secret<String>>,
}
#[derive(Debug, Serialize)]
pub struct BamboraPaymentsRequest {
    amount: f64,
    payment_method:String,
    card: Option<BamboraPaymentMethod>,
    
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let a =match item.payment_method {
            storage_models::enums::PaymentMethodType::Card => get_card_specific_payment_data(item),
            
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        };
        println!("Dummy Data:{a:?}");
        let b = a.unwrap();
        // let jsonn = serde_json::to_string(&b).unwrap();
        println!("Dummy Data2: {:#?}",b);
        Ok(b)
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
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
// // pub enum BamboraPaymentStatus {
// //     Succeeded,
// //     Failed,
// //     #[default]
// //     Processing,
// // }

// impl From<BamboraPaymentStatus> for storage_enums::AttemptStatus {
//     fn from(item: BamboraPaymentStatus) -> Self {
//         match item {
//             BamboraPaymentStatus::Succeeded => Self::Charged,
//             BamboraPaymentStatus::Failed => Self::Failure,
//             BamboraPaymentStatus::Processing => Self::Authorizing,
//         }
//     }
// }



//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraPaymentsResponse {
    approved:String,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: if item.response.approved.to_owned() == "1"{
            storage_enums::AttemptStatus::Charged
        }else{
                storage_enums::AttemptStatus::Failure},
            
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
pub struct BamboraRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
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

impl From<RefundStatus> for storage_enums::RefundStatus {
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
pub struct BamboraErrorResponse {}
fn get_amount_data(item: &types::PaymentsAuthorizeRouterData) -> f64 {
    item.request.amount as f64
}

fn get_payment_method_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<BamboraPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    match item.request.payment_method_data {
        api::PaymentMethod::Card(ref card) => {
            let bambora_card = Card {
                // payment_type: PaymentType::Card,
               
                number: card.card_number.clone(),
                expiry_month: card.card_exp_month.clone(),
                expiry_year: card.card_exp_year.clone(),
                cvd: Some(card.card_cvc.clone()),
                name:card.card_holder_name.clone()
            };
            Ok(BamboraPaymentMethod::BamboraCard(bambora_card))
        }
        api::PaymentMethod::Wallet(ref _wallet_data) => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        api_models::payments::PaymentMethod::PayLater(ref _pay_later_data) => 
            Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        
        api_models::payments::PaymentMethod::BankTransfer
        | api_models::payments::PaymentMethod::Paypal => {
            Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
        }
    }
}
fn get_card_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<BamboraPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = get_amount_data(item);
    let payment_method=get_payment_method_data(item)?;
  
    Ok(BamboraPaymentsRequest {
        amount,
        payment_method:"card".to_owned(),
        card: Some(payment_method),
      
    })
}