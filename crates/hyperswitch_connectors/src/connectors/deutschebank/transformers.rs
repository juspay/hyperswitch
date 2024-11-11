use std::collections::HashMap;

use common_enums::enums;
use common_utils::{ext_traits::ValueExt, pii::Email, types::MinorUnit};
use error_stack::ResultExt;
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
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsCancelResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RefundsRequestData, RouterData as OtherRouterData,
    },
};

pub struct DeutschebankRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for DeutschebankRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
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
pub struct DeutschebankMandatePostRequest {
    approval_by: DeutschebankSEPAApproval,
    email_address: Email,
    iban: Secret<String>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DeutschebankPaymentsRequest {
    MandatePost(DeutschebankMandatePostRequest),
    DirectDebit(DeutschebankDirectDebitRequest),
}

impl TryFrom<&DeutschebankRouterData<&PaymentsAuthorizeRouterData>>
    for DeutschebankPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item
            .router_data
            .request
            .mandate_id
            .clone()
            .and_then(|mandate_id| mandate_id.mandate_reference_id)
        {
            None => {
                // To facilitate one-off payments via SEPA with Deutsche Bank, we are considering not storing the connector mandate ID in our system if future usage is on-session.
                // We will only check for customer acceptance to make a one-off payment. we will be storing the connector mandate details only when setup future usage is off-session.
                if item.router_data.request.customer_acceptance.is_some() {
                    match item.router_data.request.payment_method_data.clone() {
                        PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit {
                            iban, ..
                        }) => {
                            let billing_address = item.router_data.get_billing_address()?;
                            Ok(Self::MandatePost(DeutschebankMandatePostRequest {
                                approval_by: DeutschebankSEPAApproval::Click,
                                email_address: item.router_data.request.get_email()?,
                                iban: Secret::from(iban.peek().replace(" ", "")),
                                first_name: billing_address.get_first_name()?.clone(),
                                last_name: billing_address.get_last_name()?.clone(),
                            }))
                        }
                        _ => Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("deutschebank"),
                        )
                        .into()),
                    }
                } else {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "customer_acceptance",
                    }
                    .into())
                }
            }
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(mandate_data)) => {
                let mandate_metadata: DeutschebankMandateMetadata = mandate_data
                    .get_mandate_metadata()
                    .ok_or(errors::ConnectorError::MissingConnectorMandateMetadata)?
                    .clone()
                    .parse_value("DeutschebankMandateMetadata")
                    .change_context(errors::ConnectorError::ParsingFailed)?;
                Ok(Self::DirectDebit(DeutschebankDirectDebitRequest {
                    amount_total: DeutschebankAmount {
                        amount: item.amount,
                        currency: item.router_data.request.currency,
                    },
                    means_of_payment: DeutschebankMeansOfPayment {
                        bank_account: DeutschebankBankAccount {
                            account_holder: mandate_metadata.account_holder,
                            iban: mandate_metadata.iban,
                        },
                    },
                    mandate: DeutschebankMandate {
                        reference: mandate_metadata.reference,
                        signed_on: mandate_metadata.signed_on,
                    },
                }))
            }
            Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_))
            | Some(api_models::payments::MandateReferenceId::NetworkMandateId(_)) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("deutschebank"),
                )
                .into())
            }
        }
    }
}

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
pub struct DeutschebankMandateMetadata {
    account_holder: Secret<String>,
    iban: Secret<String>,
    reference: Secret<String>,
    signed_on: String,
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
        let signed_on = match item.response.approval_date.clone() {
            Some(date) => date.chars().take(10).collect(),
            None => time::OffsetDateTime::now_utc().date().to_string(),
        };
        match item.response.reference.clone() {
            Some(reference) => Ok(Self {
                status: if item.response.rc == "0" {
                    match item.response.state.clone() {
                        Some(state) => common_enums::AttemptStatus::from(state),
                        None => common_enums::AttemptStatus::Failure,
                    }
                } else {
                    common_enums::AttemptStatus::Failure
                },
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(Some(RedirectForm::Form {
                        endpoint: item.data.request.get_complete_authorize_url()?,
                        method: common_utils::request::Method::Get,
                        form_fields: HashMap::from([
                            ("reference".to_string(), reference.clone()),
                            ("signed_on".to_string(), signed_on.clone()),
                        ]),
                    })),
                    mandate_reference: if item.data.request.is_mandate_payment() {
                        Box::new(Some(MandateReference {
                            connector_mandate_id: item.response.mandate_id,
                            payment_method_id: None,
                            mandate_metadata: Some(Secret::new(
                                serde_json::json!(DeutschebankMandateMetadata {
                                account_holder: item.data.get_billing_address()?.get_full_name()?,
                                iban: match item.data.request.payment_method_data.clone() {
                                    PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit {
                                        iban,
                                        ..
                                    }) => Ok(Secret::from(iban.peek().replace(" ", ""))),
                                    _ => Err(errors::ConnectorError::MissingRequiredField {
                                        field_name:
                                            "payment_method_data.bank_debit.sepa_bank_debit.iban"
                                    }),
                                }?,
                                reference: Secret::from(reference.clone()),
                                signed_on,
                            }),
                            )),
                            connector_mandate_request_reference_id: None,
                        }))
                    } else {
                        Box::new(None)
                    },
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            None => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                ..item.data
            }),
        }
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            DeutschebankPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            DeutschebankPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
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
                resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
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
    signed_on: String,
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
        let signed_on = queries_params
            .get("signed_on")
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "signed_on",
            })?
            .to_owned();

        match item.router_data.request.payment_method_data.clone() {
            Some(PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban, .. })) => {
                Ok(Self {
                    amount_total: DeutschebankAmount {
                        amount: item.amount,
                        currency: item.router_data.request.currency,
                    },
                    means_of_payment: DeutschebankMeansOfPayment {
                        bank_account: DeutschebankBankAccount {
                            account_holder,
                            iban: Secret::from(iban.peek().replace(" ", "")),
                        },
                    },
                    mandate: {
                        DeutschebankMandate {
                            reference,
                            signed_on,
                        }
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("deutschebank"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeutschebankTXAction {
    Authorization,
    Capture,
    Credit,
    Preauthorization,
    Refund,
    Reversal,
    RiskCheck,
    #[serde(rename = "verify-mop")]
    VerifyMop,
    Payment,
    AccountInformation,
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
    bank_account: Option<BankAccount>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct DeutschebankTransactionInfo {
    back_state: Option<String>,
    ip_address: Option<Secret<String>>,
    #[serde(rename = "type")]
    pm_type: Option<String>,
    transaction_bankaccount_info: Option<TransactionBankAccountInfo>,
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
    tx_action: Option<DeutschebankTXAction>,
    tx_id: String,
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
                resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
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

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeutschebankTransactionKind {
    Directdebit,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankCaptureRequest {
    changed_amount: MinorUnit,
    kind: DeutschebankTransactionKind,
}

impl TryFrom<&DeutschebankRouterData<&PaymentsCaptureRouterData>> for DeutschebankCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            changed_amount: item.amount,
            kind: DeutschebankTransactionKind::Directdebit,
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
                resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            status: if item.response.rc == "0" {
                common_enums::AttemptStatus::Charged
            } else {
                common_enums::AttemptStatus::Failure
            },
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
        let status = if item.response.rc == "0" {
            item.response
                .tx_action
                .and_then(|tx_action| match tx_action {
                    DeutschebankTXAction::Preauthorization => {
                        Some(common_enums::AttemptStatus::Authorized)
                    }
                    DeutschebankTXAction::Authorization | DeutschebankTXAction::Capture => {
                        Some(common_enums::AttemptStatus::Charged)
                    }
                    DeutschebankTXAction::Credit
                    | DeutschebankTXAction::Refund
                    | DeutschebankTXAction::Reversal
                    | DeutschebankTXAction::RiskCheck
                    | DeutschebankTXAction::VerifyMop
                    | DeutschebankTXAction::Payment
                    | DeutschebankTXAction::AccountInformation => None,
                })
        } else {
            Some(common_enums::AttemptStatus::Failure)
        };
        match status {
            Some(status) => Ok(Self {
                status,
                ..item.data
            }),
            None => Ok(Self { ..item.data }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DeutschebankReversalRequest {
    kind: DeutschebankTransactionKind,
}

impl TryFrom<&PaymentsCancelRouterData> for DeutschebankReversalRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: DeutschebankTransactionKind::Directdebit,
        })
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<DeutschebankPaymentsResponse>>
    for PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: if item.response.rc == "0" {
                common_enums::AttemptStatus::Voided
            } else {
                common_enums::AttemptStatus::VoidFailed
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DeutschebankRefundRequest {
    changed_amount: MinorUnit,
    kind: DeutschebankTransactionKind,
}

impl<F> TryFrom<&DeutschebankRouterData<&RefundsRouterData<F>>> for DeutschebankRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DeutschebankRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            changed_amount: item.amount.to_owned(),
            kind: DeutschebankTransactionKind::Directdebit,
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, DeutschebankPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.tx_id,
                refund_status: if item.response.rc == "0" {
                    enums::RefundStatus::Success
                } else {
                    enums::RefundStatus::Failure
                },
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, DeutschebankPaymentsResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status = if item.response.rc == "0" {
            item.response
                .tx_action
                .and_then(|tx_action| match tx_action {
                    DeutschebankTXAction::Credit | DeutschebankTXAction::Refund => {
                        Some(enums::RefundStatus::Success)
                    }
                    DeutschebankTXAction::Preauthorization
                    | DeutschebankTXAction::Authorization
                    | DeutschebankTXAction::Capture
                    | DeutschebankTXAction::Reversal
                    | DeutschebankTXAction::RiskCheck
                    | DeutschebankTXAction::VerifyMop
                    | DeutschebankTXAction::Payment
                    | DeutschebankTXAction::AccountInformation => None,
                })
        } else {
            Some(enums::RefundStatus::Failure)
        };
        match status {
            Some(refund_status) => Ok(Self {
                response: Ok(RefundsResponseData {
                    refund_status,
                    connector_refund_id: item.data.request.get_connector_refund_id()?,
                }),
                ..item.data
            }),
            None => Ok(Self { ..item.data }),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankErrorResponse {
    pub rc: String,
    pub message: String,
}
