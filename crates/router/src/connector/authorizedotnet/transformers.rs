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
            Ok(MerchantAuthentication {
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
    Klarna,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    transaction_type: TransactionType,
    amount: i32,
    currency_code: String,
    payment: PaymentDetails,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsRequest {
    merchant_authentication: MerchantAuthentication,
    transaction_request: TransactionRequest,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentsRequest,
}

impl TryFrom<&types::PaymentsRouterData> for CreateTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsRouterData) -> Result<Self, Self::Error> {
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
        };

        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Payment,
            amount: item.amount,
            payment: payment_details,
            currency_code: item.currency.to_string(),
        };

        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        Ok(CreateTransactionRequest {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorizedotnetPaymentStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[default]
    #[serde(rename = "4")]
    HeldForReview,
}

pub type AuthorizedotnetRefundStatus = AuthorizedotnetPaymentStatus;

impl From<AuthorizedotnetPaymentStatus> for enums::AttemptStatus {
    fn from(item: AuthorizedotnetPaymentStatus) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => enums::AttemptStatus::Charged,
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                enums::AttemptStatus::Failure
            }
            AuthorizedotnetPaymentStatus::HeldForReview => enums::AttemptStatus::Pending,
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

impl TryFrom<types::PaymentsResponseRouterData<AuthorizedotnetPaymentsResponse>>
    for types::PaymentsRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<AuthorizedotnetPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.transaction_response.response_code);
        let error = item
            .response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            });

        Ok(types::RouterData {
            status,
            response: Some(types::PaymentsResponseData {
                connector_transaction_id: item.response.transaction_response.transaction_id,
                //TODO: Add redirection details here
                redirection_data: None,
                redirect: false,
            }),
            error_response: error,
            ..item.data
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundTransactionRequest {
    transaction_type: TransactionType,
    amount: i32,
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
        };

        merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        transaction_request = RefundTransactionRequest {
            transaction_type: TransactionType::Refund,
            amount: item.request.refund_amount,
            payment: payment_details,
            currency_code: item.currency.to_string(),
            reference_transaction_id: item.request.connector_transaction_id.clone(),
        };

        Ok(CreateRefundRequest {
            create_transaction_request: AuthorizedotnetRefundRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl From<self::AuthorizedotnetPaymentStatus> for enums::RefundStatus {
    fn from(item: self::AuthorizedotnetRefundStatus) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => enums::RefundStatus::Success,
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                enums::RefundStatus::Failure
            }
            AuthorizedotnetPaymentStatus::HeldForReview => enums::RefundStatus::Pending,
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

        Ok(types::RouterData {
            response: Some(types::RefundsResponseData {
                connector_refund_id: transaction_response.transaction_id.clone(),
                refund_status,
            }),
            error_response: error,
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
pub struct CreateSyncRequest {
    get_transaction_details_request: TransactionDetails,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let transaction_id = item
            .response
            .as_ref()
            .map(|refund_response_data| refund_response_data.connector_refund_id.clone());
        let merchant_authentication = MerchantAuthentication::try_from(&item.connector_auth_type)?;

        let payload = CreateSyncRequest {
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
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: SyncStatus,
}

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    transaction: SyncTransactionResponse,
}

impl From<SyncStatus> for enums::RefundStatus {
    fn from(transaction_status: SyncStatus) -> Self {
        match transaction_status {
            SyncStatus::RefundSettledSuccessfully => enums::RefundStatus::Success,
            SyncStatus::RefundPendingSettlement => enums::RefundStatus::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, SyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, SyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.transaction.transaction_status);
        Ok(types::RouterData {
            response: Some(types::RefundsResponseData {
                connector_refund_id: item.response.transaction.transaction_id.clone(),
                refund_status,
            }),
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
