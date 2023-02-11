use common_utils::ext_traits::ValueExt;
use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use crate::{
    connector::utils::{self, AddressDetailsData, PaymentsRequestData},
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::{enums, self}}, logger, 
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    action: String,
    authorization_amount: i64,
    billing_address: BillingAddress,
    card: CardDetails
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BillingAddress {
    first_name: String,
    last_name: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CardDetails {
    card_type: String,
    name_on_card: String,
    account_number: String,
    expire_month: String,
    expire_year: String,
    card_verification_value: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let todo_action = match item.request.capture_method {
            Some(storage::enums::CaptureMethod::Automatic) => "sale",
            _ => "authorize"
        };
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let action = todo_action.to_string();
                let authorization_amount = item.request.amount;
                let address_details = item.get_billing()?
                    .address
                    .as_ref()
                    .ok_or_else(utils::missing_field_err("billing.address"))?;
                let billing_address = BillingAddress {
                    first_name: address_details.get_first_name()?.to_owned().peek().to_string(),
                    last_name: address_details.get_last_name()?.to_owned().peek().to_string(),
                };
                let card= CardDetails {
                    card_type: String::from("visa"),
                    name_on_card: ccard.card_holder_name.peek().clone(),
                    account_number: ccard.card_number.peek().clone(),
                    expire_month: ccard.card_exp_month.peek().clone(),
                    expire_year: ccard.card_exp_year.peek().clone(),
                    card_verification_value: ccard.card_cvc.peek().clone()
                };
                Ok(Self {
                    action,
                    authorization_amount,
                    billing_address,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum FortePaymentStatus {
    A,
    D,
    #[default]
    E
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::A => Self::Charged,
            FortePaymentStatus::D => Self::Failure,
            FortePaymentStatus::E => Self::Failure,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    transaction_id: String,
    response: ResponseDetails,
    authorization_code: String,
    action: String
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseDetails {
    response_type: FortePaymentStatus
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentMetadata {
    pub authorization_code: String
}

pub fn convert_status(item: FortePaymentStatus, action: String) -> enums::AttemptStatus {
    if action == "sale" {
        match item {
            FortePaymentStatus::A => enums::AttemptStatus::Charged,
            FortePaymentStatus::D => enums::AttemptStatus::Failure,
            FortePaymentStatus::E => enums::AttemptStatus::Failure,
        }
    }
    else if action == "authorize" {
        match item {
            FortePaymentStatus::A => enums::AttemptStatus::Authorized,
            FortePaymentStatus::D => enums::AttemptStatus::Failure,
            FortePaymentStatus::E => enums::AttemptStatus::AuthorizationFailed,
        }
    }
    else if action == "void" {
        match item {
            FortePaymentStatus::A => enums::AttemptStatus::Voided,
            FortePaymentStatus::D => enums::AttemptStatus::Failure,
            FortePaymentStatus::E => enums::AttemptStatus::Failure,
        }
    }
    else {
        match item {
            FortePaymentStatus::A => enums::AttemptStatus::Charged,
            FortePaymentStatus::D => enums::AttemptStatus::Failure,
            FortePaymentStatus::E => enums::AttemptStatus::Failure,
        }
    }
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: convert_status(item.response.response.response_type, item.response.action),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: Some(
                    serde_json::to_value( PaymentMetadata {
                        authorization_code: item.response.authorization_code
                    })
                    .into_report()
                    .change_context(errors::ParsingError)?,
                ),
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize)]
pub struct ForteCancelRequest {
    action: String,
    authorization_code: String,
    entered_by: String
}

impl TryFrom<&types::PaymentsCancelRouterData> for ForteCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let metadata = item.request.connector_metadata
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let payment_metadata: PaymentMetadata = metadata
            .parse_value("PaymentMetadata")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        println!("AUTH_CODE - {}", payment_metadata.authorization_code);
        Ok(Self {
            action: String::from("void"),
            authorization_code: payment_metadata.authorization_code,
            entered_by: String::from("aditya")
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        println!("Parth2");
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        println!("Parth3");
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        println!("Parth4");
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {}
