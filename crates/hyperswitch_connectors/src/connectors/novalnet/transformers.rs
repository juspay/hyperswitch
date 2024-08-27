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
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct NovalnetRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NovalnetRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
<<<<<<< Updated upstream
=======
//TODO: DG change optional types and correct types
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Address {
    company: String,
    house_no: String,
    street: String,
    city: String,
    zip: String,
    country_code: String,
    state: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Merchant {
    signature: String,
    tariff: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Billing {
    company: String,
    house_no: String,
    street: String,
    city: String,
    zip: String,
    country_code: String,
    state: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Customer {
    first_name: String,
    last_name: String,
    email: String,
    tel: String,
    mobile: String,
    billing: Billing,
    customer_ip: String,
    birth_date: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaymentData {
    pan_hash: String,
    unique_id: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Transaction {
    test_mode: String,
    payment_type: String,
    amount: String,
    currency: String,
    order_no: String,
    hook_url: String,
    create_token: String,
    payment_data: PaymentData,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Custom {
    lang: String,
}

>>>>>>> Stashed changes
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct NovalnetPaymentsRequest {
    amount: StringMinorUnit,
    card: NovalnetCard,
<<<<<<< Updated upstream
=======
    merchant: Option<Merchant>,
    customer: Option<Customer>,
    transaction: Option<Transaction>,
    custom: Option<Custom>,
>>>>>>> Stashed changes
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NovalnetCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

<<<<<<< Updated upstream
impl TryFrom<&NovalnetRouterData<&PaymentsAuthorizeRouterData>> for NovalnetPaymentsRequest {
=======
impl TryFrom<&NovalnetRouterData<&PaymentsAuthorizeRouterData>> for NovalnetPaymentsRequest {//
>>>>>>> Stashed changes
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NovalnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
<<<<<<< Updated upstream
                let card = NovalnetCard {
=======
                let card = NovalnetCard {//
>>>>>>> Stashed changes
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
<<<<<<< Updated upstream
=======
                    //TODO: get data from item
                    merchant: None,
                    customer: None,
                    transaction: None,
                    custom: None,
>>>>>>> Stashed changes
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NovalnetAuthType {
    pub(super) api_key: Secret<String>,
}

<<<<<<< Updated upstream
impl TryFrom<&ConnectorAuthType> for NovalnetAuthType {
=======
impl TryFrom<&ConnectorAuthType> for NovalnetAuthType { //
>>>>>>> Stashed changes
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
<<<<<<< Updated upstream
pub enum NovalnetPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NovalnetPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NovalnetPaymentStatus) -> Self {
        match item {
            NovalnetPaymentStatus::Succeeded => Self::Charged,
            NovalnetPaymentStatus::Failed => Self::Failure,
            NovalnetPaymentStatus::Processing => Self::Authorizing,
=======
#[allow(non_camel_case_types)]
pub enum NovalnetPaymentStatus {
    SUCCESS,
    FAILURE,
    CONFIRMED,
    ON_HOLD,
    PENDING,
    #[default]
    DEACTIVATED,
}


impl From<NovalnetPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NovalnetPaymentStatus) -> Self {
        match item {
            NovalnetPaymentStatus::SUCCESS => Self::Charged,
            NovalnetPaymentStatus::CONFIRMED => Self::Authorizing,
            _ => Self::Failure,
>>>>>>> Stashed changes
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
<<<<<<< Updated upstream
pub struct NovalnetPaymentsResponse {
    status: NovalnetPaymentStatus,
=======
pub struct ResultData {
    redirect_url: String,
    status: NovalnetPaymentStatus,
    status_code: i32,
    status_text: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: String,
    status_code: i32,
    txn_secret: String,
    tid: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovalnetPaymentsResponse {
    result: ResultData,
    transaction: TransactionData,
>>>>>>> Stashed changes
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, NovalnetPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NovalnetPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
<<<<<<< Updated upstream
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
=======
            status: common_enums::AttemptStatus::from(item.response.result.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                // resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.tid),
>>>>>>> Stashed changes
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
pub struct NovalnetRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&NovalnetRouterData<&RefundsRouterData<F>>> for NovalnetRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NovalnetRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct NovalnetErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
