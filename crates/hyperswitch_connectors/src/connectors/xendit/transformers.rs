use common_enums::enums;
use common_utils::{crypto::OptionalEncryptableEmail, types::StringMinorUnit, id_type::CustomerId};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, ConnectorCustomerRouterData},
    
};
use common_utils::pii::Email;
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};

//TODO: Fill the struct with respective fields
pub struct XenditRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for XenditRouterData<T> {
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
pub struct XenditPaymentsRequest {
    amount: StringMinorUnit,
    card: XenditCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct XenditCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&XenditRouterData<&PaymentsAuthorizeRouterData>> for XenditPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &XenditRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = XenditCard {
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
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct XenditAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for XenditAuthType {
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
pub enum XenditPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<XenditPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: XenditPaymentStatus) -> Self {
        match item {
            XenditPaymentStatus::Succeeded => Self::Charged,
            XenditPaymentStatus::Failed => Self::Failure,
            XenditPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct XenditPaymentsResponse {
    status: XenditPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, XenditPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, XenditPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
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
pub struct XenditRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&XenditRouterData<&RefundsRouterData<F>>> for XenditRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &XenditRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct XenditErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// Xendit Customer


#[derive(Debug, Serialize, Deserialize)]
pub struct XenditCustomerIndividualDetail{
    pub given_names: Secret<String>,
    pub surname: Secret<String>
}

impl TryFrom<&ConnectorCustomerRouterData> for XenditCustomerIndividualDetail{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        
        // if item.request.name.is_none(){
        //     Err(errors::ConnectorError::MissingRequiredField {
        //         field_name: "name",
        //     }
        //     .into());
        // }

        Ok(Self{
            given_names: item.get_billing_full_name()?,
            surname: item.get_billing_last_name()?
        })
    }
}

// pub enum XenditCustomerBusinessType{
//     CORPORATION,
//     SOLEPROPRIETOR,
//     PARTNERSHIP,
//     COOPERATIVE,
//     TRUST,
//     NONPROFIT,
//     GOVERNMENT
// }

// reference-id = Merchant-provided identifier for the customer.
#[derive(Debug, Serialize)]
pub struct XenditCustomerRequest{
    pub reference_id: CustomerId,
    pub customer_type: String,
    pub individual_detail: Option<XenditCustomerIndividualDetail>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
}

impl TryFrom<&ConnectorCustomerRouterData> for XenditCustomerRequest{

    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {

        if item.request.email.is_none() && item.request.phone.is_none() {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "email or phone",
            }
            .into())
        } else {
            Ok(Self {
                reference_id:  item.get_customer_id()?,
                customer_type: "INDIVIDUAL".to_string(),
                individual_detail: Some(XenditCustomerIndividualDetail::try_from(item)?),
                email: item.request.email.to_owned(),
                phone: item.request.phone.to_owned(),
            })
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XenditCustomerResponse{
    pub customer_id: String,
    pub reference_id: Secret<String>,
    pub customer_type: String,
    pub individual_detail: Option<XenditCustomerIndividualDetail>,
    // pub business_detail: Option<XenditCustomerBusinessDetail>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
}


impl<F,T> TryFrom<ResponseRouterData<F, XenditCustomerResponse, T, PaymentsResponseData>> for RouterData<F,T,PaymentsResponseData>{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: ResponseRouterData<F, XenditCustomerResponse, T, PaymentsResponseData>) -> Result<Self, Self::Error> {

        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.customer_id,
            }),
            ..item.data
        })
    }
}




// Xendit Direct Debit

// Step 1: Create Customer

// Step 2: Initialize Linked Account Tokenization

pub enum XenditLATStatus{
    SUCCESS,
    PENDING,
    FAILED
}

pub enum XenditChannelCode{
    DCBRI,
    BCAONEKLIK,
    BABPI,
    BPIRECURRING,
    BAUBP,
    UBPEADA,
    BABBL,
    BABAY,
    BAKTB,
    BASCB
}

pub enum XenditDirectDebitUsability{
    SINGLEUSE,
    MULTIPLEUSE
}

pub enum XenditPaymentMethodStatus{
    REQUIRESACTION,
    ACTIVE,
    PENDING
}

pub struct XenditLATDebitCardProperties{
    pub account_mobile_number: Secret<String>,
    pub card_last_four: Secret<String>, // Card's last four digits
    pub card_expiry: Secret<String>,
    pub account_email: Email
}

pub struct XenditLATBankAccountProperties{
    pub success_redirect_url: String,
    pub failure_redirect_url: String,
}

pub struct XenditLATBCAOneKlikProperties{
    pub account_mobile_number: Secret<String>,
    pub success_redirect_url: String,
    pub failure_redirect_url: Option<String>,
    pub callback_url: Option<String>
}

// Step (2.1): Sending LAT Request

pub struct XenditDirectDebitPayload<T>{
    pub channel_code: XenditChannelCode,
    pub properties: T,
}
pub struct XenditLinkedAccountTokenizationRequest<T>{
    pub payment_method_type: String, // This would be "DIRECT_DEBIT" for this case
    pub direct_debit: XenditDirectDebitPayload<T>,
    pub reusability: XenditDirectDebitUsability,
    pub customer_id: String
    // METADATA
}

pub struct XenditLATActions{
    pub method: String,
    pub url_type: String,
    pub action: String,
    pub url: String,
}

pub struct XenditLinkedAccountTokenizationResponse{
    pub id: String,
    pub business_id: String,
    pub customer_id: String,
    pub reusability: XenditDirectDebitUsability,
    pub status: XenditPaymentMethodStatus,
    pub actions: Vec<XenditLATActions>
    // METADATA
}

// Step (2.2) - Validation of Linked Account Tokenization 
// For debit card we have to send them OTP
// For bank account Xendit LAT Response returns auth url from where customer has to authorize
// This step might have been skipped in payments api v2

pub struct XenditDebitCardValidateRequest{
    pub otp_code: String
}

pub struct XenditLATValidationResponse{
    pub id: String,
    pub customer_id: String,
    pub channel_code: XenditChannelCode,
    pub status: XenditLATStatus,
    // METADATA
}

// Step (2.3) - Retrieve the list of accounts

pub struct XenditLinkedAccount<T>{
    pub channel_code: XenditChannelCode,
    pub id: String,
    pub properties : T,
    pub link_type: String // Whether Debit Card, Bank acc, wallet, etc
}

// pub struct XenditLinkedAccountResponse{
//     pub accounts: Vec<XenditLinkedAccount<T>>
// }