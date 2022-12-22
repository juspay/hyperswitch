use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum TransactionType {
    #[serde(rename = "authCaptureTransaction")]
    Payment,
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
                name: api_key.clone(),
                transaction_key: key1.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CreditCardDetails {
    card_number: String,
    expiration_date: String,
    card_code: String,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BankAccountDetails {
    account_number: String,
}

#[derive(Serialize, PartialEq)]
enum PaymentDetails {
    #[serde(rename = "creditCard")]
    CreditCard(CreditCardDetails),
    #[serde(rename = "bankAccount")]
    BankAccount(BankAccountDetails),
    Wallet,
    Klarna,
    Paypal,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    transaction_type: TransactionType,
    amount: i64,
    currency_code: String,
    payment: PaymentDetails,
    authorization_indicator_type: Option<AuthorizationIndicator>,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AuthorizationIndicator {
    authorization_indicator: AuthorizationType,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TransactionVoidRequest {
    transaction_type: TransactionType,
    ref_trans_id: String,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsRequest {
    merchant_authentication: MerchantAuthentication,
    transaction_request: TransactionRequest,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentCancelRequest {
    merchant_authentication: MerchantAuthentication,
    transaction_request: TransactionVoidRequest,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentsRequest,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CancelTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentCancelRequest,
}

#[derive(Serialize, PartialEq, Eq)]
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
        let payment_details = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let expiry_month = ccard.card_exp_month.peek().clone();
                let expiry_year = ccard.card_exp_year.peek().clone();

                PaymentDetails::CreditCard(CreditCardDetails {
                    card_number: ccard.card_number.peek().clone(),
                    expiration_date: format!("{expiry_year}-{expiry_month}"),
                    card_code: ccard.card_cvc.peek().clone(),
                })
            }
            api::PaymentMethod::BankTransfer => PaymentDetails::BankAccount(BankAccountDetails {
                account_number: "XXXXX".to_string(),
            }),
            api::PaymentMethod::PayLater(_) => PaymentDetails::Klarna,
            api::PaymentMethod::Wallet(_) => PaymentDetails::Wallet,
            api::PaymentMethod::Paypal => PaymentDetails::Paypal,
        };
        let authorization_indicator_type =
            item.request.capture_method.map(|c| AuthorizationIndicator {
                authorization_indicator: c.into(),
            });
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Payment,
            amount: item.request.amount,
            payment: payment_details,
            currency_code: item.request.currency.to_string(),
            authorization_indicator_type,
        };

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let transaction_request = TransactionVoidRequest {
            transaction_type: TransactionType::Void,
            ref_trans_id: item.request.connector_transaction_id.to_string(),
        };

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentCancelRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

// Safety: Enum as u8 conversions, we are specifying discriminants which are well within the range
// of u8
#[allow(clippy::as_conversions)]
mod status {
    #[derive(
        Debug,
        Clone,
        Default,
        PartialEq,
        Eq,
        serde_repr::Serialize_repr,
        serde_repr::Deserialize_repr,
    )]
    #[repr(u8)]
    pub enum AuthorizedotnetPaymentStatus {
        Approved = 1,
        Declined = 2,
        Error = 3,
        #[default]
        HeldForReview = 4,
    }
}
pub use status::AuthorizedotnetPaymentStatus;
pub type AuthorizedotnetRefundStatus = AuthorizedotnetPaymentStatus;

impl From<AuthorizedotnetPaymentStatus> for enums::AttemptStatus {
    fn from(item: AuthorizedotnetPaymentStatus) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => Self::Charged,
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetPaymentStatus::HeldForReview => Self::Pending,
        }
    }
}

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
    auth_code: String,
    #[serde(rename = "transId")]
    transaction_id: String,
    pub(super) errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsResponse {
    pub transaction_response: TransactionResponse,
    pub messages: ResponseMessages,
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
        let status = enums::AttemptStatus::from(item.response.transaction_response.response_code);
        let error = item
            .response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.into_iter().next().map(|error| types::ErrorResponse {
                    code: error.error_code,
                    message: error.error_text,
                    reason: None,
                })
            });

        Ok(Self {
            status,
            response: match error {
                Some(err) => Err(err),
                None => Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transaction_response.transaction_id,
                    ),
                    redirection_data: None,
                    redirect: false,
                    mandate_reference: None,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundTransactionRequest {
    transaction_type: TransactionType,
    amount: i64,
    currency_code: String,
    payment: PaymentDetails,
    #[serde(rename = "refTransId")]
    reference_transaction_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundRequest {
    merchant_authentication: MerchantAuthentication,
    transaction_request: RefundTransactionRequest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateRefundRequest {
    create_transaction_request: AuthorizedotnetRefundRequest,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let (payment_details, merchant_authentication, transaction_request);
        payment_details = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let expiry_month = ccard.card_exp_month.peek().clone();
                let expiry_year = ccard.card_exp_year.peek().clone();

                PaymentDetails::CreditCard(CreditCardDetails {
                    card_number: ccard.card_number.peek().clone(),
                    expiration_date: format!("{expiry_year}-{expiry_month}"),
                    card_code: ccard.card_cvc.peek().clone(),
                })
            }
            api::PaymentMethod::BankTransfer => PaymentDetails::BankAccount(BankAccountDetails {
                account_number: "XXXXX".to_string(),
            }),
            api::PaymentMethod::PayLater(_) => PaymentDetails::Klarna,
            api::PaymentMethod::Wallet(_) => PaymentDetails::Wallet,
            api::PaymentMethod::Paypal => PaymentDetails::Paypal,
        };

        merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        transaction_request = RefundTransactionRequest {
            transaction_type: TransactionType::Refund,
            amount: item.request.refund_amount,
            payment: payment_details,
            currency_code: item.request.currency.to_string(),
            reference_transaction_id: item.request.connector_transaction_id.clone(),
        };

        Ok(Self {
            create_transaction_request: AuthorizedotnetRefundRequest {
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
            AuthorizedotnetPaymentStatus::HeldForReview => Self::Pending,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    merchant_authentication: MerchantAuthentication,
    #[serde(rename = "transId")]
    transaction_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetCreateSyncRequest {
    get_transaction_details_request: TransactionDetails,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for AuthorizedotnetCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let transaction_id = item
            .response
            .as_ref()
            .map(|refund_response_data| refund_response_data.connector_refund_id.clone())
            .ok();
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

#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: SyncStatus,
}

#[derive(Debug, Deserialize)]
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
                redirect: false,
                mandate_reference: None,
            }),
            status: payment_status,
            ..item.data
        })
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
