use cards::CardNumber;
use common_enums::enums;
use common_utils::types::{StringMajorUnit, StringMinorUnit};
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

use crate::utils::RouterData as OtherRouterData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
    utils::{self, missing_field_err, CardData as CardDataUtil},
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
pub enum PaymentType {
    Card,
    Applepay,
    Googlepay,
}

//TODO: Fill the struct with respective fields

#[derive(Debug, Serialize, Deserialize)]
pub enum NovalNetPaymentTypes {
    CREDITCARD,
    DEBITCARD,
}
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
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct NovalNetCard {
    card_number: CardNumber,
    card_expiry_month: Secret<String>,
    card_expiry_year: Secret<String>,
    card_cvc: Option<Secret<String>>,
    card_holder: Option<Secret<String>>,
    pan_hash: Option<String>,
    unique_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NovalNetPaymentData {
    PaymentCard(NovalNetCard),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovalNetTransaction {
    test_mode: i8,
    payment_type: NovalNetPaymentTypes,
    amount: Option<StringMinorUnit>,
    currency: Option<String>,
    order_no: Option<String>,
    create_token: Option<i8>,
    payment_data: NovalNetPaymentData,
    hook_url: Option<String>,
    return_url: Option<String>,
    error_return_url: Option<String>,
}
// #[derive(Default, Debug, Serialize)]
// pub struct Custom {
//     lang: String,
// }

#[derive(Debug, Serialize)]
pub struct NovalnetPaymentsRequest {
    merchant: Merchant,
    customer: Customer,
    transaction: NovalNetTransaction,
    // custom: Option<Custom>,
}
type Error = error_stack::Report<errors::ConnectorError>;
fn result_to_option(result: Result<String, Error>) -> Option<String> {
    result.ok()
}

impl TryFrom<&NovalnetRouterData<&PaymentsAuthorizeRouterData>> for NovalnetPaymentsRequest {
    //
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NovalnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let merchant = Merchant {
                    signature: "".to_string(),
                    tariff: "".to_string(),
                };

                let novalnet_card = NovalNetCard {
                    card_number: req_card.card_number,
                    card_expiry_month: req_card.card_exp_month,
                    card_expiry_year: req_card.card_exp_year,
                    card_cvc: Some(req_card.card_cvc),
                    card_holder: item.router_data.get_optional_billing_full_name(),
                    pan_hash: None,
                    unique_id: None,
                };

                let return_url = result_to_option(item.router_data.request.get_return_url());
                let hook_url = result_to_option(item.router_data.request.get_webhook_url());
                let transaction = NovalNetTransaction {
                    test_mode: 1,
                    payment_type: NovalNetPaymentTypes::CREDITCARD,
                    amount: Some(item.amount.clone()),
                    currency: Some(item.router_data.request.currency.to_string()),
                    order_no: Some(item.router_data.connector_request_reference_id.clone()),
                    hook_url: hook_url,
                    return_url: return_url.clone(),
                    error_return_url: return_url.clone(),
                    create_token: None,
                    payment_data: NovalNetPaymentData::PaymentCard(novalnet_card),
                };

                let billing = Billing {
                    company: "".to_string(),
                    house_no: "".to_string(),
                    street: "".to_string(),
                    city: "".to_string(),
                    zip: "".to_string(),
                    country_code: "".to_string(),
                    state: "".to_string(),
                };
                

                let customer = Customer {
                    first_name: "".to_string(),
                    last_name: "".to_string(),
                    email: "".to_string(),
                    tel: "".to_string(),
                    mobile: "".to_string(),
                    billing: billing,
                    customer_ip: "".to_string(),
                    birth_date: "".to_string(),
                };
                Ok(NovalnetPaymentsRequest {
                    merchant: merchant,
                    transaction: transaction,
                    customer: customer,
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

impl TryFrom<&ConnectorAuthType> for NovalnetAuthType {
    //
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
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            status: common_enums::AttemptStatus::from(item.response.result.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                // resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.tid),
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
