// use cards::CardNumber;
use common_enums::enums;
use common_utils::{
    pii::Email, types::StringMajorUnit
};
// date_time::format_date,
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsCaptureData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
// use ring::agreement::PublicKey;
use serde::{Deserialize, Serialize}; 
use strum::Display;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData}, 
    utils::{ PaymentsAuthorizeRequestData, RouterData as OtherRouterData}
};
// use crate::utils::RouterData;
type Error = error_stack::Report<errors::ConnectorError>;
//TODO: Fill the struct with respective fields
pub struct ElavonRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for ElavonRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ElavonPaymentsRequest {
    amount: StringMajorUnit,
    card: ElavonCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ElavonCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

fn get_exp_date(month: Secret<String>, year:Secret<String>)->Secret<String>{

    let mmyy=format!("{}{}", month.expose(),year.expose());
    return Secret::new(mmyy);
}

fn get_transaction_type(capture_method: Option<enums::CaptureMethod>) -> Result<u8, Error> {
    match capture_method {
        Some(enums::CaptureMethod::Automatic) | None => Ok(1),
        Some(enums::CaptureMethod::Manual) => Ok(2),
        _ => Err(errors::ConnectorError::CaptureMethodNotSupported)?,
    }
}

fn get_transaction_body(
    req: &ElavonRouterData<&PaymentsAuthorizeRouterData>,
)-> Result<String, Error> {

    let auth_details = ElavonAuthType::try_from(&req.router_data.connector_auth_type)?;
    let transaction_type = get_transaction_type(req.router_data.request.capture_method)?;
    // let card_info = get_card_data(req.router_data)?;

    let transaction_data = format!(
        r#"
        <txn>
            <ssl_transaction_type>ccauthonly</ssl_transaction_type>
            <ssl_account_id>{}</ssl_account_id>
            <ssl_user_id>{}</ssl_user_id>
            <ssl_email>suman.maji@juspay.in</ssl_email>
            <ssl_pin>XTR2W5ZQUWRA8T0B3B56UT6R7ZFB55OAL27T0HPL601B1J3HG9PM67R4T6E1AO0F</ssl_pin>
            <ssl_vendor_id>{}</ssl_vendor_id>
            <ssl_card_number>4111111111111111</ssl_card_number>
            <ssl_exp_date>1249</ssl_exp_date>
            <ssl_amount>0.02</ssl_amount>
            <ssl_cvv2cvc2>123</ssl_cvv2cvc2>
            <ssl_invoice_number>INV001</ssl_invoice_number>
        </txn>
    "#, 
    auth_details.ssl_account_id.expose(),
    auth_details.ssl_user_id.expose(),
    auth_details.ssl_vendor_id.expose(),

    );
    Ok(transaction_data)
}

// enum TransactionType{

// }

pub struct ElavonCreditCardSaleReq {
    pub ssl_transaction_type: String, //ccsale
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_vendor_id: Secret<String>,
    pub ssl_email: Email,

    pub ssl_cvv2cvc2_indicator: Option<i8>,
    pub ssl_card_number: cards::CardNumber,
    pub ssl_exp_date: String,
    pub ssl_cvv2cvc2: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_avs_address: String,
    pub ssl_avs_zip: String,
    pub ssl_invoice_number: String,


    pub ssl_first_name:Option<String>,
    pub ssl_last_name:Option<String> ,
    pub ssl_address2:Option<String>,
    pub ssl_city:Option<String>,
    pub ssl_state:Option<String>,
    pub ssl_country:Option<String>,
    pub ssl_phone:Option<String>,
    pub ssl_ship_to_company:Option<String>,
    pub ssl_ship_to_first_name:Option<String>,
    pub ssl_ship_to_last_name: Option<String>,
    pub ssl_ship_to_address1: Option<String>,
    pub ssl_ship_to_address2: Option<String>,
    pub ssl_ship_to_city: Option<String>,
    pub ssl_ship_to_zip: Option<String>,
    pub ssl_ship_to_country: Option<String>,
    pub ssl_ship_to_phone: Option<String>,
    pub ssl_customer_code: Option<String>,
    pub ssl_salestax: f64,
    pub ssl_merchant_txn_id: Option<String>,
    pub ssl_description: Option<String>,
    pub ssl_dynamic_dba: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq, Clone)]
