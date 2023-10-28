use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret, StrongSecret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsSyncRequestData, RefundsRequestData, WalletData},
    core::errors,
    services,
    types::{self, api, storage::enums},
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
    pub data_value: String,
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
        Some(api_models::payments::MandateReferenceId::NetworkMandateId(
            original_network_trans_id,
        )) => {
            let processing_options = Some(ProcessingOptions {
                is_subsequent_auth: true,
            });
            let subseuent_auth_info = Some(SubsequentAuthInformation {
                original_network_trans_id,
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
                _ => Err(errors::ConnectorError::NotSupported {
                    message: format!("{:?}", item.router_data.request.payment_method_data),
                    connector: "AuthorizeDotNet",
                })?,
            }
        }
        _ => match item.router_data.request.payment_method_data {
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
            _ => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.router_data.request.payment_method_data),
                connector: "AuthorizeDotNet",
            })?,
        },
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    transaction_type: TransactionType,
    amount: f64,
    currency_code: String,
    payment: PaymentDetails,
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
pub struct SubsequentAuthInformation {
    original_network_trans_id: String,
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
    transaction_request: TransactionRequest,
    ref_id: String,
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

impl From<enums::CaptureMethod> for AuthorizationType {
    fn from(item: enums::CaptureMethod) -> Self {
        match item {
            enums::CaptureMethod::Manual => Self::Pre,
            _ => Self::Final,
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
        let authorization_indicator_type =
            item.router_data
                .request
                .capture_method
                .map(|c| AuthorizationIndicator {
                    authorization_indicator: c.into(),
                });
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::from(item.router_data.request.capture_method),
            amount: item.amount,
            payment: payment_details,
            currency_code: item.router_data.request.currency.to_string(),
            processing_options,
            subsequent_auth_information,
            authorization_indicator_type,
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
                ref_id: item.router_data.connector_request_reference_id.clone(),
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

#[derive(Debug, Clone, Default, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Deserialize)]
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

impl From<AuthorizedotnetPaymentStatus> for enums::AttemptStatus {
    fn from(item: AuthorizedotnetPaymentStatus) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => Self::Pending,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub error_code: String,
    pub error_text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TransactionResponse {
    AuthorizedotnetTransactionResponse(Box<AuthorizedotnetTransactionResponse>),
    AuthorizedotnetTransactionResponseError(Box<AuthorizedotnetTransactionResponseError>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthorizedotnetTransactionResponseError {
    _supplemental_data_qualification_indicator: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetTransactionResponse {
    response_code: AuthorizedotnetPaymentStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<String>,
    pub(super) account_number: Option<String>,
    pub(super) errors: Option<Vec<ErrorMessage>>,
    secure_acceptance: Option<SecureAcceptance>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    response_code: AuthorizedotnetRefundStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    #[allow(dead_code)]
    network_trans_id: Option<String>,
    pub account_number: Option<String>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SecureAcceptance {
    secure_acceptance_url: Option<url::Url>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsResponse {
    pub transaction_response: Option<TransactionResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetVoidResponse {
    pub transaction_response: Option<VoidResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    response_code: AuthorizedotnetVoidStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<String>,
    pub account_number: Option<String>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize)]
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
            AuthorizedotnetVoidStatus::Approved => Self::VoidInitiated,
            AuthorizedotnetVoidStatus::Declined | AuthorizedotnetVoidStatus::Error => Self::Failure,
            AuthorizedotnetVoidStatus::HeldForReview => Self::Pending,
        }
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            AuthorizedotnetPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(TransactionResponse::AuthorizedotnetTransactionResponse(transaction_response)) => {
                let status = enums::AttemptStatus::from(transaction_response.response_code.clone());
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| types::ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: None,
                        status_code: item.http_code,
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        Encode::<'_, PaymentDetails>::encode_to_value(
                            &construct_refund_payment_details(acc_no.clone()),
                        )
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
                            network_txn_id: transaction_response.network_trans_id.clone(),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                        }),
                    },
                    ..item.data
                })
            }
            Some(TransactionResponse::AuthorizedotnetTransactionResponseError(_)) | None => {
                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response: Err(get_err_response(item.http_code, item.response.messages)),
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
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        Encode::<'_, PaymentDetails>::encode_to_value(
                            &construct_refund_payment_details(acc_no.clone()),
                        )
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
                            network_txn_id: transaction_response.network_trans_id.clone(),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                        }),
                    },
                    ..item.data
                })
            }
            None => Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(get_err_response(item.http_code, item.response.messages)),
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

#[derive(Debug, Deserialize)]
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

impl From<SyncStatus> for enums::RefundStatus {
    fn from(transaction_status: SyncStatus) -> Self {
        match transaction_status {
            SyncStatus::RefundSettledSuccessfully => Self::Success,
            SyncStatus::RefundPendingSettlement => Self::Pending,
            _ => Self::Failure,
        }
    }
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
            _ => Self::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, AuthorizedotnetSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, AuthorizedotnetSyncResponse>,
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
                response: Err(get_err_response(item.http_code, item.response.messages)),
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
                        connector_response_reference_id: Some(transaction.transaction_id.clone()),
                    }),
                    status: payment_status,
                    ..item.data
                })
            }
            None => Ok(Self {
                response: Err(get_err_response(item.http_code, item.response.messages)),
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

impl From<Option<enums::CaptureMethod>> for TransactionType {
    fn from(capture_method: Option<enums::CaptureMethod>) -> Self {
        match capture_method {
            Some(enums::CaptureMethod::Manual) => Self::Authorization,
            _ => Self::Payment,
        }
    }
}

fn get_err_response(status_code: u16, message: ResponseMessages) -> types::ErrorResponse {
    types::ErrorResponse {
        code: message.message[0].code.clone(),
        message: message.message[0].text.clone(),
        reason: None,
        status_code,
    }
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
                data_value: wallet_data.get_encoded_wallet_token()?,
            }))
        }
        api_models::payments::WalletData::ApplePay(applepay_token) => {
            Ok(PaymentDetails::OpaqueData(WalletDetails {
                data_descriptor: WalletMethod::Applepay,
                data_value: applepay_token.payment_data.clone(),
            }))
        }
        api_models::payments::WalletData::PaypalRedirect(_) => {
            Ok(PaymentDetails::PayPal(PayPalDetails {
                success_url: return_url.to_owned(),
                cancel_url: return_url.to_owned(),
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment method".to_string(),
        ))?,
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
    payer_id: String,
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
        let payer_id: Secret<String> = Secret::new(
            serde_urlencoded::from_str::<PaypalQueryParams>(params.peek())
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
                .payer_id,
        );
        let transaction_type = match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual) => TransactionType::ContinueAuthorization,
            _ => TransactionType::ContinueCapture,
        };
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
