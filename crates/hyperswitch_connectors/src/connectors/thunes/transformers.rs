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

//----Quotation-----------

// #[serde(rename_all = "UPPERCASE")]
// #[serde(rename_all = "snake_case")]
pub enum QuotationMode{
    SourceAmount,  //SOURCE_AMOUNT
    DestinationAmount,
}

pub enum TransactionType{
    C2C, //Quotation or Transaction is from an individual end user to an individual end user
    C2B, //Quotation or Transaction is from an individual end user to a business
    B2C, //Quotation or Transaction is from a business to an individual end user
    B2B, //Quotation or Transaction is from a business to a business
}

pub struct SourceInfo{
    pub country_iso_code: enums::CountryAlpha3,
    pub currency: enums::Currency,
    pub amount: Option<f64>,
}

pub struct DestinationInfo{
    pub currency: enums::Currency,
    pub amount: Option<f64>,
}

pub struct QuotationRequest{
    pub external_id: String,
    pub payer_id: u64,   // size is not specified in the docs
    pub mode: QuotationMode,
    pub transaction_type: TransactionType,  // not present in v1
    pub source: SourceInfo,
    pub destination: DestinationInfo,
}



pub struct ServiceType{
    pub id: i64,
    pub name: String,
}


pub struct CreditPartyVerification{
    pub credit_party_identifiers_accepted: Vec<String>,
    pub required_beneficiary_fields: Vec<String>,
}
pub struct PayerType{
    pub id: i64,
    pub name: String,
    pub precision: i64,
    pub increment: i64,
    pub currency: enums::Currency,
    pub country_iso_code: enums::CountryAlpha3,
    pub minimum_transaction_amount: f64,
    pub maximum_transaction_amount: Option<f64>,
    pub service: ServiceType,
    pub credit_party_identifiers_accepted: Vec<String>,
    pub required_sender_fields: Vec<String>,
    pub required_beneficiary_fields: Vec<String>,
    pub credit_party_information: Vec<String>,
    pub credit_party_verification: CreditPartyVerification,
}

pub struct AmountWithCountry{
    pub currency: enums::Currency,
    pub amount: f64,
}

pub struct QuotationResponse{
    pub id: i64,
    pub external_id: String,
    pub payer: PayerType,
    pub mode: QuotationMode,
    pub transaction_type: TransactionType,
    pub source: SourceInfo,
    pub destination: AmountWithCountry,
    pub sent_amount: AmountWithCountry,
    pub wholesale_fx_rate: f64,
    pub fee: AmountWithCountry,
    pub creation_date: String,
    pub expiration_date: String,
}

//----Transaction----------

pub struct TransactionRequest{

}

pub struct CreditPartyIdentifier{
    pub msisdn : Option<String>,
    pub bank_account_number: Option<String>,
    pub iban: Option<String>,
    pub clabe: Option<String>,
    pub cbu: Option<String>,
    pub cbu_alias: Option<String>,
    pub swift_bic_code: Option<String>,
    pub bik_code: Option<String>,
    pub ifs_code: Option<String>,
    pub sort_code: Option<String>,
    pub aba_routing_number: Option<String>,
    pub bsb_number: Option<String>,
    pub branch_number: Option<String>,
    pub routing_code: Option<String>,
    pub entity_tt_id: Option<String>,
    pub account_type: Option<String>,
    pub account_number: Option<String>,
    pub email: Option<String>,
    pub card_number: Option<String>,

}

pub struct Sender{
    pub lastname: Option<String>,
    pub lastname2: Option<String>,
    pub middlename: Option<String>,
    pub firstname: Option<String>,
    pub nativename: Option<String>,
    pub nationality_country_iso_code: Option<String>,
    pub code: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_of_birth_iso_code: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country_iso_code: Option<String>,
    pub msisdn: Option<String>,
    pub email: Option<String>,
    pub id_type: Option<String>,
    pub id_country_iso_code: Option<String>,
    pub id_number: Option<String>,
    pub id_delivery_date: Option<String>,
    pub id_expiration_date: Option<String>,
    pub occupation: Option<String>,
    pub bank_account_number: Option<String>,
    pub province_state: Option<String>,
    pub beneficiary_relationship: Option<String>,
    pub source_of_funds: Option<String>,
} 

pub struct Beneficiary{
    pub lastname: Option<String>,
    pub lastname2: Option<String>,
    pub middlename: Option<String>,
    pub firstname: Option<String>,
    pub nativename: Option<String>,
    pub nationality_country_i: Option<String>,
    pub code: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_of_birth_iso_code: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country_iso_code: Option<String>,
    pub msisdn: Option<String>,
    pub email: Option<String>,
    pub id_type: Option<String>,
    pub id_country_iso_code: Option<String>,
    pub id_number: Option<String>,
    pub id_delivery_date: Option<String>,
    pub id_expiration_date: Option<String>,
    pub occupation: Option<String>,
    pub bank_account_holder_name: Option<String>,
    pub province_state: Option<String>,
}

pub struct TransactionResponse{
    pub id: i64,
    pub status: String,
    pub status_message: String,
    pub status_class: String,
    pub status_class_message: String,
    pub external_id: String,
    pub external_code: Option<String>,
    pub payer_transaction_reference: Option<String>,
    pub payer_transaction_code: Option<String>,
    pub creation_date: String,
    pub expiration_date: String,
    pub credit_party_identifier: CreditPartyIdentifier,
    pub source: SourceInfo,
    pub destination: DestinationInfo,
    pub payer: PayerType,
    pub sender: Sender,
    pub beneficiary: Beneficiary,
    pub callback_url: String,
    pub sent_amount: DestinationInfo,
    pub wholesale_fx_rate: f64,
    pub retail_rate: Option<f64>,
    pub retail_fee: Option<f64>,
    pub retail_fee_currency: enums::Currency,
    pub fee: DestinationInfo,
    pub purpose_of_remittance: Option<String>,
    pub additional_information_1: Option<String>,
    pub additional_information_2: Option<String>,
    pub additional_information_3: Option<String>,

}

//--------------------------

pub struct ThunesRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for ThunesRouterData<T> {
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
pub struct ThunesPaymentsRequest {
    amount: StringMinorUnit,
    card: ThunesCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ThunesCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ThunesRouterData<&PaymentsAuthorizeRouterData>> for ThunesPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThunesRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = ThunesCard {
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
pub struct ThunesAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ThunesAuthType {
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
pub enum ThunesPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ThunesPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ThunesPaymentStatus) -> Self {
        match item {
            ThunesPaymentStatus::Succeeded => Self::Charged,
            ThunesPaymentStatus::Failed => Self::Failure,
            ThunesPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThunesPaymentsResponse {
    status: ThunesPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, ThunesPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ThunesPaymentsResponse, T, PaymentsResponseData>,
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
pub struct ThunesRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&ThunesRouterData<&RefundsRouterData<F>>> for ThunesRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ThunesRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct ThunesErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
