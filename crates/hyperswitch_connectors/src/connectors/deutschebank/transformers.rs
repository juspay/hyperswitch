use std::collections::HashMap;

use cards::CardNumber;
use common_enums::{enums, PaymentMethod};
use common_utils::{ext_traits::ValueExt, pii::Email, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsCancelResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, CardData, PaymentsAuthorizeRequestData,
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
    CreditCard(Box<DeutschebankThreeDSInitializeRequest>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequest {
    means_of_payment: DeutschebankThreeDSInitializeRequestMeansOfPayment,
    tds_20_data: DeutschebankThreeDSInitializeRequestTds20Data,
    amount_total: DeutschebankThreeDSInitializeRequestAmountTotal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestMeansOfPayment {
    credit_card: DeutschebankThreeDSInitializeRequestCreditCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestCreditCard {
    number: CardNumber,
    expiry_date: DeutschebankThreeDSInitializeRequestCreditCardExpiry,
    code: Secret<String>,
    cardholder: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestCreditCardExpiry {
    year: Secret<String>,
    month: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestAmountTotal {
    amount: MinorUnit,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestTds20Data {
    communication_data: DeutschebankThreeDSInitializeRequestCommunicationData,
    customer_data: DeutschebankThreeDSInitializeRequestCustomerData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestCommunicationData {
    method_notification_url: String,
    cres_notification_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestCustomerData {
    billing_address: DeutschebankThreeDSInitializeRequestCustomerBillingData,
    cardholder_email: Email,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeutschebankThreeDSInitializeRequestCustomerBillingData {
    street: Secret<String>,
    postal_code: Secret<String>,
    city: String,
    state: Secret<String>,
    country: String,
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
                match item.router_data.request.payment_method_data.clone() {
                    PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban, .. }) => {
                        if item.router_data.request.customer_acceptance.is_some() {
                            let billing_address = item.router_data.get_billing_address()?;
                            Ok(Self::MandatePost(DeutschebankMandatePostRequest {
                                approval_by: DeutschebankSEPAApproval::Click,
                                email_address: item.router_data.request.get_email()?,
                                iban: Secret::from(iban.peek().replace(" ", "")),
                                first_name: billing_address.get_first_name()?.clone(),
                                last_name: billing_address.get_last_name()?.clone(),
                            }))
                        } else {
                            Err(errors::ConnectorError::MissingRequiredField {
                                field_name: "customer_acceptance",
                            }
                            .into())
                        }
                    }
                    PaymentMethodData::Card(ccard) => {
                        if !item.router_data.clone().is_three_ds() {
                            Err(errors::ConnectorError::NotSupported {
                                message: "Non-ThreeDs".to_owned(),
                                connector: "deutschebank",
                            }
                            .into())
                        } else {
                            let billing_address = item.router_data.get_billing_address()?;
                            Ok(Self::CreditCard(Box::new(DeutschebankThreeDSInitializeRequest {
                                    means_of_payment: DeutschebankThreeDSInitializeRequestMeansOfPayment {
                                        credit_card: DeutschebankThreeDSInitializeRequestCreditCard {
                                            number: ccard.clone().card_number,
                                            expiry_date: DeutschebankThreeDSInitializeRequestCreditCardExpiry {
                                                year: ccard.get_expiry_year_4_digit(),
                                                month: ccard.card_exp_month,
                                            },
                                            code: ccard.card_cvc,
                                            cardholder: item.router_data.get_billing_full_name()?,
                                        }},
                                    amount_total: DeutschebankThreeDSInitializeRequestAmountTotal {
                                        amount: item.amount,
                                        currency: item.router_data.request.currency,
                                    },
                                    tds_20_data: DeutschebankThreeDSInitializeRequestTds20Data {
                                        communication_data: DeutschebankThreeDSInitializeRequestCommunicationData {
                                            method_notification_url: item.router_data.request.get_complete_authorize_url()?,
                                            cres_notification_url: item.router_data.request.get_complete_authorize_url()?,
                                        },
                                        customer_data: DeutschebankThreeDSInitializeRequestCustomerData {
                                            billing_address: DeutschebankThreeDSInitializeRequestCustomerBillingData {
                                                street: billing_address.get_line1()?.clone(),
                                                postal_code: billing_address.get_zip()?.clone(),
                                                city: billing_address.get_city()?.to_string(),
                                                state: billing_address.get_state()?.clone(),
                                                country: item.router_data.get_billing_country()?.to_string(),
                                            },
                                            cardholder_email: item.router_data.request.get_email()?,
                                        }
                                    }
                                })))
                        }
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("deutschebank"),
                    )
                    .into()),
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
pub struct DeutschebankThreeDSInitializeResponse {
    outcome: DeutschebankThreeDSInitializeResponseOutcome,
    challenge_required: Option<DeutschebankThreeDSInitializeResponseChallengeRequired>,
    processed: Option<DeutschebankThreeDSInitializeResponseProcessed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankThreeDSInitializeResponseProcessed {
    rc: String,
    message: String,
    tx_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeutschebankThreeDSInitializeResponseOutcome {
    Processed,
    ChallengeRequired,
    MethodRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeutschebankThreeDSInitializeResponseChallengeRequired {
    acs_url: String,
    creq: String,
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            DeutschebankThreeDSInitializeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            DeutschebankThreeDSInitializeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.outcome {
            DeutschebankThreeDSInitializeResponseOutcome::Processed => {
                match item.response.processed {
                    Some(processed) => Ok(Self {
                        status: if is_response_success(&processed.rc) {
                            match item.data.request.is_auto_capture()? {
                                true => common_enums::AttemptStatus::Charged,
                                false => common_enums::AttemptStatus::Authorized,
                            }
                        } else {
                            common_enums::AttemptStatus::AuthenticationFailed
                        },
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                processed.tx_id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: Some(processed.tx_id.clone()),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    }),
                    None => {
                        let response_string = format!("{:?}", item.response);
                        Err(
                            errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                                response_string,
                            ))
                            .into(),
                        )
                    }
                }
            }
            DeutschebankThreeDSInitializeResponseOutcome::ChallengeRequired => {
                match item.response.challenge_required {
                    Some(challenge) => Ok(Self {
                        status: common_enums::AttemptStatus::AuthenticationPending,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::NoResponseId,
                            redirection_data: Box::new(Some(
                                RedirectForm::DeutschebankThreeDSChallengeFlow {
                                    acs_url: challenge.acs_url,
                                    creq: challenge.creq,
                                },
                            )),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    }),
                    None => {
                        let response_string = format!("{:?}", item.response);
                        Err(
                            errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                                response_string,
                            ))
                            .into(),
                        )
                    }
                }
            }
            DeutschebankThreeDSInitializeResponseOutcome::MethodRequired => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_owned(),
                    message: "METHOD_REQUIRED Flow not supported for deutschebank 3ds payments".to_owned(),
                    reason: Some("METHOD_REQUIRED Flow is not currently supported for deutschebank 3ds payments".to_owned()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
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

fn get_error_response(error_code: String, error_reason: String, status_code: u16) -> ErrorResponse {
    ErrorResponse {
        code: error_code.to_string(),
        message: error_reason.clone(),
        reason: Some(error_reason),
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    }
}

fn is_response_success(rc: &String) -> bool {
    rc == "0"
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
        let response_code = item.response.rc.clone();
        let is_response_success = is_response_success(&response_code);

        match (
            item.response.reference.clone(),
            item.response.state.clone(),
            is_response_success,
        ) {
            (Some(reference), Some(state), true) => Ok(Self {
                status: common_enums::AttemptStatus::from(state),
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
                    charges: None,
                }),
                ..item.data
            }),
            _ => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
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
        let response_code = item.response.rc.clone();
        if is_response_success(&response_code) {
            Ok(Self {
                status: match item.data.request.is_auto_capture()? {
                    true => common_enums::AttemptStatus::Charged,
                    false => common_enums::AttemptStatus::Authorized,
                },
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            })
        }
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

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DeutschebankCompleteAuthorizeRequest {
    DeutschebankDirectDebitRequest(DeutschebankDirectDebitRequest),
    DeutschebankThreeDSCompleteAuthorizeRequest(DeutschebankThreeDSCompleteAuthorizeRequest),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DeutschebankThreeDSCompleteAuthorizeRequest {
    cres: String,
}

impl TryFrom<&DeutschebankRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for DeutschebankCompleteAuthorizeRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DeutschebankRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if matches!(item.router_data.payment_method, PaymentMethod::Card) {
            let redirect_response_payload = item
                .router_data
                .request
                .get_redirect_response_payload()?
                .expose();

            let cres = redirect_response_payload
                .get("cres")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "cres" })?;

            Ok(Self::DeutschebankThreeDSCompleteAuthorizeRequest(
                DeutschebankThreeDSCompleteAuthorizeRequest { cres },
            ))
        } else {
            match item.router_data.request.payment_method_data.clone() {
                Some(PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit {
                    iban, ..
                })) => {
                    let account_holder = item.router_data.get_billing_address()?.get_full_name()?;
                    let redirect_response =
                        item.router_data.request.redirect_response.clone().ok_or(
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
                                        .ok_or(
                                            errors::ConnectorError::ResponseDeserializationFailed,
                                        )?
                                        .to_string(),
                                    pair.get(1)
                                        .ok_or(
                                            errors::ConnectorError::ResponseDeserializationFailed,
                                        )?
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
                    Ok(Self::DeutschebankDirectDebitRequest(
                        DeutschebankDirectDebitRequest {
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
                        },
                    ))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("deutschebank"),
                )
                .into()),
            }
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
        let response_code = item.response.rc.clone();
        if is_response_success(&response_code) {
            Ok(Self {
                status: match item.data.request.is_auto_capture()? {
                    true => common_enums::AttemptStatus::Charged,
                    false => common_enums::AttemptStatus::Authorized,
                },
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeutschebankTransactionKind {
    Directdebit,
    #[serde(rename = "CREDITCARD_3DS20")]
    Creditcard3ds20,
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
        if matches!(item.router_data.payment_method, PaymentMethod::BankDebit) {
            Ok(Self {
                changed_amount: item.amount,
                kind: DeutschebankTransactionKind::Directdebit,
            })
        } else if item.router_data.is_three_ds()
            && matches!(item.router_data.payment_method, PaymentMethod::Card)
        {
            Ok(Self {
                changed_amount: item.amount,
                kind: DeutschebankTransactionKind::Creditcard3ds20,
            })
        } else {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("deutschebank"),
            )
            .into())
        }
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
        let response_code = item.response.rc.clone();
        if is_response_success(&response_code) {
            Ok(Self {
                status: common_enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.tx_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            })
        }
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
        let response_code = item.response.rc.clone();
        let status = if is_response_success(&response_code) {
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
            Some(common_enums::AttemptStatus::Failure) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            }),
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
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        if matches!(item.payment_method, PaymentMethod::BankDebit) {
            Ok(Self {
                kind: DeutschebankTransactionKind::Directdebit,
            })
        } else if item.is_three_ds() && matches!(item.payment_method, PaymentMethod::Card) {
            Ok(Self {
                kind: DeutschebankTransactionKind::Creditcard3ds20,
            })
        } else {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("deutschebank"),
            )
            .into())
        }
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<DeutschebankPaymentsResponse>>
    for PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response_code = item.response.rc.clone();
        if is_response_success(&response_code) {
            Ok(Self {
                status: common_enums::AttemptStatus::Voided,
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::VoidFailed,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            })
        }
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
        if matches!(item.router_data.payment_method, PaymentMethod::BankDebit) {
            Ok(Self {
                changed_amount: item.amount,
                kind: DeutschebankTransactionKind::Directdebit,
            })
        } else if item.router_data.is_three_ds()
            && matches!(item.router_data.payment_method, PaymentMethod::Card)
        {
            Ok(Self {
                changed_amount: item.amount,
                kind: DeutschebankTransactionKind::Creditcard3ds20,
            })
        } else {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("deutschebank"),
            )
            .into())
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, DeutschebankPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response_code = item.response.rc.clone();
        if is_response_success(&response_code) {
            Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.response.tx_id,
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            })
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, DeutschebankPaymentsResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, DeutschebankPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response_code = item.response.rc.clone();
        let status = if is_response_success(&response_code) {
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
            Some(enums::RefundStatus::Failure) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(get_error_response(
                    response_code.clone(),
                    item.response.message.clone(),
                    item.http_code,
                )),
                ..item.data
            }),
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PaymentsErrorResponse {
    pub rc: String,
    pub message: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AccessTokenErrorResponse {
    pub cause: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DeutschebankError {
    PaymentsErrorResponse(PaymentsErrorResponse),
    AccessTokenErrorResponse(AccessTokenErrorResponse),
}
