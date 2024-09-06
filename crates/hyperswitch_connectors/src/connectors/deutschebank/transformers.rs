use common_enums::enums;
use common_utils::{pii::Email, types::StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, RouterData},
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
pub struct DeutschebankRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for DeutschebankRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct DeutschebankAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) merchant_id: Secret<String>,
    pub(super) client_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DeutschebankAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey { api_key, key1, api_secret } => Ok(Self {
                client_id: api_key.to_owned(),
                merchant_id: key1.to_owned(),
                client_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DeutschebankAccessTokenRequest {
    pub grant_type: String,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub scope: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct DeutschebankAccessTokenResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
    pub expires_on: i64,
    pub scope: String,
    pub token_type: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, DeutschebankAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DeutschebankAccessTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeutschebankSEPAApproval {
    Click,
    Email,
    Sms,
    Dynamic,
}


//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankPaymentsRequest {
    approval_by: DeutschebankSEPAApproval,
    email_address: Email,
    iban: Secret<String>,
}

impl TryFrom<&DeutschebankRouterData<&PaymentsAuthorizeRouterData>>
    for DeutschebankPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(bank_debit_data) => {
                match bank_debit_data {
                    BankDebitData::SepaBankDebit { iban } => {
                        Ok(Self {
                            approval_by: DeutschebankSEPAApproval::Click,
                            email_address: item.router_data.request.get_email()?,
                            iban,
                        })
                    }
                    _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeutschebankPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<DeutschebankPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: DeutschebankPaymentStatus) -> Self {
        match item {
            DeutschebankPaymentStatus::Succeeded => Self::Charged,
            DeutschebankPaymentStatus::Failed => Self::Failure,
            DeutschebankPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankPaymentsResponse {
    status: DeutschebankPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, DeutschebankPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DeutschebankPaymentsResponse, T, PaymentsResponseData>,
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
pub struct DeutschebankRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&DeutschebankRouterData<&RefundsRouterData<F>>> for DeutschebankRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DeutschebankRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct DeutschebankErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
