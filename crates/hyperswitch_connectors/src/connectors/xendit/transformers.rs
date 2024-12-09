use common_enums::enums;
use common_utils::{id_type::CustomerId, pii::Email, types::FloatMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, RefundsRouterData,
        TokenizationRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{self, PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};

//TODO: Fill the struct with respective fields
pub struct XenditRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for XenditRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
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
    pub amount: FloatMajorUnit,
    pub currency: String,
    pub payment_method_id: String,
    pub customer_id: String,
    pub description: String,
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
        let amount = item.amount;

        match item.router_data.request.payment_method_data.clone() {
            // PaymentMethodData::Card(req_card) => {
            //     let card = XenditCard {
            //         number: req_card.card_number,
            //         expiry_month: req_card.card_exp_month,
            //         expiry_year: req_card.card_exp_year,
            //         cvc: req_card.card_cvc,
            //         complete: item.router_data.request.is_auto_capture()?,
            //     };

            //     unimplemented!()

            //     // Ok(Self {
            //     //     amount: item.amount.clone(),
            //     //     card,
            //     // })
            // }
            PaymentMethodData::BankTransfer(bank_transfer_data) => match *bank_transfer_data {
                BankTransferData::LocalBankTransfer { bank_code } => {
                    let customer_id = item.router_data.get_connector_customer_id()?;
                    let pm_token = item.router_data.get_payment_method_token()?;
                    println!("^^^^^^^^conn_token{:?}", pm_token);

                    Ok(Self {
                        amount,
                        currency: "PHP".to_string(),
                        payment_method_id: match pm_token {
                            PaymentMethodToken::Token(token) => token.expose(),
                            PaymentMethodToken::ApplePayDecrypt(_) => Err(
                                unimplemented_payment_method!("Apple Pay", "Simplified", "Xendit"),
                            )?,
                            PaymentMethodToken::PazeDecrypt(_) => {
                                Err(unimplemented_payment_method!("Paze", "Xendit"))?
                            }
                        },
                        customer_id,
                        description: "FOO BAR".to_string(),
                    })
                }
                BankTransferData::AchBankTransfer { .. }
                | BankTransferData::SepaBankTransfer { .. }
                | BankTransferData::BacsBankTransfer { .. }
                | BankTransferData::MultibancoBankTransfer { .. }
                | BankTransferData::PermataBankTransfer { .. }
                | BankTransferData::BcaBankTransfer { .. }
                | BankTransferData::BniVaBankTransfer { .. }
                | BankTransferData::BriVaBankTransfer { .. }
                | BankTransferData::CimbVaBankTransfer { .. }
                | BankTransferData::DanamonVaBankTransfer { .. }
                | BankTransferData::MandiriVaBankTransfer { .. }
                | BankTransferData::Pix { .. }
            | BankTransferData::Pse {} => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
            },
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
    Pending,
    RequiresAction,
}

impl From<XenditPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: XenditPaymentStatus) -> Self {
        match item {
            XenditPaymentStatus::Succeeded => Self::Charged,
            XenditPaymentStatus::Failed => Self::Failure,
            XenditPaymentStatus::Processing
            | XenditPaymentStatus::Pending
            | XenditPaymentStatus::RequiresAction => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct XenditPaymentsResponse {
    status: XenditPaymentStatus,
    id: String,
    reference_id: String,
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
                connector_response_reference_id: Some(item.response.reference_id),
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
    pub amount: FloatMajorUnit,
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
pub struct XenditCustomerIndividualDetail {
    pub given_names: Secret<String>,
    pub surname: Secret<String>,
}

impl TryFrom<&ConnectorCustomerRouterData> for XenditCustomerIndividualDetail {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        // if item.request.name.is_none(){
        //     Err(errors::ConnectorError::MissingRequiredField {
        //         field_name: "name",
        //     }
        //     .into());
        // }

        Ok(Self {
            given_names: item.get_billing_first_name()?,
            surname: item.get_billing_last_name()?,
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
pub struct XenditCustomerRequest {
    pub reference_id: CustomerId,
    #[serde(rename = "type")]
    pub customer_type: String,
    pub individual_detail: Option<XenditCustomerIndividualDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<Secret<String>>,
}

impl TryFrom<&ConnectorCustomerRouterData> for XenditCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            reference_id: item.get_customer_id()?,
            customer_type: "INDIVIDUAL".to_string(),
            individual_detail: Some(XenditCustomerIndividualDetail::try_from(item)?),
            email: item.request.email.to_owned(),
            phone: item.request.phone.to_owned(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XenditCustomerResponse {
    pub id: String,
    pub reference_id: CustomerId,
    #[serde(rename = "type")]
    pub customer_type: String,
    pub individual_detail: Option<XenditCustomerIndividualDetail>,
    // pub business_detail: Option<XenditCustomerBusinessDetail>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
}

impl<F, T> TryFrom<ResponseRouterData<F, XenditCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, XenditCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id,
            }),
            ..item.data
        })
    }
}

// Xendit Direct Debit

// Step 1: Create Customer

// Step 2: Initialize Linked Account Tokenization

pub enum XenditLATStatus {
    SUCCESS,
    PENDING,
    FAILED,
}

pub enum XenditChannelCode {
    DCBRI,
    BCAONEKLIK,
    BABPI,
    BPIRECURRING,
    BAUBP,
    UBPEADA,
    BABBL,
    BABAY,
    BAKTB,
    BASCB,
}

pub enum XenditDirectDebitUsability {
    SINGLEUSE,
    MULTIPLEUSE,
}

pub enum XenditPaymentMethodStatus {
    REQUIRESACTION,
    ACTIVE,
    PENDING,
}

pub struct XenditLATDebitCardProperties {
    pub account_mobile_number: Secret<String>,
    pub card_last_four: Secret<String>, // Card's last four digits
    pub card_expiry: Secret<String>,
    pub account_email: Email,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XenditLATBankAccountProperties {
    pub success_return_url: String,
    pub failure_return_url: String,
}

pub struct XenditLATBCAOneKlikProperties {
    pub account_mobile_number: Secret<String>,
    pub success_redirect_url: String,
    pub failure_redirect_url: Option<String>,
    pub callback_url: Option<String>,
}

// Step (2.1): Sending LAT Request

#[derive(Debug, Deserialize, Serialize)]
pub struct XenditDirectDebitPayload {
    pub channel_code: String,
    pub channel_properties: XenditLATBankAccountProperties,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct XenditLinkedAccountTokenizationRequest {
    #[serde(rename = "type")]
    pub action_type: String,
    pub direct_debit: XenditDirectDebitPayload,
    pub reusability: String,
    pub customer_id: String,
}

impl TryFrom<&TokenizationRouterData> for XenditLinkedAccountTokenizationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &TokenizationRouterData) -> Result<Self, Self::Error> {
        let customer_id = item.get_connector_customer_id()?;

