// use cards::CardNumber;
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
use masking::Secret;
use ring::agreement::PublicKey;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct ElavonRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for ElavonRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
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
    amount: StringMinorUnit,
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

impl TryFrom<&ElavonRouterData<&PaymentsAuthorizeRouterData>> for ElavonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = ElavonCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}


// enum TransactionType{

// }

pub struct ElavonCreditCardSaleReq {
    pub ssl_transaction_type: String, //ccsale
    pub ssl_account_id: i64,
    pub ssl_user_id: String,
    pub ssl_pin: String,
    pub ssl_vendor_id: i64,
    pub ssl_email: String,

    pub ssl_cvv2cvc2_indicator: Option<i8>,
    pub ssl_card_number: cards::CardNumber,
    pub ssl_exp_date: cards::CardExpiration,
    pub ssl_cvv2cvc2: cards::CardSecurityCode,
    pub ssl_amount: f64,
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

pub struct ElavonCreditCardAuthOnlyReq {
    pub ssl_transaction_type: String, //ccauthonly
    pub ssl_account_id: i64,
    pub ssl_user_id: String,
    pub ssl_pin: String,
    pub ssl_vendor_id: i64,
    pub ssl_email: String,

    pub ssl_cvv2cvc2_indicator: Option<i8>,
    pub ssl_card_number: cards::CardNumber,
    pub ssl_exp_date: cards::CardExpiration,
    pub ssl_cvv2cvc2: cards::CardSecurityCode,
    pub ssl_amount: f64,
    pub ssl_invoice_number: String,
}

pub struct ElavonCreditCardCompletionReq {
    pub ssl_transaction_type: String, //cccomplete
    pub ssl_account_id: i64,
    pub ssl_user_id: String,
    pub ssl_pin: String,
    pub ssl_vendor_id: i64,

    pub ssl_txn_id: String,
    pub ssl_amount: i64,      // not needed for full completion
}

pub struct ElavonCreditCardRefund {
    pub ssl_transaction_type: String, //ccreturn
    pub ssl_account_id: i64,
    pub ssl_user_id: String,
    pub ssl_pin: String,
    pub ssl_vendor_id: i64,

    pub ssl_txn_id: String,
    pub ssl_amount: i64,      // not needed for full refund
}

pub struct ElavonCreditCardViod{
    pub ssl_transaction_type: String, //ccvoid
    pub ssl_account_id: i64,
    pub ssl_user_id: String,
    pub ssl_pin: String,
    pub ssl_vendor_id: i64,

    pub ssl_txn_id: String,
}

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
    pub ssl_email: String,
    pub ssl_amount: f64,
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










pub struct ElavonAuthType {
    pub(super) ssl_account_id: Secret<String>,
    pub(super) ssl_user_id: Secret<String>,
    pub(super) ssl_pin: Secret<String>,
    pub(super) ssl_vendor_id: Secret<String>,
}

// impl TryFrom<&ConnectorAuthType> for ElavonAuthType {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
//         match auth_type {
//             ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
//                 api_key: api_key.to_owned(),
//             }),
//             _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
//         }
//     }
// }
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElavonPaymentsResponse {
    status: ElavonPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, ElavonPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ElavonPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ElavonRefundRequest {
    pub amount: StringMinorUnit,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ElavonErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
