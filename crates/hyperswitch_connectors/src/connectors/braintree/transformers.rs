use api_models::{
    payments as payment_types,
    payments::{ApplePaySessionResponse, SessionToken},
    webhooks::IncomingWebhookEvent,
};
use common_enums::enums;
use common_utils::{
    ext_traits::{OptionExt, ValueExt},
    pii,
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{CompleteAuthorizeData, MandateRevokeRequestData, ResponseId},
    router_response_types::{
        MandateReference, MandateRevokeResponseData, PaymentsResponseData, RedirectForm,
        RefundsResponseData,
    },
    types::{self, RefundsRouterData},
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;
use time::PrimitiveDateTime;

use crate::{
    types::{
        PaymentsCaptureResponseRouterData, PaymentsResponseRouterData,
        PaymentsSessionResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    unimplemented_payment_method,
    utils::{
        self, ForeignTryFrom, PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        RefundsRequestData, RouterData as _,
    },
};

pub const CHANNEL_CODE: &str = "HyperSwitchBT_Ecom";
pub const CLIENT_TOKEN_MUTATION: &str = "mutation createClientToken($input: CreateClientTokenInput!) { createClientToken(input: $input) { clientToken}}";
pub const TOKENIZE_CREDIT_CARD: &str = "mutation  tokenizeCreditCard($input: TokenizeCreditCardInput!) { tokenizeCreditCard(input: $input) { clientMutationId paymentMethod { id } } }";
pub const CHARGE_CREDIT_CARD_MUTATION: &str = "mutation ChargeCreditCard($input: ChargeCreditCardInput!) { chargeCreditCard(input: $input) { transaction { id legacyId createdAt amount { value currencyCode } status } } }";
pub const AUTHORIZE_CREDIT_CARD_MUTATION: &str = "mutation authorizeCreditCard($input: AuthorizeCreditCardInput!) { authorizeCreditCard(input: $input) {  transaction { id legacyId amount { value currencyCode } status } } }";
pub const CAPTURE_TRANSACTION_MUTATION: &str = "mutation captureTransaction($input: CaptureTransactionInput!) { captureTransaction(input: $input) { clientMutationId transaction { id legacyId amount { value currencyCode } status } } }";
pub const VOID_TRANSACTION_MUTATION: &str = "mutation voidTransaction($input:  ReverseTransactionInput!) { reverseTransaction(input: $input) { clientMutationId reversal { ...  on Transaction { id legacyId amount { value currencyCode } status } } } }";
pub const REFUND_TRANSACTION_MUTATION: &str = "mutation refundTransaction($input:  RefundTransactionInput!) { refundTransaction(input: $input) {clientMutationId refund { id legacyId amount { value currencyCode } status } } }";
pub const AUTHORIZE_AND_VAULT_CREDIT_CARD_MUTATION: &str="mutation authorizeCreditCard($input: AuthorizeCreditCardInput!) { authorizeCreditCard(input: $input) { transaction { id status createdAt paymentMethod { id } } } }";
pub const CHARGE_AND_VAULT_TRANSACTION_MUTATION: &str ="mutation ChargeCreditCard($input: ChargeCreditCardInput!) { chargeCreditCard(input: $input) { transaction { id status createdAt paymentMethod { id } } } }";
pub const DELETE_PAYMENT_METHOD_FROM_VAULT_MUTATION: &str = "mutation deletePaymentMethodFromVault($input: DeletePaymentMethodFromVaultInput!) { deletePaymentMethodFromVault(input: $input) { clientMutationId } }";
pub const TRANSACTION_QUERY: &str = "query($input: TransactionSearchInput!) { search { transactions(input: $input) { edges { node { id status } } } } }";
pub const REFUND_QUERY: &str = "query($input: RefundSearchInput!) { search { refunds(input: $input, first: 1) { edges { node { id status createdAt amount { value currencyCode } orderId } } } } }";
pub const CHARGE_GOOGLE_PAY_MUTATION: &str = "mutation ChargeGPay($input: ChargePaymentMethodInput!) { chargePaymentMethod(input: $input) { transaction { id status amount { value currencyCode } } } }";
pub const AUTHORIZE_GOOGLE_PAY_MUTATION: &str = "mutation authorizeGPay($input: AuthorizePaymentMethodInput!) { authorizePaymentMethod(input: $input) { transaction { id legacyId amount { value currencyCode } status } } }";
pub const CHARGE_APPLE_PAY_MUTATION: &str = "mutation ChargeApplepay($input: ChargePaymentMethodInput!) { chargePaymentMethod(input: $input) { transaction { id status amount { value currencyCode } } } }";
pub const AUTHORIZE_APPLE_PAY_MUTATION: &str = "mutation authorizeApplepay($input: AuthorizePaymentMethodInput!) { authorizePaymentMethod(input: $input) { transaction { id legacyId amount { value currencyCode } status } } }";
pub const CHARGE_AND_VAULT_APPLE_PAY_MUTATION: &str = "mutation ChargeApplepay($input: ChargePaymentMethodInput!) { chargePaymentMethod(input: $input) { transaction { id status amount { value currencyCode } paymentMethod { id } } } }";
pub const AUTHORIZE_AND_VAULT_APPLE_PAY_MUTATION: &str = "mutation authorizeApplepay($input: AuthorizePaymentMethodInput!) { authorizePaymentMethod(input: $input) { transaction { id legacyId amount { value currencyCode } status paymentMethod { id } } } }";
pub const CHARGE_PAYPAL_MUTATION: &str = "mutation ChargePaypal($input: ChargePaymentMethodInput!) { chargePaymentMethod(input: $input) { transaction { id status amount { value currencyCode } } } }";
pub const AUTHORIZE_PAYPAL_MUTATION: &str = "mutation authorizePaypal($input: AuthorizePaymentMethodInput!) { authorizePaymentMethod(input: $input) { transaction { id legacyId amount { value currencyCode } status } } }";

pub type CardPaymentRequest = GenericBraintreeRequest<VariablePaymentInput>;
pub type MandatePaymentRequest = GenericBraintreeRequest<VariablePaymentInput>;
pub type BraintreeClientTokenRequest = GenericBraintreeRequest<VariableClientTokenInput>;
pub type BraintreeTokenRequest = GenericBraintreeRequest<VariableInput>;
pub type BraintreeCaptureRequest = GenericBraintreeRequest<VariableCaptureInput>;
pub type BraintreeRefundRequest = GenericBraintreeRequest<BraintreeRefundVariables>;
pub type BraintreePSyncRequest = GenericBraintreeRequest<PSyncInput>;
pub type BraintreeRSyncRequest = GenericBraintreeRequest<RSyncInput>;

pub type BraintreeRefundResponse = GenericBraintreeResponse<RefundResponse>;
pub type BraintreeCaptureResponse = GenericBraintreeResponse<CaptureResponse>;
pub type BraintreePSyncResponse = GenericBraintreeResponse<PSyncResponse>;

pub type VariablePaymentInput = GenericVariableInput<PaymentInput>;
pub type VariableClientTokenInput = GenericVariableInput<InputClientTokenData>;
pub type VariableInput = GenericVariableInput<InputData>;
pub type VariableCaptureInput = GenericVariableInput<CaptureInputData>;
pub type BraintreeRefundVariables = GenericVariableInput<BraintreeRefundInput>;
pub type PSyncInput = GenericVariableInput<TransactionSearchInput>;
pub type RSyncInput = GenericVariableInput<RefundSearchInput>;

pub type BraintreeWalletRequest = GenericBraintreeRequest<GenericVariableInput<WalletPaymentInput>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletTransactionBody {
    amount: StringMajorUnit,
    merchant_account_id: Secret<String>,
    order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_details: Option<CustomerBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vault_payment_method_after_transacting: Option<TransactionTiming>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPaymentInput {
    payment_method_id: Secret<String>,
    transaction: WalletTransactionBody,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenericBraintreeRequest<T> {
    query: String,
    variables: T,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GenericBraintreeResponse<T> {
    SuccessResponse(Box<T>),
    ErrorResponse(Box<ErrorResponse>),
}
#[derive(Debug, Clone, Serialize)]
pub struct GenericVariableInput<T> {
    input: T,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeApiErrorResponse {
    pub api_error_response: ApiErrorResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorsObject {
    pub errors: Vec<ErrorObject>,

    pub transaction: Option<TransactionError>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionError {
    pub errors: Vec<ErrorObject>,
    pub credit_card: Option<CreditCardError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreditCardError {
    pub errors: Vec<ErrorObject>,
}
#[derive(Debug, Serialize)]
pub struct BraintreeRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(StringMajorUnit, T)> for BraintreeRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (StringMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorObject {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeErrorResponse {
    pub errors: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ErrorResponses {
    BraintreeApiErrorResponse(Box<BraintreeApiErrorResponse>),
    BraintreeErrorResponse(Box<BraintreeErrorResponse>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiErrorResponse {
    pub message: String,
    pub errors: ErrorsObject,
}

pub struct BraintreeAuthType {
    pub(super) public_key: Secret<String>,
    pub(super) private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BraintreeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            api_secret,
            key1: _merchant_id,
        } = item
        {
            Ok(Self {
                public_key: api_key.to_owned(),
                private_key: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    payment_method_id: Secret<String>,
    transaction: TransactionBody,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BraintreePaymentsRequest {
    Card(CardPaymentRequest),
    CardThreeDs(BraintreeClientTokenRequest),
    Mandate(MandatePaymentRequest),
    Wallet(BraintreeWalletRequest),
    Session(BraintreeClientTokenRequest),
}

#[derive(Debug, Deserialize)]
pub struct BraintreeMeta {
    merchant_account_id: Secret<String>,
    merchant_config_currency: enums::Currency,
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
pub struct CustomerBody {
    email: pii::Email,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegularTransactionBody {
    amount: StringMajorUnit,
    merchant_account_id: Secret<String>,
    channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_details: Option<CustomerBody>,
    order_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultTransactionBody {
    amount: StringMajorUnit,
    merchant_account_id: Secret<String>,
    vault_payment_method_after_transacting: TransactionTiming,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_details: Option<CustomerBody>,
    order_id: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TransactionBody {
    Regular(RegularTransactionBody),
    Vault(VaultTransactionBody),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VaultTiming {
    Always,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionTiming {
    when: VaultTiming,
}

impl
    TryFrom<(
        &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
        String,
        BraintreeMeta,
    )> for MandatePaymentRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id, metadata): (
            &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
            String,
            BraintreeMeta,
        ),
    ) -> Result<Self, Self::Error> {
        let (query, transaction_body) = (
            match item.router_data.request.is_auto_capture()? {
                true => CHARGE_CREDIT_CARD_MUTATION.to_string(),
                false => AUTHORIZE_CREDIT_CARD_MUTATION.to_string(),
            },
            TransactionBody::Regular(RegularTransactionBody {
                amount: item.amount.to_owned(),
                merchant_account_id: metadata.merchant_account_id,
                channel: CHANNEL_CODE.to_string(),
                customer_details: None,
                order_id: item.router_data.connector_request_reference_id.clone(),
            }),
        );
        Ok(Self {
            query,
            variables: VariablePaymentInput {
                input: PaymentInput {
                    payment_method_id: connector_mandate_id.into(),
                    transaction: transaction_body,
                },
            },
        })
    }
}

impl TryFrom<&BraintreeRouterData<&types::PaymentsAuthorizeRouterData>>
    for BraintreePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BraintreeRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta = if let (
            Some(merchant_account_id),
            Some(merchant_config_currency),
        ) = (
            item.router_data.request.merchant_account_id.clone(),
            item.router_data.request.merchant_config_currency,
        ) {
            router_env::logger::info!(
                "BRAINTREE: Picking merchant_account_id and merchant_config_currency from payments request"
            );

            BraintreeMeta {
                merchant_account_id,
                merchant_config_currency,
            }
        } else {
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?
        };
        utils::validate_currency(
            item.router_data.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => {
                if item.router_data.is_three_ds() {
                    Ok(Self::CardThreeDs(BraintreeClientTokenRequest::try_from(
                        metadata,
                    )?))
                } else {
                    Ok(Self::Card(CardPaymentRequest::try_from((item, metadata))?))
                }
            }
            PaymentMethodData::MandatePayment => {
                let connector_mandate_id = item.router_data.request.connector_mandate_id().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_mandate_id",
                    },
                )?;
                Ok(Self::Mandate(MandatePaymentRequest::try_from((
                    item,
                    connector_mandate_id,
                    metadata,
                ))?))
            }
            PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                WalletData::GooglePayThirdPartySdk(ref req_wallet) => {
                    let payment_method_id = &req_wallet.token;
                    let query = match item.router_data.request.is_auto_capture()? {
                        true => CHARGE_GOOGLE_PAY_MUTATION.to_string(),
                        false => AUTHORIZE_GOOGLE_PAY_MUTATION.to_string(),
                    };

                    Ok(Self::Wallet(BraintreeWalletRequest {
                        query,
                        variables: GenericVariableInput {
                            input: WalletPaymentInput {
                                payment_method_id: payment_method_id.clone().ok_or(
                                    errors::ConnectorError::MissingRequiredField {
                                        field_name: "google_pay token",
                                    },
                                )?,

                                transaction: WalletTransactionBody {
                                    amount: item.amount.clone(),
                                    merchant_account_id: metadata.merchant_account_id,
                                    order_id: item
                                        .router_data
                                        .connector_request_reference_id
                                        .clone(),
                                    customer_details: None,
                                    vault_payment_method_after_transacting: None,
                                },
                            },
                        },
                    }))
                }
                WalletData::ApplePayThirdPartySdk(ref req_wallet) => {
                    let payment_method_id = &req_wallet.token;
                    let is_mandate = item.router_data.request.is_mandate_payment();
                    let is_auto_capture = item.router_data.request.is_auto_capture()?;

                    let (query, customer_details, vault_payment_method_after_transacting) =
                        if is_mandate {
                            (
                                if is_auto_capture {
                                    CHARGE_AND_VAULT_APPLE_PAY_MUTATION.to_string()
                                } else {
                                    AUTHORIZE_AND_VAULT_APPLE_PAY_MUTATION.to_string()
                                },
                                item.router_data
                                    .get_billing_email()
                                    .ok()
                                    .map(|email| CustomerBody { email }),
                                Some(TransactionTiming {
                                    when: VaultTiming::Always,
                                }),
                            )
                        } else {
                            (
                                if is_auto_capture {
                                    CHARGE_APPLE_PAY_MUTATION.to_string()
                                } else {
                                    AUTHORIZE_APPLE_PAY_MUTATION.to_string()
                                },
                                None,
                                None,
                            )
                        };

                    Ok(Self::Wallet(BraintreeWalletRequest {
                        query,
                        variables: GenericVariableInput {
                            input: WalletPaymentInput {
                                payment_method_id: payment_method_id.clone().ok_or(
                                    errors::ConnectorError::MissingRequiredField {
                                        field_name: "apple_pay token",
                                    },
                                )?,
                                transaction: WalletTransactionBody {
                                    amount: item.amount.clone(),
                                    merchant_account_id: metadata.merchant_account_id,
                                    order_id: item
                                        .router_data
                                        .connector_request_reference_id
                                        .clone(),
                                    customer_details,
                                    vault_payment_method_after_transacting,
                                },
                            },
                        },
                    }))
                }
                WalletData::PaypalSdk(ref req_wallet) => {
                    let payment_method_id = req_wallet.token.clone();
                    let query = match item.router_data.request.is_auto_capture()? {
                        true => CHARGE_PAYPAL_MUTATION.to_string(),
                        false => AUTHORIZE_PAYPAL_MUTATION.to_string(),
                    };
                    Ok(Self::Wallet(BraintreeWalletRequest {
                        query,
                        variables: GenericVariableInput {
                            input: WalletPaymentInput {
                                payment_method_id: payment_method_id.clone().into(),
                                transaction: WalletTransactionBody {
                                    amount: item.amount.clone(),
                                    merchant_account_id: metadata.merchant_account_id,
                                    order_id: item
                                        .router_data
                                        .connector_request_reference_id
                                        .clone(),
                                    customer_details: None,
                                    vault_payment_method_after_transacting: None,
                                },
                            },
                        },
                    }))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("braintree"),
                )
                .into()),
            },
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
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
            | api_models::enums::PaymentMethod::RealTimePayment
            | api_models::enums::PaymentMethod::MobilePayment
            | api_models::enums::PaymentMethod::Upi
            | api_models::enums::PaymentMethod::OpenBanking
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthResponse {
    data: DataAuthResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeAuthResponse {
    AuthResponse(Box<AuthResponse>),
    ClientTokenResponse(Box<ClientTokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
    WalletAuthResponse(Box<WalletAuthResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeCompleteAuthResponse {
    AuthResponse(Box<AuthResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentMethodInfo {
    id: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionAuthChargeResponseBody {
    id: String,
    status: BraintreePaymentStatus,
    payment_method: Option<PaymentMethodInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAuthResponse {
    authorize_credit_card: AuthChargeCreditCard,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthChargeCreditCard {
    transaction: TransactionAuthChargeResponseBody,
}

impl TryFrom<PaymentsResponseRouterData<BraintreeAuthResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<BraintreeAuthResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeAuthResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreeAuthResponse::AuthResponse(auth_response) => {
                let transaction_data = auth_response.data.authorize_credit_card.transaction;
                let status = enums::AttemptStatus::from(transaction_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: transaction_data.status.to_string(),
                        message: transaction_data.status.to_string(),
                        reason: Some(transaction_data.status.to_string()),
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_data.id),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(transaction_data.payment_method.as_ref().map(
                            |pm| MandateReference {
                                connector_mandate_id: Some(pm.id.clone().expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            BraintreeAuthResponse::ClientTokenResponse(client_token_data) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(Some(get_braintree_redirect_form(
                        *client_token_data,
                        item.data.get_payment_method_token()?,
                        item.data.request.payment_method_data.clone(),
                        item.data.request.get_complete_authorize_url()?,
                    )?)),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            BraintreeAuthResponse::WalletAuthResponse(wallet_response) => {
                let txn = &wallet_response.data.authorize_payment_method.transaction;
                let status = enums::AttemptStatus::from(txn.status.clone());

                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: txn.status.to_string(),
                        message: txn.status.to_string(),
                        reason: Some(txn.status.to_string()),
                        attempt_status: None,
                        connector_transaction_id: Some(txn.id.clone()),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(txn.id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: txn.legacy_id.clone(),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };

                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

fn build_error_response<T>(
    response: &[ErrorDetails],
    http_code: u16,
) -> Result<T, Box<hyperswitch_domain_models::router_data::ErrorResponse>> {
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
            .first()
            .and_then(|err_details| err_details.extensions.as_ref())
            .and_then(|extensions| extensions.legacy_code.clone()),
        response
            .first()
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
) -> Result<T, Box<hyperswitch_domain_models::router_data::ErrorResponse>> {
    Err(Box::new(
        hyperswitch_domain_models::router_data::ErrorResponse {
            code: error_code.unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: error_msg.unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: error_reason,
            status_code: http_code,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        },
    ))
}

#[derive(Debug, Clone, Deserialize, Serialize, strum::Display)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub message: String,
    pub extensions: Option<AdditionalErrorDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalErrorDetails {
    pub legacy_code: Option<String>,
}

impl From<BraintreePaymentStatus> for enums::AttemptStatus {
    fn from(item: BraintreePaymentStatus) -> Self {
        match item {
            BraintreePaymentStatus::Settling
            | BraintreePaymentStatus::Settled
            | BraintreePaymentStatus::SettlementConfirmed
            | BraintreePaymentStatus::SubmittedForSettlement
            | BraintreePaymentStatus::SettlementPending => Self::Charged,
            BraintreePaymentStatus::Authorizing => Self::Authorizing,
            BraintreePaymentStatus::AuthorizedExpired => Self::AuthorizationFailed,
            BraintreePaymentStatus::Failed
            | BraintreePaymentStatus::GatewayRejected
            | BraintreePaymentStatus::ProcessorDeclined
            | BraintreePaymentStatus::SettlementDeclined => Self::Failure,
            BraintreePaymentStatus::Authorized => Self::Authorized,
            BraintreePaymentStatus::Voided => Self::Voided,
        }
    }
}

impl TryFrom<PaymentsResponseRouterData<BraintreePaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<BraintreePaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreePaymentsResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors.clone(), item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreePaymentsResponse::PaymentsResponse(payment_response) => {
                let transaction_data = payment_response.data.charge_credit_card.transaction;
                let status = enums::AttemptStatus::from(transaction_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: transaction_data.status.to_string().clone(),
                        message: transaction_data.status.to_string().clone(),
                        reason: Some(transaction_data.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_data.id),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(transaction_data.payment_method.as_ref().map(
                            |pm| MandateReference {
                                connector_mandate_id: Some(pm.id.clone().expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            BraintreePaymentsResponse::ClientTokenResponse(client_token_data) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(Some(get_braintree_redirect_form(
                        *client_token_data,
                        item.data.get_payment_method_token()?,
                        item.data.request.payment_method_data.clone(),
                        item.data.request.get_complete_authorize_url()?,
                    )?)),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            BraintreePaymentsResponse::WalletPaymentsResponse(wallet_payments_response) => {
                let txn = &wallet_payments_response
                    .data
                    .charge_payment_method
                    .transaction;
                let status = enums::AttemptStatus::from(txn.status.clone());

                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: txn.status.to_string(),
                        message: txn.status.to_string(),
                        reason: Some(txn.status.to_string()),
                        attempt_status: None,
                        connector_transaction_id: Some(txn.id.clone()),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(txn.id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(txn.payment_method.as_ref().map(|pm| {
                            MandateReference {
                                connector_mandate_id: Some(pm.id.clone().expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            }
                        })),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };

                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BraintreeCompleteChargeResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BraintreeCompleteChargeResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCompleteChargeResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors.clone(), item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreeCompleteChargeResponse::PaymentsResponse(payment_response) => {
                let transaction_data = payment_response.data.charge_credit_card.transaction;
                let status = enums::AttemptStatus::from(transaction_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: transaction_data.status.to_string().clone(),
                        message: transaction_data.status.to_string().clone(),
                        reason: Some(transaction_data.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_data.id),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(transaction_data.payment_method.as_ref().map(
                            |pm| MandateReference {
                                connector_mandate_id: Some(pm.id.clone().expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BraintreeCompleteAuthResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BraintreeCompleteAuthResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCompleteAuthResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreeCompleteAuthResponse::AuthResponse(auth_response) => {
                let transaction_data = auth_response.data.authorize_credit_card.transaction;
                let status = enums::AttemptStatus::from(transaction_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: transaction_data.status.to_string().clone(),
                        message: transaction_data.status.to_string().clone(),
                        reason: Some(transaction_data.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_data.id),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(transaction_data.payment_method.as_ref().map(
                            |pm| MandateReference {
                                connector_mandate_id: Some(pm.id.clone().expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentsResponse {
    data: DataResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalletPaymentsResponse {
    pub data: WalletDataResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDataResponse {
    pub charge_payment_method: WalletTransactionWrapper,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalletTransactionWrapper {
    pub transaction: WalletTransaction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletTransaction {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_id: Option<String>,
    pub status: BraintreePaymentStatus,
    pub amount: Amount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<PaymentMethodInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    pub value: String,
    pub currency_code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalletAuthResponse {
    pub data: WalletAuthDataResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletAuthDataResponse {
    pub authorize_payment_method: WalletTransactionWrapper,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreePaymentsResponse {
    PaymentsResponse(Box<PaymentsResponse>),
    WalletPaymentsResponse(Box<WalletPaymentsResponse>),
    ClientTokenResponse(Box<ClientTokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeCompleteChargeResponse {
    PaymentsResponse(Box<PaymentsResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataResponse {
    charge_credit_card: AuthChargeCreditCard,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundInputData {
    amount: StringMajorUnit,
    merchant_account_id: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order_id: Option<String>,
}
#[derive(Serialize, Debug, Clone)]
struct IdFilter {
    is: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionSearchInput {
    id: IdFilter,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeRefundInput {
    transaction_id: String,
    refund: RefundInputData,
}

impl<F> TryFrom<BraintreeRouterData<&RefundsRouterData<F>>> for BraintreeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: BraintreeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta = if let (
            Some(merchant_account_id),
            Some(merchant_config_currency),
        ) = (
            item.router_data.request.merchant_account_id.clone(),
            item.router_data.request.merchant_config_currency,
        ) {
            router_env::logger::info!(
                "BRAINTREE: Picking merchant_account_id and merchant_config_currency from payments request"
            );

            BraintreeMeta {
                merchant_account_id,
                merchant_config_currency,
            }
        } else {
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?
        };

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
                    order_id: item.router_data.refund_id.clone(),
                },
            },
        };
        Ok(Self { query, variables })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, strum::Display)]
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
            BraintreeRefundStatus::Settled
            | BraintreeRefundStatus::Settling
            | BraintreeRefundStatus::SubmittedForSettlement
            | BraintreeRefundStatus::SettlementPending => Self::Success,
            BraintreeRefundStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BraintreeRefundTransactionBody {
    pub id: String,
    pub status: BraintreeRefundStatus,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BraintreeRefundTransaction {
    pub refund: BraintreeRefundTransactionBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeRefundResponseData {
    pub refund_transaction: BraintreeRefundTransaction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefundResponse {
    pub data: BraintreeRefundResponseData,
}

impl TryFrom<RefundsResponseRouterData<Execute, BraintreeRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, BraintreeRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response {
                BraintreeRefundResponse::ErrorResponse(error_response) => {
                    build_error_response(&error_response.errors, item.http_code).map_err(|err| *err)
                }
                BraintreeRefundResponse::SuccessResponse(refund_data) => {
                    let refund_data = refund_data.data.refund_transaction.refund;
                    let refund_status = enums::RefundStatus::from(refund_data.status.clone());
                    if utils::is_refund_failure(refund_status) {
                        Err(hyperswitch_domain_models::router_data::ErrorResponse {
                            code: refund_data.status.to_string().clone(),
                            message: refund_data.status.to_string().clone(),
                            reason: Some(refund_data.status.to_string().clone()),
                            attempt_status: None,
                            connector_transaction_id: Some(refund_data.id),
                            status_code: item.http_code,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    } else {
                        Ok(RefundsResponseData {
                            connector_refund_id: refund_data.id.clone(),
                            refund_status,
                        })
                    }
                }
            },
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundSearchInput {
    id: IdFilter,
}
impl TryFrom<&types::RefundSyncRouterData> for BraintreeRSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta = if let (
            Some(merchant_account_id),
            Some(merchant_config_currency),
        ) = (
            item.request.merchant_account_id.clone(),
            item.request.merchant_config_currency,
        ) {
            router_env::logger::info!(
                "BRAINTREE: Picking merchant_account_id and merchant_config_currency from payments request"
            );

            BraintreeMeta {
                merchant_account_id,
                merchant_config_currency,
            }
        } else {
            utils::to_connector_meta_from_secret(item.connector_meta_data.clone()).change_context(
                errors::ConnectorError::InvalidConnectorConfig { config: "metadata" },
            )?
        };
        utils::validate_currency(
            item.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        let refund_id = item.request.get_connector_refund_id()?;
        Ok(Self {
            query: REFUND_QUERY.to_string(),
            variables: RSyncInput {
                input: RefundSearchInput {
                    id: IdFilter { is: refund_id },
                },
            },
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RSyncNodeData {
    id: String,
    status: BraintreeRefundStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RSyncEdgeData {
    node: RSyncNodeData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefundData {
    edges: Vec<RSyncEdgeData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RSyncSearchData {
    refunds: RefundData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RSyncResponseData {
    search: RSyncSearchData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RSyncResponse {
    data: RSyncResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeRSyncResponse {
    RSyncResponse(Box<RSyncResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl TryFrom<RefundsResponseRouterData<RSync, BraintreeRSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, BraintreeRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeRSyncResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code)
                    .map_err(|err| *err),
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
                let response = Ok(RefundsResponseData {
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditCardData {
    number: cards::CardNumber,
    expiration_year: Secret<String>,
    expiration_month: Secret<String>,
    cvv: Secret<String>,
    cardholder_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTokenInput {
    merchant_account_id: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputData {
    credit_card: CreditCardData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputClientTokenData {
    client_token: ClientTokenInput,
}

impl TryFrom<&types::TokenizationRouterData> for BraintreeTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => Ok(Self {
                query: TOKENIZE_CREDIT_CARD.to_string(),
                variables: VariableInput {
                    input: InputData {
                        credit_card: CreditCardData {
                            number: card_data.card_number,
                            expiration_year: card_data.card_exp_year,
                            expiration_month: card_data.card_exp_month,
                            cvv: card_data.card_cvc,
                            cardholder_name: item
                                .get_optional_billing_full_name()
                                .unwrap_or(Secret::new("".to_string())),
                        },
                    },
                },
            }),
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("braintree"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenizePaymentMethodData {
    id: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizeCreditCardData {
    payment_method: TokenizePaymentMethodData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientToken {
    client_token: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizeCreditCard {
    tokenize_credit_card: TokenizeCreditCardData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTokenData {
    create_client_token: ClientToken,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTokenExtensions {
    request_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientTokenResponse {
    data: ClientTokenData,
    extensions: ClientTokenExtensions,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    data: TokenizeCreditCard,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorResponse {
    errors: Vec<ErrorDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeTokenResponse {
    TokenResponse(Box<TokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeSessionResponse {
    SessionTokenResponse(Box<ClientTokenResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl<F, T> TryFrom<ResponseRouterData<F, BraintreeTokenResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BraintreeTokenResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response {
                BraintreeTokenResponse::ErrorResponse(error_response) => {
                    build_error_response(error_response.errors.as_ref(), item.http_code)
                        .map_err(|err| *err)
                }

                BraintreeTokenResponse::TokenResponse(token_response) => {
                    Ok(PaymentsResponseData::TokenizationResponse {
                        token: token_response
                            .data
                            .tokenize_credit_card
                            .payment_method
                            .id
                            .expose()
                            .clone(),
                    })
                }
            },
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Display, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GooglePayPriceStatus {
    #[strum(serialize = "FINAL")]
    Final,
}

#[derive(Debug, Clone, Display, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaypalFlow {
    Checkout,
}

impl From<PaypalFlow> for payment_types::PaypalFlow {
    fn from(item: PaypalFlow) -> Self {
        match item {
            PaypalFlow::Checkout => Self::Checkout,
        }
    }
}

impl
    ForeignTryFrom<(
        PaymentsSessionResponseRouterData<BraintreeSessionResponse>,
        Self,
    )> for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, data): (
            PaymentsSessionResponseRouterData<BraintreeSessionResponse>,
            Self,
        ),
    ) -> Result<Self, Self::Error> {
        let response = &item.response;

        match response {
            BraintreeSessionResponse::SessionTokenResponse(res) => {
                let session_token = match data.payment_method_type {
                    Some(common_enums::PaymentMethodType::ApplePay) => {
                        let payment_request_data: payment_types::PaymentRequestMetadata =
                            if let Some(connector_meta) = data.connector_meta_data.clone() {
                                let meta_value: serde_json::Value = connector_meta.expose();
                                meta_value
                                    .get("apple_pay_combined")
                                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                                    .attach_printable("Missing apple_pay_combined metadata")?
                                    .get("manual")
                                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                                    .attach_printable("Missing manual metadata")?
                                    .get("payment_request_data")
                                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                                    .attach_printable("Missing payment_request_data metadata")?
                                    .clone()
                                    .parse_value("PaymentRequestMetadata")
                                    .change_context(errors::ConnectorError::ParsingFailed)
                                    .attach_printable(
                                        "Failed to parse apple_pay_combined.manual.payment_request_data metadata",
                                    )?
                            } else {
                                return Err(errors::ConnectorError::NoConnectorMetaData)
                                    .attach_printable("connector_meta_data is None");
                            };

                        let session_token_data = Some(ApplePaySessionResponse::ThirdPartySdk(
                            payment_types::ThirdPartySdkSessionResponse {
                                secrets: payment_types::SecretInfoToInitiateSdk {
                                    display: res.data.create_client_token.client_token.clone(),
                                    payment: None,
                                },
                            },
                        ));
                        SessionToken::ApplePay(Box::new(
                            api_models::payments::ApplepaySessionTokenResponse {
                                session_token_data,
                                payment_request_data: Some(
                                    api_models::payments::ApplePayPaymentRequest {
                                        country_code: data.request.country.ok_or(
                                            errors::ConnectorError::MissingRequiredField {
                                                field_name: "country",
                                            },
                                        )?,
                                        currency_code: data.request.currency,
                                        total: api_models::payments::AmountInfo {
                                            label: payment_request_data.label,
                                            total_type: None,
                                            amount: StringMajorUnitForConnector
                                                .convert(
                                                    MinorUnit::new(data.request.amount),
                                                    data.request.currency,
                                                )
                                                .change_context(
                                                    errors::ConnectorError::AmountConversionFailed,
                                                )?,
                                        },
                                        merchant_capabilities: Some(
                                            payment_request_data.merchant_capabilities,
                                        ),
                                        supported_networks: Some(
                                            payment_request_data.supported_networks,
                                        ),
                                        merchant_identifier: None,
                                        required_billing_contact_fields: None,
                                        required_shipping_contact_fields: None,
                                        recurring_payment_request: None,
                                    },
                                ),
                                connector: data.connector.clone(),
                                delayed_session_token: false,
                                sdk_next_action: api_models::payments::SdkNextAction {
                                    next_action: api_models::payments::NextActionCall::Confirm,
                                },
                                connector_reference_id: None,
                                connector_sdk_public_key: None,
                                connector_merchant_id: None,
                            },
                        ))
                    }
                    Some(common_enums::PaymentMethodType::GooglePay) => {
                        let gpay_data: payment_types::GpaySessionTokenData =
                            if let Some(connector_meta) = data.connector_meta_data.clone() {
                                connector_meta
                                    .expose()
                                    .parse_value("GpaySessionTokenData")
                                    .change_context(errors::ConnectorError::ParsingFailed)
                                    .attach_printable("Failed to parse gpay metadata")?
                            } else {
                                return Err(errors::ConnectorError::NoConnectorMetaData)
                                    .attach_printable("connector_meta_data is None");
                            };

                        SessionToken::GooglePay(Box::new(
                            api_models::payments::GpaySessionTokenResponse::GooglePaySession(
                                api_models::payments::GooglePaySessionResponse {
                                    merchant_info: payment_types::GpayMerchantInfo {
                                        merchant_name: gpay_data.data.merchant_info.merchant_name,
                                        merchant_id: gpay_data.data.merchant_info.merchant_id,
                                    },
                                    shipping_address_required: false,
                                    email_required: false,
                                    shipping_address_parameters:
                                        payment_types::GpayShippingAddressParameters {
                                            phone_number_required: false,
                                        },
                                    allowed_payment_methods: gpay_data.data.allowed_payment_methods,
                                    transaction_info: payment_types::GpayTransactionInfo {
                                        country_code: data.request.country.ok_or(
                                            errors::ConnectorError::MissingRequiredField {
                                                field_name: "country",
                                            },
                                        )?,
                                        currency_code: data.request.currency,
                                        total_price_status: GooglePayPriceStatus::Final.to_string(),
                                        total_price: StringMajorUnitForConnector
                                            .convert(
                                                MinorUnit::new(data.request.amount),
                                                data.request.currency,
                                            )
                                            .change_context(
                                                errors::ConnectorError::AmountConversionFailed,
                                            )?,
                                    },
                                    secrets: Some(payment_types::SecretInfoToInitiateSdk {
                                        display: res.data.create_client_token.client_token.clone(),
                                        payment: None,
                                    }),
                                    delayed_session_token: false,
                                    connector: data.connector.clone(),
                                    sdk_next_action: payment_types::SdkNextAction {
                                        next_action: payment_types::NextActionCall::Confirm,
                                    },
                                },
                            ),
                        ))
                    }
                    Some(common_enums::PaymentMethodType::Paypal) => {
                        let paypal_sdk_data = data
                            .connector_meta_data
                            .clone()
                            .parse_value::<payment_types::PaypalSdkSessionTokenData>(
                                "PaypalSdkSessionTokenData",
                            )
                            .change_context(errors::ConnectorError::NoConnectorMetaData)
                            .attach_printable("Failed to parse paypal_sdk metadata.".to_string())?;

                        SessionToken::Paypal(Box::new(
                            api_models::payments::PaypalSessionTokenResponse {
                                connector: data.connector.clone(),
                                session_token: paypal_sdk_data.data.client_id,
                                sdk_next_action: api_models::payments::SdkNextAction {
                                    next_action: api_models::payments::NextActionCall::Confirm,
                                },
                                client_token: Some(
                                    res.data.create_client_token.client_token.clone().expose(),
                                ),
                                transaction_info: Some(
                                    api_models::payments::PaypalTransactionInfo {
                                        flow: PaypalFlow::Checkout.into(),
                                        currency_code: data.request.currency,
                                        total_price: StringMajorUnitForConnector
                                            .convert(
                                                MinorUnit::new(data.request.amount),
                                                data.request.currency,
                                            )
                                            .change_context(
                                                errors::ConnectorError::AmountConversionFailed,
                                            )?,
                                    },
                                ),
                            },
                        ))
                    }
                    _ => {
                        return Err(errors::ConnectorError::NotImplemented(
                            format!(
                                "SDK session token generation is not supported for payment method: {:?}",
                                data.payment_method_type
                            )
                        )
                        .into());
                    }
                };

                Ok(Self {
                    response: Ok(PaymentsResponseData::SessionResponse { session_token }),
                    ..data
                })
            }
            BraintreeSessionResponse::ErrorResponse(error_response) => {
                let err = build_error_response(error_response.errors.as_ref(), item.http_code)
                    .map_err(|err| *err);
                Ok(Self {
                    response: err,
                    ..data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureTransactionBody {
    amount: StringMajorUnit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInputData {
    transaction_id: String,
    transaction: CaptureTransactionBody,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaptureResponseTransactionBody {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaptureTransactionData {
    transaction: CaptureResponseTransactionBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponseData {
    capture_transaction: CaptureTransactionData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaptureResponse {
    data: CaptureResponseData,
}

impl TryFrom<PaymentsCaptureResponseRouterData<BraintreeCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<BraintreeCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCaptureResponse::SuccessResponse(capture_data) => {
                let transaction_data = capture_data.data.capture_transaction.transaction;
                let status = enums::AttemptStatus::from(transaction_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: transaction_data.status.to_string().clone(),
                        message: transaction_data.status.to_string().clone(),
                        reason: Some(transaction_data.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_data.id),
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_data.id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            BraintreeCaptureResponse::ErrorResponse(error_data) => Ok(Self {
                response: build_error_response(&error_data.errors, item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePaymentMethodFromVaultInputData {
    payment_method_id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct VariableDeletePaymentMethodFromVaultInput {
    input: DeletePaymentMethodFromVaultInputData,
}

#[derive(Debug, Serialize)]
pub struct BraintreeRevokeMandateRequest {
    query: String,
    variables: VariableDeletePaymentMethodFromVaultInput,
}

impl TryFrom<&types::MandateRevokeRouterData> for BraintreeRevokeMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::MandateRevokeRouterData) -> Result<Self, Self::Error> {
        let query = DELETE_PAYMENT_METHOD_FROM_VAULT_MUTATION.to_string();
        let variables = VariableDeletePaymentMethodFromVaultInput {
            input: DeletePaymentMethodFromVaultInputData {
                payment_method_id: Secret::new(
                    item.request
                        .connector_mandate_id
                        .clone()
                        .ok_or(errors::ConnectorError::MissingConnectorMandateID)?,
                ),
            },
        };
        Ok(Self { query, variables })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BraintreeRevokeMandateResponse,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        >,
    > for RouterData<F, MandateRevokeRequestData, MandateRevokeResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BraintreeRevokeMandateResponse,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response {
                BraintreeRevokeMandateResponse::ErrorResponse(error_response) => {
                    build_error_response(error_response.errors.as_ref(), item.http_code)
                        .map_err(|err| *err)
                }
                BraintreeRevokeMandateResponse::RevokeMandateResponse(..) => {
                    Ok(MandateRevokeResponseData {
                        mandate_status: common_enums::MandateStatus::Revoked,
                    })
                }
            },
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeRevokeMandateResponse {
    RevokeMandateResponse(Box<RevokeMandateResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeMandateResponse {
    data: DeletePaymentMethodFromVault,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePaymentMethodFromVault {
    client_mutation_id: Option<String>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelResponseTransactionBody {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelTransactionData {
    reversal: CancelResponseTransactionBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponseData {
    reverse_transaction: CancelTransactionData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelResponse {
    data: CancelResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BraintreeCancelResponse {
    CancelResponse(Box<CancelResponse>),
    ErrorResponse(Box<ErrorResponse>),
}

impl<F, T> TryFrom<ResponseRouterData<F, BraintreeCancelResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BraintreeCancelResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreeCancelResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreeCancelResponse::CancelResponse(void_response) => {
                let void_data = void_response.data.reverse_transaction.reversal;
                let status = enums::AttemptStatus::from(void_data.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: void_data.status.to_string().clone(),
                        message: void_data.status.to_string().clone(),
                        reason: Some(void_data.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for BraintreePSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(Self {
            query: TRANSACTION_QUERY.to_string(),
            variables: PSyncInput {
                input: TransactionSearchInput {
                    id: IdFilter { is: transaction_id },
                },
            },
        })
    }
}

impl TryFrom<&types::PaymentsSessionRouterData> for BraintreePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let metadata: BraintreeMeta = utils::to_connector_meta_from_secret(
            item.connector_meta_data.clone(),
        )
        .change_context(errors::ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
        Ok(Self::Session(BraintreeClientTokenRequest {
            query: CLIENT_TOKEN_MUTATION.to_owned(),
            variables: VariableClientTokenInput {
                input: InputClientTokenData {
                    client_token: ClientTokenInput {
                        merchant_account_id: metadata.merchant_account_id,
                    },
                },
            },
        }))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeData {
    id: String,
    status: BraintreePaymentStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeData {
    node: NodeData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionData {
    edges: Vec<EdgeData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchData {
    transactions: TransactionData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PSyncResponseData {
    search: SearchData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PSyncResponse {
    data: PSyncResponseData,
}

impl<F, T> TryFrom<ResponseRouterData<F, BraintreePSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BraintreePSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BraintreePSyncResponse::ErrorResponse(error_response) => Ok(Self {
                response: build_error_response(&error_response.errors, item.http_code)
                    .map_err(|err| *err),
                ..item.data
            }),
            BraintreePSyncResponse::SuccessResponse(psync_response) => {
                let edge_data = psync_response
                    .data
                    .search
                    .transactions
                    .edges
                    .first()
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;
                let status = enums::AttemptStatus::from(edge_data.node.status.clone());
                let response = if utils::is_payment_failure(status) {
                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: edge_data.node.status.to_string().clone(),
                        message: edge_data.node.status.to_string().clone(),
                        reason: Some(edge_data.node.status.to_string().clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(edge_data.node.id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BraintreeThreeDsResponse {
    pub nonce: Secret<String>,
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
        let (query, transaction_body) = if item.router_data.request.is_mandate_payment() {
            (
                match item.router_data.request.is_auto_capture()? {
                    true => CHARGE_AND_VAULT_TRANSACTION_MUTATION.to_string(),
                    false => AUTHORIZE_AND_VAULT_CREDIT_CARD_MUTATION.to_string(),
                },
                TransactionBody::Vault(VaultTransactionBody {
                    amount: item.amount.to_owned(),
                    merchant_account_id: metadata.merchant_account_id,
                    vault_payment_method_after_transacting: TransactionTiming {
                        when: VaultTiming::Always,
                    },
                    customer_details: item
                        .router_data
                        .get_billing_email()
                        .ok()
                        .map(|email| CustomerBody { email }),
                    order_id: item.router_data.connector_request_reference_id.clone(),
                }),
            )
        } else {
            (
                match item.router_data.request.is_auto_capture()? {
                    true => CHARGE_CREDIT_CARD_MUTATION.to_string(),
                    false => AUTHORIZE_CREDIT_CARD_MUTATION.to_string(),
                },
                TransactionBody::Regular(RegularTransactionBody {
                    amount: item.amount.to_owned(),
                    merchant_account_id: metadata.merchant_account_id,
                    channel: CHANNEL_CODE.to_string(),
                    customer_details: item
                        .router_data
                        .get_billing_email()
                        .ok()
                        .map(|email| CustomerBody { email }),
                    order_id: item.router_data.connector_request_reference_id.clone(),
                }),
            )
        };
        Ok(Self {
            query,
            variables: VariablePaymentInput {
                input: PaymentInput {
                    payment_method_id: match item.router_data.get_payment_method_token()? {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Braintree"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Braintree"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Braintree"))?
                        }
                    },
                    transaction: transaction_body,
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
        let metadata: BraintreeMeta = if let (
            Some(merchant_account_id),
            Some(merchant_config_currency),
        ) = (
            item.router_data.request.merchant_account_id.clone(),
            item.router_data.request.merchant_config_currency,
        ) {
            router_env::logger::info!(
                "BRAINTREE: Picking merchant_account_id and merchant_config_currency from payments request"
            );

            BraintreeMeta {
                merchant_account_id,
                merchant_config_currency,
            }
        } else {
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?
        };
        utils::validate_currency(
            item.router_data.request.currency,
            Some(metadata.merchant_config_currency),
        )?;
        let payload_data = PaymentsCompleteAuthorizeRequestData::get_redirect_response_payload(
            &item.router_data.request,
        )?
        .expose();
        let redirection_response: BraintreeRedirectionResponse = serde_json::from_value(
            payload_data,
        )
        .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
            field_name: "redirection_response",
        })?;
        let three_ds_data = serde_json::from_str::<BraintreeThreeDsResponse>(
            &redirection_response.authentication_response,
        )
        .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
            field_name: "three_ds_data",
        })?;

        let (query, transaction_body) = if item.router_data.request.is_mandate_payment() {
            (
                match item.router_data.request.is_auto_capture()? {
                    true => CHARGE_AND_VAULT_TRANSACTION_MUTATION.to_string(),
                    false => AUTHORIZE_AND_VAULT_CREDIT_CARD_MUTATION.to_string(),
                },
                TransactionBody::Vault(VaultTransactionBody {
                    amount: item.amount.to_owned(),
                    merchant_account_id: metadata.merchant_account_id,
                    vault_payment_method_after_transacting: TransactionTiming {
                        when: VaultTiming::Always,
                    },
                    customer_details: item
                        .router_data
                        .get_billing_email()
                        .ok()
                        .map(|email| CustomerBody { email }),
                    order_id: item.router_data.connector_request_reference_id.clone(),
                }),
            )
        } else {
            (
                match item.router_data.request.is_auto_capture()? {
                    true => CHARGE_CREDIT_CARD_MUTATION.to_string(),
                    false => AUTHORIZE_CREDIT_CARD_MUTATION.to_string(),
                },
                TransactionBody::Regular(RegularTransactionBody {
                    amount: item.amount.to_owned(),
                    merchant_account_id: metadata.merchant_account_id,
                    channel: CHANNEL_CODE.to_string(),
                    customer_details: item
                        .router_data
                        .get_billing_email()
                        .ok()
                        .map(|email| CustomerBody { email }),
                    order_id: item.router_data.connector_request_reference_id.clone(),
                }),
            )
        };
        Ok(Self {
            query,
            variables: VariablePaymentInput {
                input: PaymentInput {
                    payment_method_id: three_ds_data.nonce,
                    transaction: transaction_body,
                },
            },
        })
    }
}

fn get_braintree_redirect_form(
    client_token_data: ClientTokenResponse,
    payment_method_token: PaymentMethodToken,
    card_details: PaymentMethodData,
    complete_authorize_url: String,
) -> Result<RedirectForm, error_stack::Report<errors::ConnectorError>> {
    Ok(RedirectForm::Braintree {
        client_token: client_token_data
            .data
            .create_client_token
            .client_token
            .expose(),
        card_token: match payment_method_token {
            PaymentMethodToken::Token(token) => token.expose(),
            PaymentMethodToken::ApplePayDecrypt(_) => Err(unimplemented_payment_method!(
                "Apple Pay",
                "Simplified",
                "Braintree"
            ))?,
            PaymentMethodToken::PazeDecrypt(_) => {
                Err(unimplemented_payment_method!("Paze", "Braintree"))?
            }
            PaymentMethodToken::GooglePayDecrypt(_) => {
                Err(unimplemented_payment_method!("Google Pay", "Braintree"))?
            }
        },
        bin: match card_details {
            PaymentMethodData::Card(card_details) => card_details.card_number.get_card_isin(),
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => Err(
                errors::ConnectorError::NotImplemented("given payment method".to_owned()),
            )?,
        },
        acs_url: complete_authorize_url,
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

pub(crate) fn get_status(status: &str) -> IncomingWebhookEvent {
    match status {
        "dispute_opened" => IncomingWebhookEvent::DisputeOpened,
        "dispute_lost" => IncomingWebhookEvent::DisputeLost,
        "dispute_won" => IncomingWebhookEvent::DisputeWon,
        "dispute_accepted" | "dispute_auto_accepted" => IncomingWebhookEvent::DisputeAccepted,
        "dispute_expired" => IncomingWebhookEvent::DisputeExpired,
        "dispute_disputed" => IncomingWebhookEvent::DisputeChallenged,
        _ => IncomingWebhookEvent::EventNotSupported,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BraintreeDisputeData {
    pub amount_disputed: MinorUnit,
    pub amount_won: Option<String>,
    pub case_number: Option<String>,
    pub chargeback_protection_level: Option<String>,
    pub currency_iso_code: enums::Currency,
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
    pub amount: StringMajorUnit,
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
        "PRE_ARBITRATION" => Ok(enums::DisputeStage::PreArbitration),
        "RETRIEVAL" => Ok(enums::DisputeStage::PreDispute),
        _ => Err(errors::ConnectorError::WebhookBodyDecodingFailed),
    }
}