        let direct_debit = XenditDirectDebitPayload {
            channel_code: "BPI".to_string(),
            channel_properties: XenditLATBankAccountProperties {
                success_return_url: "https://google.com/success".to_string(),
                failure_return_url: "https://google.com/failiure".to_string(),
            },
        };

        Ok(Self {
            action_type: "DIRECT_DEBIT".to_string(),
            direct_debit,
            reusability: "MULTIPLE_USE".to_string(),
            customer_id,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XenditLATActions {
    pub method: String,
    pub url_type: String,
    pub action: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]

pub struct XenditLinkedAccountTokenizationResponse {
    pub id: Secret<String>, // payment method id
    pub business_id: String,
    pub customer_id: String,
    pub reference_id: String,
    pub status: String,
    pub actions: Vec<XenditLATActions>,
    // METADATA
}

impl<F, T>
    TryFrom<ResponseRouterData<F, XenditLinkedAccountTokenizationResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            XenditLinkedAccountTokenizationResponse,
            T,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

// Step (2.2) - Validation of Linked Account Tokenization
// For debit card we have to send them OTP
// For bank account Xendit LAT Response returns auth url from where customer has to authorize
// This step might have been skipped in payments api v2

pub struct XenditDebitCardValidateRequest {
    pub otp_code: String,
}

pub struct XenditLATValidationResponse {
    pub id: String,
    pub customer_id: String,
    pub channel_code: XenditChannelCode,
    pub status: XenditLATStatus,
    // METADATA
}

// Step (2.3) - Retrieve the list of accounts

pub struct XenditLinkedAccount<T> {
    pub channel_code: XenditChannelCode,
    pub id: String,
    pub properties: T,
    pub link_type: String, // Whether Debit Card, Bank acc, wallet, etc
}

// pub struct XenditLinkedAccountResponse{
//     pub accounts: Vec<XenditLinkedAccount<T>>
// }

// Xendit Payment with linked payment methods
