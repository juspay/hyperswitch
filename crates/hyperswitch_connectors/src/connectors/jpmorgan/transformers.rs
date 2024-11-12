use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, ExposeInterface};
use serde::{Serialize, Deserialize};
use strum::Display;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

pub struct JpmorganRouterData<T> {
    pub amount: StringMinorUnit, 
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for JpmorganRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPaymentsRequest {
    capture_method : String,
    amount: StringMinorUnit,
    currency : String,
    merchant : JpmorganMerchant,
    payment_method_type : JpmorganPaymentMethodType,

}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCard {
    account_number : Secret<String>,
    expiry: Expiry,
    is_bill_payment: bool,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPaymentMethodType {
    card : JpmorganCard,
}

#[derive(Default, Debug, Serialize)]
pub struct Expiry {
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all="camelCase")]
pub struct JpmorganMerchantSoftware {
    company_name : String,
    product_name : String,
}

#[derive(Default, Debug, Serialize)]
pub struct JpmorganMerchant {
    merchant_software : JpmorganMerchantSoftware,
}

impl TryFrom<&JpmorganRouterData<&PaymentsAuthorizeRouterData>> for JpmorganPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                
                let capture_method : String= if item.router_data.request.is_auto_capture().unwrap() {
                    String::from("NOW")
                }else{
                    String::from("MANUAL")
                };

                let currency : String = String::from("USD");

                let merchant_software = JpmorganMerchantSoftware{
                    company_name: String::from("JPMC"),
                    product_name: String::from("Hyperswitch"),      //could be Amazon or something else, subject to change
                };

                let merchant = JpmorganMerchant{
                    merchant_software,
                };

                let expiry:Expiry = Expiry {
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year, 
                };

                let card = JpmorganCard {
                    //in my case i used acc num
                    account_number: String::from("4012000033330026").into(),   //keeping a dummy val as of now
                    expiry,
                    is_bill_payment: item.router_data.request.is_auto_capture()?,
                };

                let payment_method_type = JpmorganPaymentMethodType{
                    card
                };

                Ok(Self {
                    capture_method,
                    currency,
                    amount: item.amount.clone(),
                    merchant,
                    payment_method_type,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
//in jpm, we get a client id and secret and using these two, we have a curl, we make an api call and we get a access token in res with an expiry time as well
pub struct JpmorganAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret : Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for JpmorganAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { 
                api_key, 
                key1
            } 
            => Ok(Self {
                client_id : api_key.to_owned(),
                client_secret : key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all="UPPERCASE")]
pub enum JpmorganTransactionStatus {
    SUCCESS, 
    DENIED,
    ERROR,
    /*[default]
    might be processing, not mentioned in docs*/
}
//might change

 
#[derive(Debug, Copy, Display, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganAPIStatus {
    SUCCESS,
    #[default]
    DENIED,
}
//add or remove this later

impl From<JpmorganTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: JpmorganTransactionStatus) -> Self {
        match item {
            JpmorganTransactionStatus::SUCCESS => Self::Charged,
            JpmorganTransactionStatus::DENIED | JpmorganTransactionStatus::ERROR => Self::Failure,
            //JpmorganTransactionStatus::Processing => Self::Authorizing,
            //more fields to add here
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganAdditionalData {
    electronic_commerce_indicator : Secret<String>,
    authorization_response_category : Secret<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganNetworkResponse {
    address_verification_result : Secret<String>,
    additional_data : JpmorganAdditionalData,
    network_transaction_id : Secret<String>,
    network_response_code : Secret<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCardResponse {
    card_type : Secret<String>,
    card_type_name : Secret<String>,
    unmasked_account_number : Secret<String>,    
    network_response : JpmorganNetworkResponse,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPaymentMethodTypeCard {
    card : JpmorganCardResponse
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganMerchantSoftwareResponse {
    company_name : String,
    product_name : String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganMerchantResponse {
    merchant_id : String,
    merchant_software : JpmorganMerchantSoftwareResponse,
    merchant_category_code : String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Authorization {
    authorization_id : String,
    amount : i64,
    transaction_status_code : String,
    authorization_type : String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPR{
    payment_request_id : String,
    payment_request_status : String,
    authorizations : Vec<Authorization>
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPaymentsResponse {
    transaction_id : String,
    request_id : String,
    transaction_state : String,
    response_status : String,
    response_code : String,
    response_message : String,

    payment_method_type : JpmorganPaymentMethodTypeCard,

    capture_method : String,
    initiator_type : String,
    account_on_file : String,
    is_void : bool,
    transaction_date : String,
    approval_code : String,
    host_message : String,
    amount : String,
    currency : String,
    remaining_auth_amount : i64,
    host_reference_id : String,

    merchant : JpmorganMerchantResponse,
    payment_request : JpmorganPR,   //PR stands for payment req

}

fn convert_transaction_state(transaction_state: &str) -> common_enums::AttemptStatus {
    // Map the string value of `transaction_state` to the appropriate AttemptStatus variant
    match transaction_state {
        "Authorized" => common_enums::AttemptStatus::Authorized,
        "AuthorizationFailed" => common_enums::AttemptStatus::AuthorizationFailed,
        "Charged" => common_enums::AttemptStatus::Charged,
        "PaymentMethodAwaited" => common_enums::AttemptStatus::PaymentMethodAwaited,
        "Failure" => common_enums::AttemptStatus::Failure,
        // Handle other cases if needed, using the most suitable AttemptStatus variant
        _ => common_enums::AttemptStatus::default(),  // Default to Pending if no match is found
    }
}


impl<F, T> TryFrom<ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData> 
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Use a helper method to handle the conversion of transaction_state
        let status = convert_transaction_state(&item.response.transaction_state);

        // Extracting the transaction ID
        let resource_id = ResponseId::ConnectorTransactionId(item.response.transaction_id.clone());

        // Optional fields that may or may not be present
        let network_txn_id = if !item.response.payment_method_type.card.network_response.network_transaction_id.clone().expose().is_empty() {
            Some(item.response.payment_method_type.card.network_response.network_transaction_id.clone().expose().to_string())
        } else {
            None
        };

        let connector_response_reference_id = if !item.response.host_reference_id.is_empty() {
            Some(item.response.host_reference_id.clone())
        } else {
            None
        };

        let redirection_data = None; // Set as None, as no redirection is indicated in `JpmorganPaymentsResponse`

        let mandate_reference = None; // Assuming no mandate reference is provided; adjust if necessary

        // Building the PaymentsResponseData
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: Option<String>,
    status_code: i32,
    txn_secret: Option<String>,
    tid: Option<Secret<i64>>,
    test_mode: Option<i8>,
    status: Option<JpmorganTransactionStatus>,
}

#[derive(Default, Debug, Serialize)]
pub struct JpmorganRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&JpmorganRouterData<&RefundsRouterData<F>>> for JpmorganRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &JpmorganRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
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
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct JpmorganErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
