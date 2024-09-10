use std::collections::HashMap;

use common_enums::enums;
use common_utils::{pii::Email, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{
        payments::{Authorize, Capture, CompleteAuthorize, PSync},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCompleteAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        AddressDetailsData, PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        RouterData as OtherRouterData,
    },
};

//TODO: Fill the struct with respective fields
pub struct DeutschebankRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for DeutschebankRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
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
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeutschebankSEPAApproval {
    Click,
    Email,
    Sms,
    Dynamic,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankPaymentsRequest {
    approval_by: DeutschebankSEPAApproval,
    email_address: Email,
    iban: Secret<String>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}

impl TryFrom<&DeutschebankRouterData<&PaymentsAuthorizeRouterData>>
    for DeutschebankPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let billing_address = item.router_data.get_billing_address()?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban }) => Ok(Self {
                approval_by: DeutschebankSEPAApproval::Click,
                email_address: item.router_data.request.get_email()?,
                iban,
                first_name: billing_address.get_first_name()?.clone(),
                last_name: billing_address.get_last_name()?.clone(),
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeutschebankSEPAMandateStatus {
    Created,
    PendingApproval,
    PendingSecondaryApproval,
    PendingReview,
    PendingSubmission,
    Submitted,
    Active,
    Failed,
    Discarded,
    Expired,
    Replaced,
}

impl From<DeutschebankSEPAMandateStatus> for common_enums::AttemptStatus {
    fn from(item: DeutschebankSEPAMandateStatus) -> Self {
        match item {
            DeutschebankSEPAMandateStatus::Active
            | DeutschebankSEPAMandateStatus::Created
            | DeutschebankSEPAMandateStatus::PendingApproval
            | DeutschebankSEPAMandateStatus::PendingSecondaryApproval
            | DeutschebankSEPAMandateStatus::PendingReview
            | DeutschebankSEPAMandateStatus::PendingSubmission
            | DeutschebankSEPAMandateStatus::Submitted => Self::AuthenticationPending,
            DeutschebankSEPAMandateStatus::Failed
            | DeutschebankSEPAMandateStatus::Discarded
            | DeutschebankSEPAMandateStatus::Expired
            | DeutschebankSEPAMandateStatus::Replaced => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankMandatePostResponse {
    rc: String,
    message: String,
    mandate_id: Option<String>,
    reference: Option<String>,
    approval_date: Option<String>,
    language: Option<String>,
    approval_by: Option<DeutschebankSEPAApproval>,
    state: Option<DeutschebankSEPAMandateStatus>,
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            DeutschebankMandatePostResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            DeutschebankMandatePostResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let signed_on = match item.response.approval_date {
            Some(date) => date.chars().take(10).collect(),
            None => "".to_string(),
        };
        Ok(Self {
            status: if item.response.rc == "0" {
                match item.response.state {
                    Some(state) => common_enums::AttemptStatus::from(state),
                    None => common_enums::AttemptStatus::Failure,
                }
            } else {
                common_enums::AttemptStatus::Failure
            },
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Some(RedirectForm::Form {
                    endpoint: item.data.request.get_complete_authorize_url()?,
                    method: common_utils::request::Method::Get,
                    form_fields: HashMap::from([
                        (
                            "reference".to_string(),
                            item.response.reference.unwrap_or("".to_string()),
                        ),
                        ("signed_on".to_string(), signed_on),
                    ]),
                }),
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankAmount {
    amount: MinorUnit,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankMeansOfPayment {
    bank_account: DeutschebankBankAccount,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankBankAccount {
    account_holder: Secret<String>,
    iban: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankMandate {
    reference: Secret<String>,
    signed_on: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankDirectDebitRequest {
    amount_total: DeutschebankAmount,
    means_of_payment: DeutschebankMeansOfPayment,
    mandate: DeutschebankMandate,
}

impl TryFrom<&DeutschebankRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for DeutschebankDirectDebitRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let account_holder = item.router_data.get_billing_address()?.get_full_name()?;
        let redirect_response = item.router_data.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        let queries_params = redirect_response
            .params
            .map(|param| {
                let mut queries = HashMap::<String, String>::new();
                let values = param.peek().split('&').collect::<Vec<&str>>();
                for value in values {
                    let pair = value.split('=').collect::<Vec<&str>>();
                    queries.insert(
                        pair.first()
                            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
                            .to_string(),
                        pair.get(1)
                            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
                            .to_string(),
                    );
                }
                Ok::<_, errors::ConnectorError>(queries)
            })
            .transpose()?
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        let reference = Secret::from(
            queries_params
                .get("reference")
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "reference",
                })?
                .to_owned(),
        );
        let signed_on = Secret::from(
            queries_params
                .get("signed_on")
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "signed_on",
                })?
                .to_owned(),
        );

        match item.router_data.request.payment_method_data.clone() {
            Some(PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban })) => Ok(Self {
                amount_total: DeutschebankAmount {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                },
                means_of_payment: DeutschebankMeansOfPayment {
                    bank_account: DeutschebankBankAccount {
                        account_holder,
                        iban,
                    },
                },
                mandate: {
                    DeutschebankMandate {
                        reference,
                        signed_on,
                    }
                },
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BankAccount {
    account_holder: Option<Secret<String>>,
    bank_name: Option<Secret<String>>,
    bic: Option<Secret<String>>,
    iban: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TransactionBankAccountInfo {
    bank_account: BankAccount,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct DeutschebankTransactionInfo {
    back_state: Option<String>,
    ip_address: Option<Secret<String>>,
    #[serde(rename = "type")]
    pm_type: Option<String>,
    transaction_bankaccount_info: TransactionBankAccountInfo,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct DeutschebankPaymentsResponse {
    rc: String,
    message: String,
    timestamp: String,
    back_ext_id: Option<String>,
    back_rc: Option<String>,
    event_id: Option<String>,
    kind: Option<String>,
    tx_action: Option<String>,
    tx_id: Option<String>,
    amount_total: Option<DeutschebankAmount>,
    transaction_info: Option<DeutschebankTransactionInfo>,
}

impl
    TryFrom<
        ResponseRouterData<
            CompleteAuthorize,
            DeutschebankPaymentsResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            CompleteAuthorize,
            DeutschebankPaymentsResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let resource_id = ResponseId::ConnectorTransactionId(match item.response.tx_id {
            Some(tx_id) => tx_id,
            None => item
                .response
                .event_id
                .unwrap_or(item.data.connector_request_reference_id.clone()),
        });
        Ok(Self {
            status: if item.response.rc == "0" {
                match item.data.request.is_auto_capture()? {
                    true => common_enums::AttemptStatus::Charged,
                    false => common_enums::AttemptStatus::Authorized,
                }
            } else {
                common_enums::AttemptStatus::Failure
            },
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
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

impl
    TryFrom<
        ResponseRouterData<
            Capture,
            DeutschebankPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Capture,
            DeutschebankPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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

impl
    TryFrom<
        ResponseRouterData<
            PSync,
            DeutschebankPaymentsResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<PSync, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            PSync,
            DeutschebankPaymentsResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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
    pub amount: MinorUnit,
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
