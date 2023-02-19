use base64::Engine;
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::{IntoReport, ResultExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, RefundsRequestData, RouterData},
    consts,
    core::errors,
    pii::PeekInterface,
    services,
    types::{
        self, api,
        storage::enums,
        transformers::{Foreign, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum TransactionType {
    #[serde(rename = "authCaptureTransaction")]
    Payment,
    #[serde(rename = "authOnlyTransaction")]
    PaymentAuthOnly,
    #[serde(rename = "priorAuthCaptureTransaction")]
    Capture,
    #[serde(rename = "refundTransaction")]
    Refund,
    #[serde(rename = "voidTransaction")]
    Void,
}
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct MerchantAuthentication {
    name: String,
    transaction_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for MerchantAuthentication {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                name: key1.clone(),
                transaction_key: api_key.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct CreditCardDetails {
    card_number: masking::Secret<String, common_utils::pii::CardNumber>,
    expiration_date: masking::Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_code: Option<masking::Secret<String>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct BankAccountDetails {
    account_number: masking::Secret<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct WalletDetails {
    data_descriptor: String,
    data_value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct PaypalDetails {
    success_url: String,
    cancel_url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum PaymentDetails {
    #[serde(rename = "creditCard")]
    CreditCard(CreditCardDetails),
    #[serde(rename = "bankAccount")]
    BankAccount(BankAccountDetails),
    #[serde(rename = "opaqueData")]
    Wallet(WalletDetails),
    Klarna,
    #[serde(rename = "payPal")]
    Paypal(PaypalDetails),
}

impl TryFrom<(api_models::payments::PaymentMethod, String)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (value, return_url): (api_models::payments::PaymentMethod, String),
    ) -> Result<Self, Self::Error> {
        match value {
            api::PaymentMethod::Card(ref ccard) => {
                let expiry_month = ccard.card_exp_month.peek().clone();
                let expiry_year = ccard.card_exp_year.peek().clone();

                Ok(Self::CreditCard(CreditCardDetails {
                    card_number: ccard.card_number.clone(),
                    expiration_date: format!("{expiry_year}-{expiry_month}").into(),
                    card_code: Some(ccard.card_cvc.clone()),
                }))
            }
            api::PaymentMethod::BankTransfer => Ok(Self::BankAccount(BankAccountDetails {
                account_number: "XXXXX".to_string().into(),
            })),
            api::PaymentMethod::PayLater(_) => Ok(Self::Klarna),
            api::PaymentMethod::Wallet(wallet_data) => match wallet_data.issuer_name {
                api_models::enums::WalletIssuer::GooglePay => Ok(Self::Wallet(WalletDetails {
                    data_descriptor: "COMMON.GOOGLE.INAPP.PAYMENT".to_string(),
                    data_value: consts::BASE64_ENGINE.encode(
                        wallet_data
                            .token
                            .get_required_value("token")
                            .change_context(errors::ConnectorError::RequestEncodingFailed)
                            .attach_printable("No token passed")?,
                    ),
                })),
                api_models::enums::WalletIssuer::ApplePay => Ok(Self::Wallet(WalletDetails {
                    data_descriptor: "COMMON.APPLE.INAPP.PAYMENT".to_string(),
                    data_value: consts::BASE64_ENGINE.encode(
                        wallet_data
                            .token
                            .get_required_value("token")
                            .change_context(errors::ConnectorError::RequestEncodingFailed)
                            .attach_printable("No token passed")?,
                    ),
                })),
                api_models::enums::WalletIssuer::Paypal => Ok(Self::Paypal(PaypalDetails {
                    success_url: return_url.clone(),
                    cancel_url: return_url,
                })),
            },
            api::PaymentMethod::Paypal => Err(errors::ConnectorError::NotImplemented(
                "Unknown Wallet in Payment Method".to_string(),
            ))?,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum TransactionRequest {
    Auth(PaymentRequestData),
    Capture(CaptureRequestData),
    Void(VoidRequestData),
    Refund(RefundRequestData),
    Verify(VerifyRequestData),
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequestData {
    transaction_type: TransactionType,
    amount: i64,
    currency_code: String,
    payment: PaymentDetails,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequestData {
    transaction_type: TransactionType,
    amount: i64,
    ref_trans_id: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AuthorizationIndicator {
    authorization_indicator: AuthorizationType,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoidRequestData {
    transaction_type: TransactionType,
    ref_trans_id: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRequestData {
    transaction_type: TransactionType,
    amount: i64,
    payment: PaymentDetails,
    bill_to: BillDetails,
    processing_options: ProcessingDetails,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct BillDetails {
    address: String,
    zip: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingDetails {
    is_first_subsequent_auth: bool,
    is_stored_credentials: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsRequest {
    merchant_authentication: MerchantAuthentication,
    transaction_request: TransactionRequest,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentsRequest,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
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

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payment_details = PaymentDetails::try_from((
            item.request.payment_method_data.clone(),
            item.router_return_url
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "item.router_return_url",
                })?,
        ))?;
        let transaction_type = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => TransactionType::PaymentAuthOnly,
            _ => TransactionType::Payment,
        };
        let transaction_request = TransactionRequest::Auth(PaymentRequestData {
            transaction_type,
            amount: item.request.amount,
            payment: payment_details,
            currency_code: item.request.currency.to_string(),
        });

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&types::PaymentsCaptureRouterData> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let transaction_type = TransactionType::Capture;
        let transaction_request = TransactionRequest::Capture(CaptureRequestData {
            transaction_type,
            amount: item.request.amount,
            ref_trans_id: item.request.connector_transaction_id.to_string(),
        });

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let transaction_request = TransactionRequest::Void(VoidRequestData {
            transaction_type: TransactionType::Void,
            ref_trans_id: item.request.connector_transaction_id.to_string(),
        });

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&types::VerifyRouterData> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        let payment_details = PaymentDetails::try_from((
            item.request.payment_method_data.clone(),
            item.router_return_url
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "item.router_return_url",
                })?,
        ))?;
        let transaction_type = TransactionType::PaymentAuthOnly;
        let address = item
            .get_billing()?
            .address
            .as_ref()
            .ok_or_else(utils::missing_field_err("billing.address"))?;
        let transaction_request = TransactionRequest::Verify(VerifyRequestData {
            transaction_type,
            amount: 0,
            payment: payment_details,
            bill_to: BillDetails {
                address: address.get_line1()?.to_owned().peek().to_string(),
                zip: address.get_zip()?.to_owned().peek().to_string(),
            },
            processing_options: ProcessingDetails {
                is_first_subsequent_auth: true,
                is_stored_credentials: true,
            },
        });

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
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
    NeedPayerConsent,
}

pub type AuthorizedotnetRefundStatus = AuthorizedotnetPaymentStatus;

impl TryFrom<Foreign<(AuthorizedotnetPaymentStatus, TransactionType)>>
    for Foreign<enums::AttemptStatus>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: Foreign<(AuthorizedotnetPaymentStatus, TransactionType)>,
    ) -> Result<Self, Self::Error> {
        Ok(match item.0 {
            (
                AuthorizedotnetPaymentStatus::Approved,
                TransactionType::Payment | TransactionType::Capture,
            ) => enums::AttemptStatus::Charged,
            (AuthorizedotnetPaymentStatus::Approved, TransactionType::PaymentAuthOnly) => {
                enums::AttemptStatus::Authorized
            }
            (AuthorizedotnetPaymentStatus::Approved, TransactionType::Void) => {
                enums::AttemptStatus::Voided
            }
            (AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error, _) => {
                enums::AttemptStatus::Failure
            }
            (
                AuthorizedotnetPaymentStatus::HeldForReview
                | AuthorizedotnetPaymentStatus::NeedPayerConsent,
                _,
            ) => enums::AttemptStatus::Pending,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed).into_report()?,
        }
        .into())
    }
}

// fn get_payment_status(transaction_type: TransactionType, status: enums::AttemptStatus) -> enums::AttemptStatus {
//     let is_authorized = matches!(status, enums::AttemptStatus::Charged);
//     if is_auth_only && is_authorized {
//         return enums::AttemptStatus::Authorized;
//     }
//     match transaction_type => {
//         TransactionType::Payment => enums::AttemptStatus::Charged,
//         TransactionType::PaymentAuthOnly => enums::AttemptStatus::Authorized,
//         TransactionType::Void => enums::AttemptStatus::Voided,
//     }
//     status
// }

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct ResponseMessage {
    code: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
enum ResultCode {
    Ok,
    Error,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessages {
    result_code: ResultCode,
    message: Vec<ResponseMessage>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct ErrorMessage {
    pub(super) error_code: String,
    pub(super) error_text: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    response_code: AuthorizedotnetPaymentStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    #[serde(rename = "secureAcceptance")]
    redirect_url: Option<SecureURLDetails>,
    pub(super) account_number: Option<String>,
    pub(super) errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsResponse {
    pub transaction_response: TransactionResponse,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct SecureURLDetails {
    secure_acceptance_url: String,
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            AuthorizedotnetPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
        TransactionType,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                AuthorizedotnetPaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            TransactionType,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let status = enums::AttemptStatus::foreign_try_from((
            item.response.transaction_response.response_code,
            data.1,
        ))?;
        let error = item
            .response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.into_iter().next().map(|error| types::ErrorResponse {
                    code: error.error_code,
                    message: error.error_text,
                    reason: None,
                    status_code: item.http_code,
                })
            });

        let metadata = item
            .response
            .transaction_response
            .account_number
            .map(|acc_no| {
                Encode::<'_, PaymentDetails>::encode_to_value(&construct_refund_payment_details(
                    acc_no,
                ))
            })
            .transpose()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata",
            })?;
        let mut redirection_data = None;
        let mut redirect = false;

        if item.response.transaction_response.redirect_url.is_some() {
            let redirection_url_response = Url::parse(
                &item
                    .response
                    .transaction_response
                    .redirect_url
                    .clone()
                    .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
                    .secure_acceptance_url,
            )
            .into_report()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .attach_printable("Failed to parse redirection url")?;

            let form_field_for_redirection = std::collections::HashMap::from_iter(
                redirection_url_response
                    .query_pairs()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            );

            redirection_data = Some(services::RedirectForm {
                url: redirection_url_response.to_string(),
                method: services::Method::Get,
                form_fields: form_field_for_redirection,
            });
            redirect = true;
        }
        Ok(Self {
            status,
            response: match error {
                Some(err) => Err(err),
                None => Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transaction_response.transaction_id,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: metadata,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundRequestData {
    transaction_type: TransactionType,
    amount: i64,
    currency_code: String,
    payment: PaymentDetails,
    #[serde(rename = "refTransId")]
    reference_transaction_id: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let payment_details = item
            .request
            .connector_metadata
            .as_ref()
            .get_required_value("connector_metadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata",
            })?
            .clone();

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        let transaction_request = TransactionRequest::Refund(RefundRequestData {
            transaction_type: TransactionType::Refund,
            amount: item.request.refund_amount,
            payment: payment_details
                .parse_value("PaymentDetails")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_details",
                })?,
            currency_code: item.request.currency.to_string(),
            reference_transaction_id: item.request.connector_transaction_id.clone(),
        });

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl From<AuthorizedotnetPaymentStatus> for enums::RefundStatus {
    fn from(item: AuthorizedotnetRefundStatus) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => Self::Success,
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetPaymentStatus::HeldForReview
            | AuthorizedotnetPaymentStatus::NeedPayerConsent => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundResponse {
    pub transaction_response: TransactionResponse,
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
    merchant_authentication: MerchantAuthentication,
    #[serde(rename = "transId")]
    transaction_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetCreateSyncRequest {
    get_transaction_details_request: TransactionDetails,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for AuthorizedotnetCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let transaction_id = item.request.get_connector_refund_id()?;
        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

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
        let transaction_id = item
            .response
            .as_ref()
            .ok()
            .map(|payment_response_data| match payment_response_data {
                types::PaymentsResponseData::TransactionResponse { resource_id, .. } => {
                    resource_id.get_connector_transaction_id()
                }
                _ => Err(error_stack::report!(
                    errors::ValidationError::MissingRequiredField {
                        field_name: "transaction_id".to_string()
                    }
                )),
            })
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

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
    transaction: SyncTransactionResponse,
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
            SyncStatus::SettledSuccessfully | SyncStatus::CapturedPendingSettlement => {
                Self::Charged
            }
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
        let refund_status = enums::RefundStatus::from(item.response.transaction.transaction_status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction.transaction_id.clone(),
                refund_status,
            }),
            ..item.data
        })
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
        let payment_status =
            enums::AttemptStatus::from(item.response.transaction.transaction_status);
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            status: payment_status,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthorizedotnetWebhookObjectId {
    pub payload: WebhookPaylod,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPaylod {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetWebhookEventType {
    pub event_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct AuthorizedotnetWebhookObjectResource {
    pub event_type: String,
    pub payload: WebhookResourcePayload,
}

#[derive(Debug, Deserialize)]
pub struct WebhookResourcePayload {
    id: String,
}

impl From<AuthorizedotnetWebhookObjectResource> for AuthorizedotnetSyncResponse {
    fn from(value: AuthorizedotnetWebhookObjectResource) -> Self {
        let status = match value.event_type.as_str() {
            // "net.authorize.payment.authorization.created" => self::SyncStatus::AuthorizedPendingCapture,
            "net.authorize.payment.priorAuthCapture.created"
            | "net.authorize.payment.authcapture.created" => self::SyncStatus::SettledSuccessfully,
            _ => self::SyncStatus::GeneralError,
        };
        Self {
            transaction: SyncTransactionResponse {
                transaction_id: value.payload.id,
                transaction_status: status,
            },
        }
    }
}
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Default, Debug, Deserialize, PartialEq, Eq)]
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
