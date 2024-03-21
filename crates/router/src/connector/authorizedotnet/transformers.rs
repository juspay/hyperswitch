use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret, StrongSecret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, CardData, PaymentsSyncRequestData, RefundsRequestData, RouterData, WalletData,
    },
    core::errors,
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums,
        transformers::ForeignFrom,
    },
    utils::OptionExt,
};

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "authCaptureTransaction")]
    Payment,
    #[serde(rename = "authOnlyTransaction")]
    Authorization,
    #[serde(rename = "priorAuthCaptureTransaction")]
    Capture,
    #[serde(rename = "refundTransaction")]
    Refund,
    #[serde(rename = "voidTransaction")]
    Void,
    #[serde(rename = "authOnlyContinueTransaction")]
    ContinueAuthorization,
    #[serde(rename = "authCaptureContinueTransaction")]
    ContinueCapture,
}

#[derive(Debug, Serialize)]
pub struct AuthorizedotnetRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for AuthorizedotnetRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_f64(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetAuthType {
    name: Secret<String>,
    transaction_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for AuthorizedotnetAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                name: api_key.to_owned(),
                transaction_key: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CreditCardDetails {
    card_number: StrongSecret<String, cards::CardNumberStrategy>,
    expiration_date: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_code: Option<Secret<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BankAccountDetails {
    account_number: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum PaymentDetails {
    CreditCard(CreditCardDetails),
    OpaqueData(WalletDetails),
    PayPal(PayPalDetails),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PayPalDetails {
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDetails {
    pub data_descriptor: WalletMethod,
    pub data_value: Secret<String>,
}

#[derive(Serialize, Debug, Deserialize)]
pub enum WalletMethod {
    #[serde(rename = "COMMON.GOOGLE.INAPP.PAYMENT")]
    Googlepay,
    #[serde(rename = "COMMON.APPLE.INAPP.PAYMENT")]
    Applepay,
}

fn get_pm_and_subsequent_auth_detail(
    item: &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<
    (
        PaymentDetails,
        Option<ProcessingOptions>,
        Option<SubsequentAuthInformation>,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match item
        .router_data
        .request
        .mandate_id
        .to_owned()
        .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
    {
        Some(api_models::payments::MandateReferenceId::NetworkMandateId(network_trans_id)) => {
            let processing_options = Some(ProcessingOptions {
                is_subsequent_auth: true,
            });
            let subseuent_auth_info = Some(SubsequentAuthInformation {
                original_network_trans_id: Secret::new(network_trans_id),
                reason: Reason::Resubmission,
            });
            match item.router_data.request.payment_method_data {
                api::PaymentMethodData::Card(ref ccard) => {
                    let payment_details = PaymentDetails::CreditCard(CreditCardDetails {
                        card_number: (*ccard.card_number).clone(),
                        expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                        card_code: None,
                    });
                    Ok((payment_details, processing_options, subseuent_auth_info))
                }
                api::PaymentMethodData::CardRedirect(_)
                | api::PaymentMethodData::Wallet(_)
                | api::PaymentMethodData::PayLater(_)
                | api::PaymentMethodData::BankRedirect(_)
                | api::PaymentMethodData::BankDebit(_)
                | api::PaymentMethodData::BankTransfer(_)
                | api::PaymentMethodData::Crypto(_)
                | api::PaymentMethodData::MandatePayment
                | api::PaymentMethodData::Reward
                | api::PaymentMethodData::Upi(_)
                | api::PaymentMethodData::Voucher(_)
                | api::PaymentMethodData::GiftCard(_)
                | api::PaymentMethodData::CardToken(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                    ))?
                }
            }
        }
        Some(api_models::payments::MandateReferenceId::ConnectorMandateId(_)) | None => {
            match item.router_data.request.payment_method_data {
                api::PaymentMethodData::Card(ref ccard) => {
                    Ok((
                        PaymentDetails::CreditCard(CreditCardDetails {
                            card_number: (*ccard.card_number).clone(),
                            // expiration_date: format!("{expiry_year}-{expiry_month}").into(),
                            expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                            card_code: Some(ccard.card_cvc.clone()),
                        }),
                        Some(ProcessingOptions {
                            is_subsequent_auth: true,
                        }),
                        None,
                    ))
                }
                api::PaymentMethodData::Wallet(ref wallet_data) => Ok((
                    get_wallet_data(
                        wallet_data,
                        &item.router_data.request.complete_authorize_url,
                    )?,
                    None,
                    None,
                )),
                api::PaymentMethodData::CardRedirect(_)
                | api::PaymentMethodData::PayLater(_)
                | api::PaymentMethodData::BankRedirect(_)
                | api::PaymentMethodData::BankDebit(_)
                | api::PaymentMethodData::BankTransfer(_)
                | api::PaymentMethodData::Crypto(_)
                | api::PaymentMethodData::MandatePayment
                | api::PaymentMethodData::Reward
                | api::PaymentMethodData::Upi(_)
                | api::PaymentMethodData::Voucher(_)
                | api::PaymentMethodData::GiftCard(_)
                | api::PaymentMethodData::CardToken(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                    ))?
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    transaction_type: TransactionType,
    amount: f64,
    currency_code: String,
    payment: PaymentDetails,
    order: Order,
    bill_to: Option<BillTo>,
    processing_options: Option<ProcessingOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subsequent_auth_information: Option<SubsequentAuthInformation>,
    authorization_indicator_type: Option<AuthorizationIndicator>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingOptions {
    is_subsequent_auth: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    country: Option<api_enums::CountryAlpha2>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsequentAuthInformation {
    original_network_trans_id: Secret<String>,
    // original_auth_amount: String, Required for Discover, Diners Club, JCB, and China Union Pay transactions.
    reason: Reason,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    Resubmission,
    #[serde(rename = "delayedCharge")]
    DelayedCharge,
    Reauthorization,
    #[serde(rename = "noShow")]
    NoShow,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationIndicator {
    authorization_indicator: AuthorizationType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionVoidOrCaptureRequest {
    transaction_type: TransactionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<f64>,
    ref_trans_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    ref_id: Option<String>,
    transaction_request: TransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentCancelOrCaptureRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: TransactionVoidOrCaptureRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentsRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrCaptureTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationType {
    Final,
    Pre,
}

impl TryFrom<enums::CaptureMethod> for AuthorizationType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(capture_method: enums::CaptureMethod) -> Result<Self, Self::Error> {
        match capture_method {
            enums::CaptureMethod::Manual => Ok(Self::Pre),
            enums::CaptureMethod::Automatic => Ok(Self::Final),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                utils::construct_not_supported_error_report(capture_method, "authorizedotnet"),
            )?,
        }
    }
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>>
    for CreateTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let (payment_details, processing_options, subsequent_auth_information) =
            get_pm_and_subsequent_auth_detail(item)?;
        let authorization_indicator_type = match item.router_data.request.capture_method {
            Some(capture_method) => Some(AuthorizationIndicator {
                authorization_indicator: capture_method.try_into()?,
            }),
            None => None,
        };
        let bill_to = item
            .router_data
            .get_optional_billing()
            .and_then(|billing_address| billing_address.address.as_ref())
            .map(|address| BillTo {
                first_name: address.first_name.clone(),
                last_name: address.last_name.clone(),
                address: address.line1.clone(),
                city: address.city.clone(),
                state: address.state.clone(),
                zip: address.zip.clone(),
                country: address.country,
            });
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency.to_string(),
            payment: payment_details,
            order: Order {
                description: item.router_data.connector_request_reference_id.clone(),
            },
            bill_to,
            processing_options,
            subsequent_auth_information,
            authorization_indicator_type,
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;
        let ref_id = if item.router_data.connector_request_reference_id.len() <= 20 {
            Some(item.router_data.connector_request_reference_id.clone())
        } else {
            None
        };
        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                ref_id,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelOrCaptureTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let transaction_request = TransactionVoidOrCaptureRequest {
            amount: None, //amount is not required for void
            transaction_type: TransactionType::Void,
            ref_trans_id: item.request.connector_transaction_id.to_string(),
        };

        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsCaptureRouterData>>
    for CancelOrCaptureTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction_request = TransactionVoidOrCaptureRequest {
            amount: Some(item.amount),
            transaction_type: TransactionType::Capture,
            ref_trans_id: item
                .router_data
                .request
                .connector_transaction_id
                .to_string(),
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub enum AuthorizedotnetPaymentStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    #[default]
    HeldForReview,
    #[serde(rename = "5")]
    RequiresAction,
}

#[derive(Debug, Clone, serde::Deserialize, Serialize)]
pub enum AuthorizedotnetRefundStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    HeldForReview,
}

impl ForeignFrom<(AuthorizedotnetPaymentStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((item, auto_capture): (AuthorizedotnetPaymentStatus, bool)) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => {
                if auto_capture {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetPaymentStatus::RequiresAction => Self::AuthenticationPending,
            AuthorizedotnetPaymentStatus::HeldForReview => Self::Pending,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
pub struct ResponseMessage {
    code: String,
    pub text: String,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
enum ResultCode {
    #[default]
    Ok,
    Error,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessages {
    result_code: ResultCode,
    pub message: Vec<ResponseMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub error_code: String,
    pub error_text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TransactionResponse {
    AuthorizedotnetTransactionResponse(Box<AuthorizedotnetTransactionResponse>),
    AuthorizedotnetTransactionResponseError(Box<AuthorizedotnetTransactionResponseError>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthorizedotnetTransactionResponseError {
    _supplemental_data_qualification_indicator: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetTransactionResponse {
    response_code: AuthorizedotnetPaymentStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<Secret<String>>,
    pub(super) account_number: Option<Secret<String>>,
    pub(super) errors: Option<Vec<ErrorMessage>>,
    secure_acceptance: Option<SecureAcceptance>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    response_code: AuthorizedotnetRefundStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    #[allow(dead_code)]
    network_trans_id: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SecureAcceptance {
    secure_acceptance_url: Option<url::Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsResponse {
    pub transaction_response: Option<TransactionResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetVoidResponse {
    pub transaction_response: Option<VoidResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    response_code: AuthorizedotnetVoidStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AuthorizedotnetVoidStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    HeldForReview,
}

impl From<AuthorizedotnetVoidStatus> for enums::AttemptStatus {
    fn from(item: AuthorizedotnetVoidStatus) -> Self {
        match item {
            AuthorizedotnetVoidStatus::Approved => Self::Voided,
            AuthorizedotnetVoidStatus::Declined | AuthorizedotnetVoidStatus::Error => {
                Self::VoidFailed
            }
            AuthorizedotnetVoidStatus::HeldForReview => Self::VoidInitiated,
        }
    }
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            AuthorizedotnetPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, is_auto_capture): (
            types::ResponseRouterData<
                F,
                AuthorizedotnetPaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(TransactionResponse::AuthorizedotnetTransactionResponse(transaction_response)) => {
                let status = enums::AttemptStatus::foreign_from((
                    transaction_response.response_code.clone(),
                    is_auto_capture,
                ));
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| types::ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: None,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(acc_no.clone().expose()).encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;
                let url = transaction_response
                    .secure_acceptance
                    .as_ref()
                    .and_then(|x| x.secure_acceptance_url.to_owned());
                let redirection_data =
                    url.map(|url| services::RedirectForm::from((url, services::Method::Get)));
                Ok(Self {
                    status,
                    response: match error {
                        Some(err) => Err(err),
                        None => Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                transaction_response.transaction_id.clone(),
                            ),
                            redirection_data,
                            mandate_reference: None,
                            connector_metadata: metadata,
                            network_txn_id: transaction_response
                                .network_trans_id
                                .clone()
                                .map(|network_trans_id| network_trans_id.expose()),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                            incremental_authorization_allowed: None,
                        }),
                    },
                    ..item.data
                })
            }
            Some(TransactionResponse::AuthorizedotnetTransactionResponseError(_)) | None => {
                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response: Err(get_err_response(item.http_code, item.response.messages)?),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, AuthorizedotnetVoidResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetVoidResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(transaction_response) => {
                let status = enums::AttemptStatus::from(transaction_response.response_code.clone());
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| types::ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: None,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(acc_no.clone().expose()).encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;
                Ok(Self {
                    status,
                    response: match error {
                        Some(err) => Err(err),
                        None => Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                transaction_response.transaction_id.clone(),
                            ),
                            redirection_data: None,
                            mandate_reference: None,
                            connector_metadata: metadata,
                            network_txn_id: transaction_response
                                .network_trans_id
                                .clone()
                                .map(|network_trans_id| network_trans_id.expose()),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                            incremental_authorization_allowed: None,
                        }),
                    },
                    ..item.data
                })
            }
            None => Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundTransactionRequest {
    transaction_type: TransactionType,
    amount: f64,
    currency_code: String,
    payment: PaymentDetails,
    #[serde(rename = "refTransId")]
    reference_transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: RefundTransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateRefundRequest {
    create_transaction_request: AuthorizedotnetRefundRequest,
}

impl<F> TryFrom<&AuthorizedotnetRouterData<&types::RefundsRouterData<F>>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let payment_details = item
            .router_data
            .request
            .connector_metadata
            .as_ref()
            .get_required_value("connector_metadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata",
            })?
            .clone();

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let transaction_request = RefundTransactionRequest {
            transaction_type: TransactionType::Refund,
            amount: item.amount,
            payment: payment_details
                .parse_value("PaymentDetails")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_details",
                })?,
            currency_code: item.router_data.request.currency.to_string(),
            reference_transaction_id: item.router_data.request.connector_transaction_id.clone(),
        };

        Ok(Self {
            create_transaction_request: AuthorizedotnetRefundRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl From<AuthorizedotnetRefundStatus> for enums::RefundStatus {
    fn from(item: AuthorizedotnetRefundStatus) -> Self {
        match item {
            AuthorizedotnetRefundStatus::Approved => Self::Success,
            AuthorizedotnetRefundStatus::Declined | AuthorizedotnetRefundStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetRefundStatus::HeldForReview => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundResponse {
    pub transaction_response: RefundResponse,
    pub messages: ResponseMessages,
}

impl<F> TryFrom<types::RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_response = &item.response.transaction_response;
        let refund_status = enums::RefundStatus::from(transaction_response.response_code.clone());
        let error = transaction_response.errors.clone().and_then(|errors| {
            errors.first().map(|error| types::ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_text.clone(),
                reason: None,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(transaction_response.transaction_id.clone()),
            })
        });

        Ok(Self {
            response: match error {
                Some(err) => Err(err),
                None => Ok(types::RefundsResponseData {
                    connector_refund_id: transaction_response.transaction_id.clone(),
                    refund_status,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    merchant_authentication: AuthorizedotnetAuthType,
    #[serde(rename = "transId")]
    transaction_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetCreateSyncRequest {
    get_transaction_details_request: TransactionDetails,
}

impl<F> TryFrom<&AuthorizedotnetRouterData<&types::RefundsRouterData<F>>>
    for AuthorizedotnetCreateSyncRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AuthorizedotnetRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = item.router_data.request.get_connector_refund_id()?;
        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let payload = Self {
            get_transaction_details_request: TransactionDetails {
                merchant_authentication,
                transaction_id: Some(transaction_id),
            },
        };
        Ok(payload)
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for AuthorizedotnetCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = Some(
            item.request
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        );

        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;

        let payload = Self {
            get_transaction_details_request: TransactionDetails {
                merchant_authentication,
                transaction_id,
            },
        };
        Ok(payload)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatus {
    RefundSettledSuccessfully,
    RefundPendingSettlement,
    AuthorizedPendingCapture,
    CapturedPendingSettlement,
    SettledSuccessfully,
    Declined,
    Voided,
    CouldNotVoid,
    GeneralError,
    #[serde(rename = "FDSPendingReview")]
    FDSPendingReview,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RSyncStatus {
    RefundSettledSuccessfully,
    RefundPendingSettlement,
    Declined,
    GeneralError,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: SyncStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizedotnetSyncResponse {
    transaction: Option<SyncTransactionResponse>,
    messages: ResponseMessages,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RSyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: RSyncStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizedotnetRSyncResponse {
    transaction: Option<RSyncTransactionResponse>,
    messages: ResponseMessages,
}

impl From<SyncStatus> for enums::AttemptStatus {
    fn from(transaction_status: SyncStatus) -> Self {
        match transaction_status {
            SyncStatus::SettledSuccessfully => Self::Charged,
            SyncStatus::CapturedPendingSettlement => Self::CaptureInitiated,
            SyncStatus::AuthorizedPendingCapture => Self::Authorized,
            SyncStatus::Declined => Self::AuthenticationFailed,
            SyncStatus::Voided => Self::Voided,
            SyncStatus::CouldNotVoid => Self::VoidFailed,
            SyncStatus::GeneralError => Self::Failure,
            SyncStatus::RefundSettledSuccessfully
            | SyncStatus::RefundPendingSettlement
            | SyncStatus::FDSPendingReview => Self::Pending,
        }
    }
}

impl From<RSyncStatus> for enums::RefundStatus {
    fn from(transaction_status: RSyncStatus) -> Self {
        match transaction_status {
            RSyncStatus::RefundSettledSuccessfully => Self::Success,
            RSyncStatus::RefundPendingSettlement => Self::Pending,
            RSyncStatus::Declined | RSyncStatus::GeneralError => Self::Failure,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, AuthorizedotnetRSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, AuthorizedotnetRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let refund_status = enums::RefundStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(types::RefundsResponseData {
                        connector_refund_id: transaction.transaction_id,
                        refund_status,
                    }),
                    ..item.data
                })
            }
            None => Ok(Self {
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

impl<F, Req>
    TryFrom<
        types::ResponseRouterData<F, AuthorizedotnetSyncResponse, Req, types::PaymentsResponseData>,
    > for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetSyncResponse,
            Req,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let payment_status = enums::AttemptStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            transaction.transaction_id.clone(),
                        ),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(transaction.transaction_id),
                        incremental_authorization_allowed: None,
                    }),
                    status: payment_status,
                    ..item.data
                })
            }
            None => Ok(Self {
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Default, Debug, Deserialize)]
pub struct AuthorizedotnetErrorResponse {
    pub error: ErrorDetails,
}

fn construct_refund_payment_details(masked_number: String) -> PaymentDetails {
    PaymentDetails::CreditCard(CreditCardDetails {
        card_number: masked_number.into(),
        expiration_date: "XXXX".to_string().into(),
        card_code: None,
    })
}

impl TryFrom<Option<enums::CaptureMethod>> for TransactionType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(capture_method: Option<enums::CaptureMethod>) -> Result<Self, Self::Error> {
        match capture_method {
            Some(enums::CaptureMethod::Manual) => Ok(Self::Authorization),
            Some(enums::CaptureMethod::Automatic) | None => Ok(Self::Payment),
            Some(enums::CaptureMethod::ManualMultiple) => {
                Err(utils::construct_not_supported_error_report(
                    enums::CaptureMethod::ManualMultiple,
                    "authorizedotnet",
                ))?
            }
            Some(enums::CaptureMethod::Scheduled) => {
                Err(utils::construct_not_supported_error_report(
                    enums::CaptureMethod::Scheduled,
                    "authorizedotnet",
                ))?
            }
        }
    }
}

fn get_err_response(
    status_code: u16,
    message: ResponseMessages,
) -> Result<types::ErrorResponse, errors::ConnectorError> {
    let response_message = message
        .message
        .first()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
    Ok(types::ErrorResponse {
        code: response_message.code.clone(),
        message: response_message.text.clone(),
        reason: None,
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetWebhookObjectId {
    pub webhook_id: String,
    pub event_type: AuthorizedotnetWebhookEvent,
    pub payload: AuthorizedotnetWebhookPayload,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizedotnetWebhookPayload {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetWebhookEventType {
    pub event_type: AuthorizedotnetIncomingWebhookEventType,
}

#[derive(Debug, Deserialize)]
pub enum AuthorizedotnetWebhookEvent {
    #[serde(rename = "net.authorize.payment.authorization.created")]
    AuthorizationCreated,
    #[serde(rename = "net.authorize.payment.priorAuthCapture.created")]
    PriorAuthCapture,
    #[serde(rename = "net.authorize.payment.authcapture.created")]
    AuthCapCreated,
    #[serde(rename = "net.authorize.payment.capture.created")]
    CaptureCreated,
    #[serde(rename = "net.authorize.payment.void.created")]
    VoidCreated,
    #[serde(rename = "net.authorize.payment.refund.created")]
    RefundCreated,
}
///Including Unknown to map unknown webhook events
#[derive(Debug, Deserialize)]
pub enum AuthorizedotnetIncomingWebhookEventType {
    #[serde(rename = "net.authorize.payment.authorization.created")]
    AuthorizationCreated,
    #[serde(rename = "net.authorize.payment.priorAuthCapture.created")]
    PriorAuthCapture,
    #[serde(rename = "net.authorize.payment.authcapture.created")]
    AuthCapCreated,
    #[serde(rename = "net.authorize.payment.capture.created")]
    CaptureCreated,
    #[serde(rename = "net.authorize.payment.void.created")]
    VoidCreated,
    #[serde(rename = "net.authorize.payment.refund.created")]
    RefundCreated,
    #[serde(other)]
    Unknown,
}

impl From<AuthorizedotnetIncomingWebhookEventType> for api::IncomingWebhookEvent {
    fn from(event_type: AuthorizedotnetIncomingWebhookEventType) -> Self {
        match event_type {
            AuthorizedotnetIncomingWebhookEventType::AuthorizationCreated
            | AuthorizedotnetIncomingWebhookEventType::PriorAuthCapture
            | AuthorizedotnetIncomingWebhookEventType::AuthCapCreated
            | AuthorizedotnetIncomingWebhookEventType::CaptureCreated
            | AuthorizedotnetIncomingWebhookEventType::VoidCreated => Self::PaymentIntentSuccess,
            AuthorizedotnetIncomingWebhookEventType::RefundCreated => Self::RefundSuccess,
            AuthorizedotnetIncomingWebhookEventType::Unknown => Self::EventNotSupported,
        }
    }
}

impl From<AuthorizedotnetWebhookEvent> for SyncStatus {
    // status mapping reference https://developer.authorize.net/api/reference/features/webhooks.html#Event_Types_and_Payloads
    fn from(event_type: AuthorizedotnetWebhookEvent) -> Self {
        match event_type {
            AuthorizedotnetWebhookEvent::AuthorizationCreated => Self::AuthorizedPendingCapture,
            AuthorizedotnetWebhookEvent::CaptureCreated
            | AuthorizedotnetWebhookEvent::AuthCapCreated => Self::CapturedPendingSettlement,
            AuthorizedotnetWebhookEvent::PriorAuthCapture => Self::SettledSuccessfully,
            AuthorizedotnetWebhookEvent::VoidCreated => Self::Voided,
            AuthorizedotnetWebhookEvent::RefundCreated => Self::RefundSettledSuccessfully,
        }
    }
}

pub fn get_trans_id(
    details: &AuthorizedotnetWebhookObjectId,
) -> Result<String, errors::ConnectorError> {
    details
        .payload
        .id
        .clone()
        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)
}

impl TryFrom<AuthorizedotnetWebhookObjectId> for AuthorizedotnetSyncResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: AuthorizedotnetWebhookObjectId) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: Some(SyncTransactionResponse {
                transaction_id: get_trans_id(&item)?,
                transaction_status: SyncStatus::from(item.event_type),
            }),
            messages: ResponseMessages {
                ..Default::default()
            },
        })
    }
}

fn get_wallet_data(
    wallet_data: &api_models::payments::WalletData,
    return_url: &Option<String>,
) -> CustomResult<PaymentDetails, errors::ConnectorError> {
    match wallet_data {
        api_models::payments::WalletData::GooglePay(_) => {
            Ok(PaymentDetails::OpaqueData(WalletDetails {
                data_descriptor: WalletMethod::Googlepay,
                data_value: Secret::new(wallet_data.get_encoded_wallet_token()?),
            }))
        }
        api_models::payments::WalletData::ApplePay(applepay_token) => {
            Ok(PaymentDetails::OpaqueData(WalletDetails {
                data_descriptor: WalletMethod::Applepay,
                data_value: Secret::new(applepay_token.payment_data.clone()),
            }))
        }
        api_models::payments::WalletData::PaypalRedirect(_) => {
            Ok(PaymentDetails::PayPal(PayPalDetails {
                success_url: return_url.to_owned(),
                cancel_url: return_url.to_owned(),
            }))
        }
        api_models::payments::WalletData::AliPayQr(_)
        | api_models::payments::WalletData::AliPayRedirect(_)
        | api_models::payments::WalletData::AliPayHkRedirect(_)
        | api_models::payments::WalletData::MomoRedirect(_)
        | api_models::payments::WalletData::KakaoPayRedirect(_)
        | api_models::payments::WalletData::GoPayRedirect(_)
        | api_models::payments::WalletData::GcashRedirect(_)
        | api_models::payments::WalletData::ApplePayRedirect(_)
        | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
        | api_models::payments::WalletData::DanaRedirect {}
        | api_models::payments::WalletData::GooglePayRedirect(_)
        | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
        | api_models::payments::WalletData::MbWayRedirect(_)
        | api_models::payments::WalletData::MobilePayRedirect(_)
        | api_models::payments::WalletData::PaypalSdk(_)
        | api_models::payments::WalletData::SamsungPay(_)
        | api_models::payments::WalletData::TwintRedirect {}
        | api_models::payments::WalletData::VippsRedirect {}
        | api_models::payments::WalletData::TouchNGoRedirect(_)
        | api_models::payments::WalletData::WeChatPayRedirect(_)
        | api_models::payments::WalletData::WeChatPayQr(_)
        | api_models::payments::WalletData::CashappQr(_)
        | api_models::payments::WalletData::SwishQr(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
            ))?
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetQueryParams {
    payer_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalConfirmRequest {
    create_transaction_request: PaypalConfirmTransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalConfirmTransactionRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: TransactionConfirmRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionConfirmRequest {
    transaction_type: TransactionType,
    payment: PaypalPaymentConfirm,
    ref_trans_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalPaymentConfirm {
    pay_pal: Paypal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paypal {
    #[serde(rename = "payerID")]
    payer_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalQueryParams {
    #[serde(rename = "PayerID")]
    payer_id: Secret<String>,
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for PaypalConfirmRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let params = item
            .router_data
            .request
            .redirect_response
            .as_ref()
            .and_then(|redirect_response| redirect_response.params.as_ref())
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        let payer_id: Secret<String> =
            serde_urlencoded::from_str::<PaypalQueryParams>(params.peek())
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
                .payer_id;
        let transaction_type = match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual) => Ok(TransactionType::ContinueAuthorization),
            Some(enums::CaptureMethod::Automatic) | None => Ok(TransactionType::ContinueCapture),
            Some(enums::CaptureMethod::ManualMultiple) => {
                Err(errors::ConnectorError::NotSupported {
                    message: enums::CaptureMethod::ManualMultiple.to_string(),
                    connector: "authorizedotnet",
                })
            }
            Some(enums::CaptureMethod::Scheduled) => Err(errors::ConnectorError::NotSupported {
                message: enums::CaptureMethod::Scheduled.to_string(),
                connector: "authorizedotnet",
            }),
        }?;
        let transaction_request = TransactionConfirmRequest {
            transaction_type,
            payment: PaypalPaymentConfirm {
                pay_pal: Paypal { payer_id },
            },
            ref_trans_id: item.router_data.request.connector_transaction_id.clone(),
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: PaypalConfirmTransactionRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}