pub struct ElavonCreditCardAuthOnlyReq {
    pub ssl_transaction_type: String, //ccauthonly
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_vendor_id: Secret<String>,
    pub ssl_email: Email,

    pub ssl_cvv2cvc2_indicator: Option<i8>,
    pub ssl_card_number: cards::CardNumber,
    pub ssl_exp_date: Secret<String>,
    pub ssl_cvv2cvc2: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_invoice_number: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ElavonCreditCardCompletionReq {
    pub ssl_transaction_type: String, //cccomplete
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_vendor_id: Secret<String>,

    pub ssl_txn_id: String,
    pub ssl_amount: StringMajorUnit,      // not needed for full completion
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ElavonCreditCardRefund {
    pub ssl_transaction_type: String, //ccreturn
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_vendor_id: Secret<String>,

    pub ssl_txn_id: String,
    pub ssl_amount: StringMajorUnit,      // not needed for full refund
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ElavonCreditCardViod{
    pub ssl_transaction_type: String, //ccvoid
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_vendor_id: Secret<String>,

    pub ssl_txn_id: String,
}
#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ElavonCreditCardResponse{  // sale / authOnly / Completion / void / Refund
    pub ssl_issuer_response: i64,
    pub ssl_last_name: String,
    pub ssl_company: String,
    pub ssl_phone: String,
    pub ssl_card_number: cards::CardNumber,
    pub ssl_departure_date: String,
    pub ssl_oar_data: String,
    pub ssl_result: String,
    pub ssl_txn_id: String,
    pub ssl_loyalty_program: String,
    pub ssl_avs_response: String,
    pub ssl_approval_code: String,
    pub ssl_account_status: String,
    pub ssl_email: Email,
    pub ssl_amount: StringMajorUnit,
    pub ssl_avs_zip: String,
    pub ssl_txn_time: String,
    pub ssl_description: String,
    pub ssl_vendor_id: String,
    pub ssl_exp_date: String,
    pub ssl_card_short_description: String,
    pub ssl_completion_date: String,
    pub ssl_address2: String,
    pub ssl_get_token: String,
    pub ssl_customer_code: String,
    pub ssl_country: String,
    pub ssl_card_type: String,
    pub ssl_access_code: String,
    pub ssl_transaction_type: String,
    pub ssl_loyalty_account_balance: String,
    pub ssl_salestax: String,
    pub ssl_avs_address: String,
    pub merchant_1: String,
    pub ssl_account_balance: String,
    pub ssl_ps2000_data: String,
    pub ssl_state: String,
    pub ssl_ship_to_zip: String,
    pub ssl_city: String,
    pub ssl_result_message: String,
    pub ssl_first_name: String,
    pub ssl_invoice_number: String,
    pub ssl_ship_to_address1: String,
    pub ssl_cvv2_response: String,
    pub ssl_tender_amount: String,
    pub ssl_partner_app_id: String,
}

#[derive( Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum ElavonResponse { 
    Success(ElavonCreditCardResponse),
    Error(ElavonErrorResponse),
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Eq, Serialize, Deserialize, PartialEq)]

pub struct ElavonErrorResponse { // change it to camelCase  // Error Response
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    #[serde(rename = "errorCode")]
    pub error_code: String,
    #[serde(rename = "errorName")]
    pub error_name: String,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
}







//Auth Struct
pub struct ElavonAuthType {
    pub(super) ssl_account_id: Secret<String>,
    pub(super) ssl_user_id: Secret<String>,
    pub(super) ssl_pin: Secret<String>,
    pub(super) ssl_vendor_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ElavonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                ssl_account_id: key1.to_owned(),
                ssl_user_id: api_secret.to_owned(),
                ssl_pin:  api_key.to_owned(),
                ssl_vendor_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ElavonPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

#[derive(Debug, Display, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElavonTransactionStatus {
    Success,
    Failure,
    Confirmed,
    OnHold,
    Pending,
    Deactivated,
    Progress,
}

