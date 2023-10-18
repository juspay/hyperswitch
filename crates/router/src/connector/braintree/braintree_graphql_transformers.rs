use common_utils::pii;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

pub const CLIENT_TOKEN_MUTATION: &str = "mutation createClientToken($input: CreateClientTokenInput!) { createClientToken(input: $input) { clientToken}}";
pub const TOKENIZE_CREDIT_CARD: &str = "mutation  tokenizeCreditCard($input: TokenizeCreditCardInput!) { tokenizeCreditCard(input: $input) { clientMutationId paymentMethod { id } } }";
pub const CHARGE_CREDIT_CARD_MUTATION: &str = "mutation ChargeCreditCard($input: ChargeCreditCardInput!) { chargeCreditCard(input: $input) { transaction { id legacyId createdAt amount { value currencyCode } status } } }";
pub const AUTHORIZE_CREDIT_CARD_MUTATION: &str = "mutation authorizeCreditCard($input: AuthorizeCreditCardInput!) { authorizeCreditCard(input: $input) {  transaction { id legacyId amount { value currencyCode } status } } }";
pub const CAPTURE_TRANSACTION_MUTATION: &str = "mutation captureTransaction($input: CaptureTransactionInput!) { captureTransaction(input: $input) { clientMutationId transaction { id legacyId amount { value currencyCode } status } } }";
pub const VOID_TRANSACTION_MUTATION: &str = "mutation voidTransaction($input:  ReverseTransactionInput!) { reverseTransaction(input: $input) { clientMutationId reversal { ...  on Transaction { id legacyId amount { value currencyCode } status } } } }";
pub const REFUND_TRANSACTION_MUTATION: &str = "mutation refundTransaction($input:  RefundTransactionInput!) { refundTransaction(input: $input) {clientMutationId refund { id legacyId amount { value currencyCode } status } } }";

#[derive(Debug, Serialize)]
pub struct BraintreeRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for BraintreeRouterData<T>
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
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    payment_method_id: String,
    transaction: TransactionBody,
}

#[derive(Debug, Serialize)]
pub struct VariablePaymentInput {
    input: PaymentInput,
}

