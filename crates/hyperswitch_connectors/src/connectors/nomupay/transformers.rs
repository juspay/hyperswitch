use common_enums::{enums, Currency};
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
use serde::{ Deserialize, Serialize};


use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct NomupayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NomupayRouterData<T> {
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
pub struct NomupayPaymentsRequest {
    amount: StringMinorUnit,
    card: NomupayCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]

pub struct NomupayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub country: String,
    pub state_province: String,
    pub street: String,
    pub city: String,
    pub postal_code: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub profile_type: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub email_address: String,
    pub phone_number_country_code: Option<String>,
    pub phone_number: Option<String>,
    pub address : Address,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccout{    //1
    pub account_id: String,
    pub client_sub_account_id: String,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    pub bank_id: String,
    pub account_id: String,
    pub account_purpose: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAccountsType{
    pub country_code: String,
    pub currency_code: String,
    pub bank_id: String,
    pub bank_account_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethod {  //2
    pub country_code: String,
    pub currency_code: Currency,
    pub typee: String,    // type giving error
    pub display_name: String,
    pub bank_account: BankAccount,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {  //3
    pub source_d: String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo:  String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {  //4
    pub source_id: String,
    pub source_currency_code: Currency,
    pub destination_currency_code: Currency,
    pub amount: String,
    pub include_fee: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit { //5
    pub source_id:  String,
    pub id:  String,
    pub destination_id:  String,
    pub payment_reference:  String,
    pub amount:  String,
    pub currency_code:  Currency,
    pub purpose:  String,
    pub description:  String,
    pub internal_memo:  String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccoutResponse{
    pub account_id: String,
    pub id: String,
    pub client_sub_account_id: String,
    pub profile: Profile,
    pub virtual_accounts: Vec<VirtualAccountsType>,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethodResponse{
    pub parent_id: String,
    pub account_id: String,
    pub sub_account_id: String,
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub country_code: String,
    pub currency_code: Currency,
    pub display_name: String,
    pub typee: String,
    pub profile: Profile,
    pub bank_account: BankAccount,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub id: String,
    pub status:  String,
    pub created_on: String,
    pub last_updated: String,
    pub source_id:  String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: String,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeesType {
    pub typee: String,
    pub fees: StringMajorUnit,
    pub currency_code: Currency,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutQuoteResponse{
    pub source_id: String,
    pub destination_currency_code: Currency,
    pub amount: StringMajorUnit,
    pub source_currency_code: Currency,
    pub include_fee: bool,
    pub fees: Vec<FeesType>,
    pub payment_reference: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitResponse{
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub source_id: String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub field: String,
    pub message: String, 
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NomupayError {
    pub error_code: String,
    pub error_description: Option<String>,
    pub validation_errors: Option<Vec<Error>>
}






impl TryFrom<&NomupayRouterData<&PaymentsAuthorizeRouterData>> for NomupayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NomupayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = NomupayCard {
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

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NomupayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NomupayAuthType {
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
pub enum NomupayPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NomupayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NomupayPaymentStatus) -> Self {
        match item {
            NomupayPaymentStatus::Succeeded => Self::Charged,
            NomupayPaymentStatus::Failed => Self::Failure,
            NomupayPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NomupayPaymentsResponse {
    status: NomupayPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, NomupayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NomupayPaymentsResponse, T, PaymentsResponseData>,
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
pub struct NomupayRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&NomupayRouterData<&RefundsRouterData<F>>> for NomupayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NomupayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct NomupayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