impl From<ElavonPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ElavonPaymentStatus) -> Self {
        match item {
            ElavonPaymentStatus::Succeeded => Self::Charged,
            ElavonPaymentStatus::Failed => Self::Failure,
            ElavonPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct ElavonPaymentsResponse {
//     status: ElavonPaymentStatus,
//     id: String,
// }


pub fn get_error_response(result: ElavonErrorResponse, status_code: u16) -> ErrorResponse {
    let error_code = result.error_code;
    let error_reason = result.error_message;

    ErrorResponse {
        code: error_code.to_string(),
        message: error_reason.clone(),
        reason: Some(error_reason),
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    }
}


impl TryFrom<&ElavonRouterData<&PaymentsAuthorizeRouterData>> for ElavonCreditCardAuthOnlyReq {   // auth req
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {

        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        let email = item.router_data.get_billing_email()?;
        
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = ElavonCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                let card_exp = get_exp_date(card.expiry_month,card.expiry_year);

                Ok(Self { 
                    ssl_transaction_type: "ccauthonly".to_string(), 
                    ssl_account_id: auth.ssl_account_id, 
                    ssl_user_id: auth.ssl_user_id, 
                    ssl_pin: auth.ssl_pin, 
                    ssl_vendor_id: auth.ssl_vendor_id, 
                    ssl_email: email, 
                    ssl_cvv2cvc2_indicator: Some(1),  // how to get it
                    ssl_card_number: card.number, 
                    ssl_exp_date: card_exp, // MMYY
                    ssl_cvv2cvc2: card.cvc, 
                    ssl_amount: item.amount.clone(), 
                    ssl_invoice_number: item.router_data.connector_request_reference_id.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}


pub fn get_authorize_body(item: &ElavonRouterData<&PaymentsAuthorizeRouterData>) -> Result<Vec<u8>, Error> {

    let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
    let email = item.router_data.get_billing_email()?;

    match item.router_data.request.payment_method_data.clone() {
        PaymentMethodData::Card(req_card) => {
            let card = ElavonCard {
                number: req_card.card_number,
                expiry_month: req_card.card_exp_month,
                expiry_year: req_card.card_exp_year,
                cvc: req_card.card_cvc,
                complete: item.router_data.request.is_auto_capture()?,
            };
            let card_exp = get_exp_date(card.expiry_month,card.expiry_year);

            let transaction_data = format!(
                "
                <txn>
                    <ssl_transaction_type>ccauthonly</ssl_transaction_type>
                    <ssl_account_id>{}</ssl_account_id>
                    <ssl_user_id>{}</ssl_user_id>
                    <ssl_email>{}</ssl_email>
                    <ssl_pin>{}</ssl_pin>
                    <ssl_vendor_id>{}</ssl_vendor_id>
                    <ssl_card_number>{}</ssl_card_number>
                    <ssl_exp_date>{}</ssl_exp_date>
                    <ssl_amount>{:?}</ssl_amount>
                    <ssl_cvv2cvc2>{}</ssl_cvv2cvc2>
                    <ssl_invoice_number>{}</ssl_invoice_number>
                </txn>
                ", 
                auth.ssl_account_id.clone().expose(),
                auth.ssl_user_id.clone().expose(),
                email.peek(),
                auth.ssl_pin.clone().expose(),
                auth.ssl_vendor_id.clone().expose(),
                card.number.peek(),
                card_exp.peek(),
                item.amount, 
                card.cvc.peek(), 
                item.router_data.connector_request_reference_id.clone(),
            );
        
            Ok(transaction_data.as_bytes().to_vec())
        }
        _=>return Err(errors::ConnectorError::NotImplemented("Elavon".to_string()))?
    }
}


pub fn get_ccsale_body(item: &ElavonRouterData<&PaymentsAuthorizeRouterData>) -> Result<Vec<u8>, Error> {

    let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
    let email = item.router_data.get_billing_email()?;

    match item.router_data.request.payment_method_data.clone() {
        PaymentMethodData::Card(req_card) => {
            let card = ElavonCard {
                number: req_card.card_number,
                expiry_month: req_card.card_exp_month,
                expiry_year: req_card.card_exp_year,
                cvc: req_card.card_cvc,
                complete: item.router_data.request.is_auto_capture()?,
            };
            let card_exp = get_exp_date(card.expiry_month,card.expiry_year);

            let transaction_data = format!(
                "
                <txn>
                    <ssl_transaction_type>ccsale</ssl_transaction_type>
                    <ssl_account_id>{}</ssl_account_id>
                    <ssl_user_id>{}</ssl_user_id>
                    <ssl_email>{}</ssl_email>
                    <ssl_pin>{}</ssl_pin>
                    <ssl_vendor_id>{}</ssl_vendor_id>
                    <ssl_card_number>{}</ssl_card_number>
                    <ssl_exp_date>{}</ssl_exp_date>
                    <ssl_amount>{:?}</ssl_amount>
                    <ssl_cvv2cvc2>{}</ssl_cvv2cvc2>
                    <ssl_invoice_number>{}</ssl_invoice_number>
                </txn>
                ", 
                auth.ssl_account_id.clone().expose(),
                auth.ssl_user_id.clone().expose(),
                email.peek(),
                auth.ssl_pin.clone().expose(),
                auth.ssl_vendor_id.clone().expose(),
                card.number.peek(),
                card_exp.peek(),
                item.amount, 
                card.cvc.peek(), 
                item.router_data.connector_request_reference_id.clone(),
            );
        
            Ok(transaction_data.as_bytes().to_vec())
        }
        _=>return Err(errors::ConnectorError::NotImplemented("Elavon".to_string()))?
    }
}



impl<F, T> TryFrom<ResponseRouterData<F, ElavonResponse, T, PaymentsResponseData>>    // auth res
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ElavonResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {

        match item.response {
            ElavonResponse::Success(response) => Ok(Self{
                // status: common_enums::AttemptStatus::from(response.ssl_issuer_response),
                status: common_enums::AttemptStatus::Authorizing, // hard coded
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.ssl_txn_id.clone()),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.ssl_txn_id.to_owned()),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            ElavonResponse::Error(error_response) => {
                let response = Err(get_error_response(error_response, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    
        // Ok(Self {
        //     status: common_enums::AttemptStatus::from(item.response.ssl_result_message),
        //     response: Ok(PaymentsResponseData::TransactionResponse {
        //         resource_id: ResponseId::ConnectorTransactionId(item.response.ssl_txn_id),
        //         redirection_data: None,
        //         mandate_reference: None,
        //         connector_metadata: None,
        //         network_txn_id: None,
        //         connector_response_reference_id: None,
        //         incremental_authorization_allowed: None,
        //         charge_id: None,
        //     }),
        //     ..item.data
        // })
    }
}


impl TryFrom<&ElavonRouterData<&PaymentsCaptureRouterData>> for ElavonCreditCardCompletionReq {  //cap req
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        // let email = item.router_data.get_billing_email()?;

        Ok(Self{
            ssl_transaction_type: "cccomplete".to_string(), 
            ssl_account_id: auth.ssl_account_id,
            ssl_user_id: auth.ssl_user_id,
            ssl_pin: auth.ssl_pin,
            ssl_vendor_id: auth.ssl_vendor_id,
            ssl_txn_id: item.router_data.connector_request_reference_id.clone(), // comes from auth response
            ssl_amount: item.amount.clone(), 
        })
    }
}


/*
// impl<F, T> TryFrom<ResponseRouterData<F, ElavonResponse, T, PaymentsResponseData>>    // tyep of auth res
for RouterData<F, T, PaymentsResponseData>
*/

// impl<F> TryFrom<ResponseRouterData<F, ElavonResponse, PaymentsCaptureData, PaymentsResponseData>> // cap res
//     for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: ResponseRouterData<
//             F,
//             ElavonResponse,
//             PaymentsCaptureData,
//             PaymentsResponseData,
//         >,
//     ) -> Result<Self, Self::Error> {

//     }
// }




//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ElavonRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&ElavonRouterData<&RefundsRouterData<F>>> for ElavonRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ElavonRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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