#[derive(Debug, Serialize)]
pub struct CardPaymentRequest {
    query: String,
    variables: VariablePaymentInput,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BraintreePaymentsRequest {
    Card(CardPaymentRequest),
    CardThreeDs(BraintreeClientTokenRequest),
}

#[derive(Debug, Deserialize)]
pub struct BraintreeMeta {
    merchant_account_id: Secret<String>,
    merchant_config_currency: types::storage::enums::Currency,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for BraintreeMeta {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionBody {
    amount: String,
    merchant_account_id: Secret<String>,
}

impl TryFrom<&BraintreeRouterData<&types::PaymentsAuthorizeRouterData>>
    for BraintreePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;
        utils::validate_currency(
            item.router_data.request.currency,
            Some(metadata.merchant_config_currency),
        )?;

        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                if item.router_data.is_three_ds() {
                    Ok(Self::CardThreeDs(BraintreeClientTokenRequest::try_from(
                        metadata,
                    )?))
                } else {
                    Ok(Self::Card(CardPaymentRequest::try_from((item, metadata))?))
                }
            }
            api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("braintree"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&BraintreeRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for BraintreePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BraintreeRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.payment_method {
            api_models::enums::PaymentMethod::Card => {
                Ok(Self::Card(CardPaymentRequest::try_from(item)?))
            }
            api_models::enums::PaymentMethod::CardRedirect
            | api_models::enums::PaymentMethod::PayLater
            | api_models::enums::PaymentMethod::Wallet
            | api_models::enums::PaymentMethod::BankRedirect
            | api_models::enums::PaymentMethod::BankTransfer
            | api_models::enums::PaymentMethod::Crypto
            | api_models::enums::PaymentMethod::BankDebit
            | api_models::enums::PaymentMethod::Reward
            | api_models::enums::PaymentMethod::Upi
            | api_models::enums::PaymentMethod::Voucher
            | api_models::enums::PaymentMethod::GiftCard => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message(
                        "complete authorize flow",
                    ),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    data: DataAuthResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeAuthResponse {
    AuthResponse(Box<AuthResponse>),
    ClientTokenResponse(Box<ClientTokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeCompleteAuthResponse {
    AuthResponse(Box<AuthResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionAuthChargeResponseBody {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAuthResponse {
    authorize_credit_card: AuthChargeCreditCard,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthChargeCreditCard {
    transaction: TransactionAuthChargeResponseBody,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            BraintreeAuthResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreeAuthResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeAuthResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code),
                ..item.data
            }),
            BraintreeAuthResponse::AuthResponse(auth_response) => {
                let transaction_data = auth_response.data.authorize_credit_card.transaction;

                Ok(Self {
                    status: enums::AttemptStatus::from(transaction_data.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
            BraintreeAuthResponse::ClientTokenResponse(client_token_data) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Some(get_braintree_redirect_form(
                        *client_token_data,
                        item.data.get_payment_method_token()?,
                        item.data.request.payment_method_data.clone(),
                    )?),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                ..item.data
            }),
        }
    }
}

fn build_error_response<T>(
    response: &[ErrorDetails],
    http_code: u16,
) -> Result<T, types::ErrorResponse> {
    let error_messages = response
        .iter()
        .map(|error| error.message.to_string())
        .collect::<Vec<String>>();

    let reason = match !error_messages.is_empty() {
        true => Some(error_messages.join(" ")),
        false => None,
    };

    get_error_response(
        response
            .get(0)
            .and_then(|err_details| err_details.extensions.as_ref())
            .and_then(|extensions| extensions.legacy_code.clone()),
        response
            .get(0)
            .map(|err_details| err_details.message.clone()),
        reason,
        http_code,
    )
}

fn get_error_response<T>(
    error_code: Option<String>,
    error_msg: Option<String>,
    error_reason: Option<String>,
    http_code: u16,
) -> Result<T, types::ErrorResponse> {
    Err(types::ErrorResponse {
        code: error_code.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
        message: error_msg.unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
        reason: error_reason,
        status_code: http_code,
    })
}

// Using Auth type from braintree/transformer.rs, need this in later time when we use graphql version
// pub struct BraintreeAuthType {
//     pub(super) auth_header: String,
//     pub(super) merchant_id: Secret<String>,
// }

// impl TryFrom<&types::ConnectorAuthType> for BraintreeAuthType {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
//         if let types::ConnectorAuthType::SignatureKey {
//             api_key: public_key,
//             key1: merchant_id,
//             api_secret: private_key,
//         } = item
//         {
//             let auth_key = format!("{}:{}", public_key.peek(), private_key.peek());
//             let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
//             Ok(Self {
//                 auth_header,
//                 merchant_id: merchant_id.to_owned(),
//             })
//         } else {
//             Err(errors::ConnectorError::FailedToObtainAuthType)?
//         }
//     }
// }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BraintreePaymentStatus {
    Authorized,
    Authorizing,
    AuthorizedExpired,
    Failed,
    ProcessorDeclined,
    GatewayRejected,
    Voided,
    Settling,
    Settled,
    SettlementPending,
    SettlementDeclined,
    SettlementConfirmed,
    SubmittedForSettlement,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorDetails {
    pub message: String,
    pub extensions: Option<AdditionalErrorDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalErrorDetails {
    pub legacy_code: Option<String>,
}

impl From<BraintreePaymentStatus> for enums::AttemptStatus {
    fn from(item: BraintreePaymentStatus) -> Self {
        match item {
            BraintreePaymentStatus::Settling
            | BraintreePaymentStatus::Settled
            | BraintreePaymentStatus::SettlementConfirmed => Self::Charged,
            BraintreePaymentStatus::Authorizing => Self::Authorizing,
            BraintreePaymentStatus::AuthorizedExpired => Self::AuthorizationFailed,
            BraintreePaymentStatus::Failed
            | BraintreePaymentStatus::GatewayRejected
            | BraintreePaymentStatus::ProcessorDeclined
            | BraintreePaymentStatus::SettlementDeclined => Self::Failure,
            BraintreePaymentStatus::Authorized => Self::Authorized,
            BraintreePaymentStatus::Voided => Self::Voided,
            BraintreePaymentStatus::SubmittedForSettlement
            | BraintreePaymentStatus::SettlementPending => Self::Pending,
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            BraintreePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreePaymentsResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors.clone(), item.http_code),
                ..item.data
            }),
            BraintreePaymentsResponse::PaymentsResponse(payment_response) => {
                let transaction_data = payment_response.data.charge_credit_card.transaction;

                Ok(Self {
                    status: enums::AttemptStatus::from(transaction_data.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
            BraintreePaymentsResponse::ClientTokenResponse(client_token_data) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Some(get_braintree_redirect_form(
                        *client_token_data,
                        item.data.get_payment_method_token()?,
                        item.data.request.payment_method_data.clone(),
                    )?),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                }),
                ..item.data
            }),
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            BraintreeCompleteChargeResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreeCompleteChargeResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCompleteChargeResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors.clone(), item.http_code),
                ..item.data
            }),
            BraintreeCompleteChargeResponse::PaymentsResponse(payment_response) => {
                let transaction_data = payment_response.data.charge_credit_card.transaction;

                Ok(Self {
                    status: enums::AttemptStatus::from(transaction_data.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            BraintreeCompleteAuthResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BraintreeCompleteAuthResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCompleteAuthResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code),
                ..item.data
            }),
            BraintreeCompleteAuthResponse::AuthResponse(auth_response) => {
                let transaction_data = auth_response.data.authorize_credit_card.transaction;

                Ok(Self {
                    status: enums::AttemptStatus::from(transaction_data.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentsResponse {
    data: DataResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreePaymentsResponse {
    PaymentsResponse(Box<PaymentsResponse>),
    ClientTokenResponse(Box<ClientTokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeCompleteChargeResponse {
    PaymentsResponse(Box<PaymentsResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataResponse {
    charge_credit_card: AuthChargeCreditCard,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundInputData {
    amount: String,
    merchant_account_id: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeRefundInput {
    transaction_id: String,
    refund: RefundInputData,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeRefundVariables {
    input: BraintreeRefundInput,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeRefundRequest {
    query: String,
    variables: BraintreeRefundVariables,
}

impl<F> TryFrom<BraintreeRouterData<&types::RefundsRouterData<F>>> for BraintreeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: BraintreeRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;

        utils::validate_currency(
            item.router_data.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        let query = REFUND_TRANSACTION_MUTATION.to_string();
        let variables = BraintreeRefundVariables {
            input: BraintreeRefundInput {
                transaction_id: item.router_data.request.connector_transaction_id.clone(),
                refund: RefundInputData {
                    amount: item.amount,
                    merchant_account_id: metadata.merchant_account_id,
                },
            },
        };
        Ok(Self { query, variables })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BraintreeRefundStatus {
    SettlementPending,
    Settling,
    Settled,
    SubmittedForSettlement,
    Failed,
}

impl From<BraintreeRefundStatus> for enums::RefundStatus {
    fn from(item: BraintreeRefundStatus) -> Self {
        match item {
            BraintreeRefundStatus::Settled | BraintreeRefundStatus::Settling => Self::Success,
            BraintreeRefundStatus::SubmittedForSettlement
            | BraintreeRefundStatus::SettlementPending => Self::Pending,
            BraintreeRefundStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BraintreeRefundTransactionBody {
    pub id: String,
    pub status: BraintreeRefundStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BraintreeRefundTransaction {
    pub refund: BraintreeRefundTransactionBody,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeRefundResponseData {
    pub refund_transaction: BraintreeRefundTransaction,
}

#[derive(Debug, Clone, Deserialize)]

pub struct RefundResponse {
    pub data: BraintreeRefundResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeRefundResponse {
    RefundResponse(Box<RefundResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, BraintreeRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, BraintreeRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response {
                BraintreeRefundResponse::ErrorResponse(error_response) => {
                    build_error_response(&error_response.errors, item.http_code)
                }
                BraintreeRefundResponse::RefundResponse(refund_data) => {
                    let refund_data = refund_data.data.refund_transaction.refund;

                    Ok(types::RefundsResponseData {
                        connector_refund_id: refund_data.id.clone(),
                        refund_status: enums::RefundStatus::from(refund_data.status),
                    })
                }
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct BraintreeRSyncRequest {
    query: String,
}

impl TryFrom<&types::RefundSyncRouterData> for BraintreeRSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta = utils::to_connector_meta_from_secret(
            item.connector_meta_data.clone(),
        )
        .change_context(errors::ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
        utils::validate_currency(
            item.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        let refund_id = item.request.get_connector_refund_id()?;
        let query = format!("query {{ search {{ refunds(input: {{ id: {{is: \"{}\"}} }}, first: 1) {{ edges {{ node {{ id status createdAt amount {{ value currencyCode }} orderId }} }} }} }} }}",refund_id);

        Ok(Self { query })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RSyncNodeData {
    id: String,
    status: BraintreeRefundStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RSyncEdgeData {
    node: RSyncNodeData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefundData {
    edges: Vec<RSyncEdgeData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RSyncSearchData {
    refunds: RefundData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RSyncResponseData {
    search: RSyncSearchData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RSyncResponse {
    data: RSyncResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeRSyncResponse {
    RSyncResponse(Box<RSyncResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, BraintreeRSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, BraintreeRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeRSyncResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code),
                ..item.data
            }),
            BraintreeRSyncResponse::RSyncResponse(rsync_response) => {
                let edge_data = rsync_response
                    .data
                    .search
                    .refunds
                    .edges
                    .first()
                    .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
                let connector_refund_id = &edge_data.node.id;
                let response = Ok(types::RefundsResponseData {
                    connector_refund_id: connector_refund_id.to_string(),
                    refund_status: enums::RefundStatus::from(edge_data.node.status.clone()),
                });
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditCardData {
    number: cards::CardNumber,
    expiration_year: Secret<String>,
    expiration_month: Secret<String>,
    cvv: Secret<String>,
    cardholder_name: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTokenInput {
    merchant_account_id: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputData {
    credit_card: CreditCardData,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputClientTokenData {
    client_token: ClientTokenInput,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct VariableInput {
    input: InputData,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct VariableClientTokenInput {
    input: InputClientTokenData,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeTokenRequest {
    query: String,
    variables: VariableInput,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeClientTokenRequest {
    query: String,
    variables: VariableClientTokenInput,
}

impl TryFrom<&types::TokenizationRouterData> for BraintreeTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card_data) => {
                let query = TOKENIZE_CREDIT_CARD.to_string();
                let input = InputData {
                    credit_card: CreditCardData {
                        number: card_data.card_number,
                        expiration_year: card_data.card_exp_year,
                        expiration_month: card_data.card_exp_month,
                        cvv: card_data.card_cvc,
                        cardholder_name: card_data.card_holder_name,
                    },
                };
                Ok(Self {
                    query,
                    variables: VariableInput { input },
                })
            }
            api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("braintree"),
                )
                .into())
            }
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct TokenizePaymentMethodData {
    id: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizeCreditCardData {
    payment_method: TokenizePaymentMethodData,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientToken {
    client_token: Secret<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizeCreditCard {
    tokenize_credit_card: TokenizeCreditCardData,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTokenData {
    create_client_token: ClientToken,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct ClientTokenResponse {
    data: ClientTokenData,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct TokenResponse {
    data: TokenizeCreditCard,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    errors: Vec<ErrorDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeTokenResponse {
    TokenResponse(Box<TokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BraintreeTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BraintreeTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response {
                BraintreeTokenResponse::ErrorResponse(error_response) => {
                    build_error_response(error_response.errors.as_ref(), item.http_code)
                }

                BraintreeTokenResponse::TokenResponse(token_response) => {
                    Ok(types::PaymentsResponseData::TokenizationResponse {
                        token: token_response
                            .data
                            .tokenize_credit_card
                            .payment_method
                            .id
                            .clone(),
                    })
                }
            },
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureTransactionBody {
    amount: String,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInputData {
    transaction_id: String,
    transaction: CaptureTransactionBody,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct VariableCaptureInput {
    input: CaptureInputData,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct BraintreeCaptureRequest {
    query: String,
    variables: VariableCaptureInput,
}

impl TryFrom<&BraintreeRouterData<&types::PaymentsCaptureRouterData>> for BraintreeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BraintreeRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let query = CAPTURE_TRANSACTION_MUTATION.to_string();
        let variables = VariableCaptureInput {
            input: CaptureInputData {
                transaction_id: item.router_data.request.connector_transaction_id.clone(),
                transaction: CaptureTransactionBody {
                    amount: item.amount.to_owned(),
                },
            },
        };
        Ok(Self { query, variables })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureResponseTransactionBody {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureTransactionData {
    transaction: CaptureResponseTransactionBody,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponseData {
    capture_transaction: CaptureTransactionData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureResponse {
    data: CaptureResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeCaptureResponse {
    CaptureResponse(Box<CaptureResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<BraintreeCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<BraintreeCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCaptureResponse::CaptureResponse(capture_data) => {
                let transaction_data = capture_data.data.capture_transaction.transaction;

                Ok(Self {
                    status: enums::AttemptStatus::from(transaction_data.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
            BraintreeCaptureResponse::ErrorResponse(error_data) => Ok(Self {
                response: build_error_response(&error_data.errors, item.http_code),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelInputData {
    transaction_id: String,
}

#[derive(Debug, Serialize)]
pub struct VariableCancelInput {
    input: CancelInputData,
}

#[derive(Debug, Serialize)]
pub struct BraintreeCancelRequest {
    query: String,
    variables: VariableCancelInput,
}

impl TryFrom<&types::PaymentsCancelRouterData> for BraintreeCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let query = VOID_TRANSACTION_MUTATION.to_string();
        let variables = VariableCancelInput {
            input: CancelInputData {
                transaction_id: item.request.connector_transaction_id.clone(),
            },
        };
        Ok(Self { query, variables })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelResponseTransactionBody {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelTransactionData {
    reversal: CancelResponseTransactionBody,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponseData {
    reverse_transaction: CancelTransactionData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelResponse {
    data: CancelResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreeCancelResponse {
    CancelResponse(Box<CancelResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BraintreeCancelResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BraintreeCancelResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCancelResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code),
                ..item.data
            }),
            BraintreeCancelResponse::CancelResponse(void_response) => {
                let void_data = void_response.data.reverse_transaction.reversal;

                let transaction_id = void_data.id;
                Ok(Self {
                    status: enums::AttemptStatus::from(void_data.status),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(transaction_id),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BraintreePSyncRequest {
    query: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for BraintreePSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let query = format!("query {{ search {{ transactions(input: {{ id: {{is: \"{}\"}} }}, first: 1) {{ edges {{ node {{ id status createdAt amount {{ value currencyCode }} orderId }} }} }} }} }}", transaction_id);
        Ok(Self { query })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeData {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EdgeData {
    node: NodeData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionData {
    edges: Vec<EdgeData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchData {
    transactions: TransactionData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PSyncResponseData {
    search: SearchData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BraintreePSyncResponse {
    PSyncResponse(Box<PSyncResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PSyncResponse {
    data: PSyncResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BraintreePSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BraintreePSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreePSyncResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code),
                ..item.data
            }),
            BraintreePSyncResponse::PSyncResponse(psync_response) => {
                let edge_data = psync_response
                    .data
                    .search
                    .transactions
                    .edges
                    .first()
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;
                let transaction_id = &edge_data.node.id;
                Ok(Self {
                    status: enums::AttemptStatus::from(edge_data.node.status.clone()),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            transaction_id.to_string(),
                        ),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeThreeDsResponse {
    pub nonce: String,
    pub liability_shifted: bool,
    pub liability_shift_possible: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeThreeDsErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BraintreeRedirectionResponse {
    pub authentication_response: String,
}

impl TryFrom<BraintreeMeta> for BraintreeClientTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(metadata: BraintreeMeta) -> Result<Self, Self::Error> {
        Ok(Self {
            query: CLIENT_TOKEN_MUTATION.to_owned(),
            variables: VariableClientTokenInput {
                input: InputClientTokenData {
                    client_token: ClientTokenInput {
                        merchant_account_id: metadata.merchant_account_id,
                    },
                },
            },
        })
    }
}

impl
    TryFrom<(
        &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
        BraintreeMeta,
    )> for CardPaymentRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, metadata): (
            &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
            BraintreeMeta,
        ),
    ) -> Result<Self, Self::Error> {
        let query = match item.router_data.request.is_auto_capture()? {
            true => CHARGE_CREDIT_CARD_MUTATION.to_string(),
            false => AUTHORIZE_CREDIT_CARD_MUTATION.to_string(),
        };
        Ok(Self {
            query,
            variables: VariablePaymentInput {
                input: PaymentInput {
                    payment_method_id: match item.router_data.get_payment_method_token()? {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    },
                    transaction: TransactionBody {
                        amount: item.amount.to_owned(),
                        merchant_account_id: metadata.merchant_account_id,
                    },
                },
            },
        })
    }
}

impl TryFrom<&BraintreeRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for CardPaymentRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BraintreeRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;
        utils::validate_currency(
            item.router_data.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        let payload_data =
            utils::PaymentsCompleteAuthorizeRequestData::get_redirect_response_payload(
                &item.router_data.request,
            )?
            .expose();
        let redirection_response: BraintreeRedirectionResponse =
            serde_json::from_value(payload_data)
                .into_report()
                .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
                    field_name: "redirection_response",
                })?;
        let three_ds_data = serde_json::from_str::<BraintreeThreeDsResponse>(
            &redirection_response.authentication_response,
        )
        .into_report()
        .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
            field_name: "three_ds_data",
        })?;
        let query = match utils::PaymentsCompleteAuthorizeRequestData::is_auto_capture(
            &item.router_data.request,
        )? {
            true => CHARGE_CREDIT_CARD_MUTATION.to_string(),
            false => AUTHORIZE_CREDIT_CARD_MUTATION.to_string(),
        };
        Ok(Self {
            query,
            variables: VariablePaymentInput {
                input: PaymentInput {
                    payment_method_id: three_ds_data.nonce,
                    transaction: TransactionBody {
                        amount: item.amount.to_owned(),
                        merchant_account_id: metadata.merchant_account_id,
                    },
                },
            },
        })
    }
}

fn get_braintree_redirect_form(
    client_token_data: ClientTokenResponse,
    payment_method_token: types::PaymentMethodToken,
    card_details: api_models::payments::PaymentMethodData,
) -> Result<services::RedirectForm, error_stack::Report<errors::ConnectorError>> {
    Ok(services::RedirectForm::Braintree {
        client_token: client_token_data
            .data
            .create_client_token
            .client_token
            .expose(),
        card_token: match payment_method_token {
            types::PaymentMethodToken::Token(token) => token,
            types::PaymentMethodToken::ApplePayDecrypt(_) => {
                Err(errors::ConnectorError::InvalidWalletToken)?
            }
        },
        bin: match card_details {
            api_models::payments::PaymentMethodData::Card(card_details) => {
                card_details.card_number.get_card_isin()
            }
            api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => Err(
                errors::ConnectorError::NotImplemented("given payment method".to_owned()),
            )?,
        },
    })
}

#[derive(Debug, Deserialize)]
pub struct BraintreeWebhookResponse {
    pub bt_signature: String,
    pub bt_payload: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Notification {
    pub kind: String, // xml parse only string to fields
    pub timestamp: String,
    pub dispute: Option<BraintreeDisputeData>,
}
impl types::transformers::ForeignFrom<&str> for api_models::webhooks::IncomingWebhookEvent {
    fn foreign_from(status: &str) -> Self {
        match status {
            "dispute_opened" => Self::DisputeOpened,
            "dispute_lost" => Self::DisputeLost,
            "dispute_won" => Self::DisputeWon,
            "dispute_accepted" | "dispute_auto_accepted" => Self::DisputeAccepted,
            "dispute_expired" => Self::DisputeExpired,
            "dispute_disputed" => Self::DisputeChallenged,
            _ => Self::EventNotSupported,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BraintreeDisputeData {
    pub amount_disputed: i64,
    pub amount_won: Option<String>,
    pub case_number: Option<String>,
    pub chargeback_protection_level: Option<String>,
    pub currency_iso_code: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    pub evidence: Option<DisputeEvidence>,
    pub id: String,
    pub kind: String, // xml parse only string to fields
    pub status: String,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub reply_by_date: Option<PrimitiveDateTime>,
    pub transaction: DisputeTransaction,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeTransaction {
    pub amount: String,
    pub id: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeEvidence {
    pub comment: String,
    pub id: Secret<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub url: url::Url,
}

pub(crate) fn get_dispute_stage(code: &str) -> Result<enums::DisputeStage, errors::ConnectorError> {
    match code {
        "CHARGEBACK" => Ok(enums::DisputeStage::Dispute),
        "PRE_ARBITATION" => Ok(enums::DisputeStage::PreArbitration),
        "RETRIEVAL" => Ok(enums::DisputeStage::PreDispute),
        _ => Err(errors::ConnectorError::WebhookBodyDecodingFailed),
    }
}
