use common_enums::enums;
use common_types::payments::{ApplePayPredecryptData, GPayPredecryptData};
use common_utils::types::FloatMajorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{
        ApplePayWalletData, Card, GooglePayWalletData, PaymentMethodData, WalletData,
    },
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefreshTokenRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self as connector_utils, CardData, PaymentsAuthorizeRequestData, PaymentsSyncRequestData,
        RefundsRequestData, RouterData as _,
    },
};

pub mod tesouro_queries {
    pub const AUTHORIZE_TRANSACTION: &str = "mutation AuthorizeCustomerInitiatedTransaction($authorizeCustomerInitiatedTransactionInput: AuthorizeCustomerInitiatedTransactionInput!) { authorizeCustomerInitiatedTransaction(authorizeCustomerInitiatedTransactionInput: $authorizeCustomerInitiatedTransactionInput) { authorizationResponse { paymentId transactionId __typename ... on AuthorizationApproval { __typename paymentId transactionId } ... on AuthorizationDecline { __typename transactionId paymentId } } errors { ... on InternalServiceError { message transactionId processorResponseCode } ... on AcceptorNotFoundError { message transactionId processorResponseCode } ... on RuleInViolationError { message transactionId processorResponseCode } ... on SyntaxOnNetworkResponseError {  message transactionId processorResponseCode } ... on TimeoutOnNetworkResponseError { message transactionId processorResponseCode } ... on ValidationFailureError {  message processorResponseCode transactionId } ... on UnknownCardError {  message processorResponseCode transactionId } ... on TokenNotFoundError {  message processorResponseCode transactionId } ... on InvalidTokenError {  message processorResponseCode transactionId } ... on RouteNotFoundError {  message processorResponseCode transactionId } } } }";
    pub const CAPTURE_TRANSACTION: &str = "mutation CaptureAuthorization($captureAuthorizationInput: CaptureAuthorizationInput!) { captureAuthorization(captureAuthorizationInput: $captureAuthorizationInput) { captureAuthorizationResponse { __typename ... on CaptureAuthorizationApproval { __typename paymentId transactionId } ... on CaptureAuthorizationDecline { __typename paymentId transactionId } } errors { ... on InternalServiceError { message processorResponseCode transactionId } ... on RuleInViolationError {  message processorResponseCode transactionId } ... on SyntaxOnNetworkResponseError {  message processorResponseCode transactionId } ... on TimeoutOnNetworkResponseError {  message processorResponseCode transactionId } ... on ValidationFailureError {  message processorResponseCode transactionId} ... on PriorPaymentNotFoundError { message processorResponseCode transactionId } } } }";
    pub const VOID_TRANSACTION: &str = "mutation ReverseTransaction($reverseTransactionInput: ReverseTransactionInput!) { reverseTransaction(reverseTransactionInput: $reverseTransactionInput) { errors {  ... on InternalServiceError {  message processorResponseCode transactionId } ... on RuleInViolationError {  message processorResponseCode transactionId } ... on SyntaxOnNetworkResponseError {  message processorResponseCode transactionId } ... on TimeoutOnNetworkResponseError {  message processorResponseCode transactionId } ... on ValidationFailureError {  message processorResponseCode transactionId } ... on PriorTransactionNotFoundError {  message processorResponseCode transactionId } } reverseTransactionResponse {  paymentId transactionId ... on ReverseTransactionApproval {  paymentId transactionId } ... on ReverseTransactionDecline {  message paymentId transactionId declineType } } } }";
    pub const REFUND_TRANSACTION: &str = "mutation RefundPreviousPayment($refundPreviousPaymentInput: RefundPreviousPaymentInput!) { refundPreviousPayment(refundPreviousPaymentInput: $refundPreviousPaymentInput) { errors {  ... on InternalServiceError {  message processorResponseCode transactionId } ... on RuleInViolationError { processorResponseCode message transactionId } ... on SyntaxOnNetworkResponseError { message processorResponseCode transactionId } ... on TimeoutOnNetworkResponseError {  processorResponseCode message transactionId } ... on ValidationFailureError {  message processorResponseCode transactionId } ... on PriorPaymentNotFoundError {  message processorResponseCode transactionId } } refundPreviousPaymentResponse { __typename ... on RefundPreviousPaymentApproval { __typename paymentId transactionId } ... on RefundPreviousPaymentDecline { __typename declineType message transactionId paymentId } } } }";
    pub const SYNC_TRANSACTION: &str = "query PaymentTransaction($paymentTransactionId: UUID!) { paymentTransaction(id: $paymentTransactionId) { __typename responseType reference id paymentId ... on AcceptedSale { __typename id processorResponseCode processorResponseMessage } ... on ApprovedAuthorization { __typename id processorResponseCode processorResponseMessage } ... on ApprovedCapture { __typename id processorResponseCode processorResponseMessage } ... on ApprovedReversal { __typename id processorResponseCode processorResponseMessage } ... on DeclinedAuthorization { __typename id processorResponseCode processorResponseMessage } ... on DeclinedCapture { __typename id processorResponseCode processorResponseMessage } ... on DeclinedReversal { __typename id processorResponseCode processorResponseMessage } ... on GenericPaymentTransaction { __typename id processorResponseCode processorResponseMessage } ... on Authorization { __typename id processorResponseCode processorResponseMessage } ... on Capture { __typename id processorResponseCode processorResponseMessage } ... on Reversal { __typename id processorResponseCode processorResponseMessage } ... on Sale { __typename id processorResponseCode processorResponseMessage } } }";
}

pub mod tesouro_constants {
    pub const MAX_PAYMENT_REFERENCE_ID_LENGTH: usize = 28;
}

#[derive(Debug, Clone, Serialize)]
pub struct GenericTesouroRequest<T> {
    query: String,
    variables: T,
}

pub type TesouroAuthorizeRequest = GenericTesouroRequest<TesouroAuthorizeInput>;
pub type TesouroCaptureRequest = GenericTesouroRequest<TesouroCaptureInput>;
pub type TesouroVoidRequest = GenericTesouroRequest<TesouroVoidInput>;
pub type TesouroRefundRequest = GenericTesouroRequest<TesouroRefundInput>;
pub type TesouroSyncRequest = GenericTesouroRequest<TesouroSyncInput>;

pub type TesouroAuthorizeResponse = TesouroApiResponse<TesouroAuthorizeResponseData>;
pub type TesouroCaptureResponse = TesouroApiResponse<TesouroCaptureResponseData>;
pub type TesouroVoidResponse = TesouroApiResponse<TesouroVoidResponseData>;
pub type TesouroRefundResponse = TesouroApiResponse<RefundTransactionResponseData>;
pub type TesouroSyncResponse = TesouroApiResponse<TesouroSyncResponseData>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TesouroApiResponse<T> {
    TesouroApiSuccessResponse(TesouroApiResponseData<T>),
    TesouroErrorResponse(TesouroApiErrorResponse),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroApiResponseData<T> {
    data: T,
}

pub struct TesouroRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for TesouroRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponseData<T> {
    #[serde(rename = "__typename")]
    pub type_name: Option<T>,
    pub payment_id: Option<String>,
    pub transaction_id: String,
    pub decline_type: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroAuthorizeResponseData {
    authorize_customer_initiated_transaction: AuthorizeCustomerInitiatedTransactionResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AuthorizeTransactionResponseType {
    AuthorizationApproval,
    AuthorizationDecline,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeCustomerInitiatedTransactionResponseData {
    authorization_response: Option<TransactionResponseData<AuthorizeTransactionResponseType>>,
    errors: Option<Vec<TesouroTransactionErrorResponseData>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroCaptureResponseData {
    capture_authorization: CaptureCustomerInitiatedTransactionResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureCustomerInitiatedTransactionResponseData {
    capture_authorization_response: Option<TransactionResponseData<CaptureTransactionResponseType>>,
    errors: Option<Vec<TesouroTransactionErrorResponseData>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CaptureTransactionResponseType {
    CaptureAuthorizationApproval,
    CaptureAuthorizationDecline,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroVoidResponseData {
    reverse_transaction: ReverseTransactionResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseTransactionResponseData {
    reverse_transaction_response: Option<TransactionResponseData<ReverseTransactionResponseType>>,
    errors: Option<Vec<TesouroTransactionErrorResponseData>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ReverseTransactionResponseType {
    ReverseTransactionApproval,
    ReverseTransactionDecline,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundTransactionResponseData {
    refund_previous_payment: TesouroRefundPreviousPaymentResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroRefundPreviousPaymentResponseData {
    refund_previous_payment_response:
        Option<TransactionResponseData<RefundTransactionResponseType>>,
    errors: Option<Vec<TesouroTransactionErrorResponseData>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RefundTransactionResponseType {
    RefundPreviousPaymentApproval,
    RefundPreviousPaymentDecline,
}

pub struct TesouroAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) acceptor_id: Secret<String>,
}

impl TesouroAuthType {
    fn get_acceptor_id(&self) -> Secret<String> {
        self.acceptor_id.clone()
    }
}

impl TryFrom<&ConnectorAuthType> for TesouroAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                api_secret,
                key1,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: api_secret.to_owned(),
                acceptor_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroApiErrorResponse {
    errors: Vec<TesouroApiErrorData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroApiErrorData {
    message: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TesouroAccessTokenRequest {
    grant_type: TesouroGrantType,
    client_id: Secret<String>,
    client_secret: Secret<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TesouroGrantType {
    ClientCredentials,
}

impl TryFrom<&RefreshTokenRouterData> for TesouroAccessTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = TesouroAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            grant_type: TesouroGrantType::ClientCredentials,
            client_id: auth.client_id,
            client_secret: auth.client_secret,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TesouroAccessTokenResponse {
    access_token: Secret<String>,
    token_type: String,
    expires_in: i64,
}

impl<F, T> TryFrom<ResponseRouterData<F, TesouroAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TesouroAccessTokenResponse, T, AccessToken>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroAuthorizeInput {
    pub authorize_customer_initiated_transaction_input: AuthorizeCustomerInitiatedTransactionInput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroCaptureInput {
    pub capture_authorization_input: CaptureAuthorizationInput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroVoidInput {
    pub reverse_transaction_input: ReverseTransactionInput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroRefundInput {
    pub refund_previous_payment_input: RefundPreviousPaymentInput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroSyncInput {
    pub payment_transaction_id: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TesouroAuthorizationIntent {
    FinalAuthorization,
    PreAuthorization,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TesouroChannel {
    Ecommerce,
    MailOrderTelephoneOrder,
    Retail,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TesouroAutomaticCapture {
    Never,
    OnApproval,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TesouroWalletType {
    ApplePay,
    GooglePay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TesouroPaymentMethodDetails {
    CardWithPanDetails(TesouroCardWithPanDetails),
    NetworkTokenPassThroughDetails(TesouroNetworkTokenPassThroughDetails),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroCardWithPanDetails {
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub account_number: cards::CardNumber,
    pub payment_entry_mode: TesouroPaymentEntryMode,
    pub security_code: TesouroSecurityCode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroNetworkTokenPassThroughDetails {
    pub cryptogram: Secret<String>,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub token_value: cards::CardNumber,
    pub wallet_type: TesouroWalletType,
    pub ecommerce_indicator: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TesouroPaymentEntryMode {
    PaymentMethodNotOnFile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TesouroSecurityCode {
    pub value: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionAmountDetails {
    pub total_amount: FloatMajorUnit,
    pub currency: enums::Currency,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillToAddress {
    pub address1: Option<Secret<String>>,
    pub address2: Option<Secret<String>>,
    pub address3: Option<Secret<String>>,
    pub city: Option<String>,
    pub country_code: Option<common_enums::CountryAlpha3>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub postal_code: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeCustomerInitiatedTransactionInput {
    pub acceptor_id: Secret<String>,
    pub transaction_reference: String,
    pub payment_method_details: TesouroPaymentMethodDetails,
    pub transaction_amount_details: TransactionAmountDetails,
    pub automatic_capture: TesouroAutomaticCapture,
    pub authorization_intent: TesouroAuthorizationIntent,
    pub bill_to_address: BillToAddress,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAuthorizationInput {
    pub acceptor_id: Secret<String>,
    pub payment_id: String,
    pub transaction_reference: String,
    pub transaction_amount_details: TransactionAmountDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseTransactionInput {
    pub acceptor_id: Secret<String>,
    pub transaction_id: String,
    pub transaction_reference: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundPreviousPaymentInput {
    pub acceptor_id: Secret<String>,
    pub payment_id: String,
    pub transaction_reference: String,
    pub transaction_amount_details: TransactionAmountDetails,
}

impl TryFrom<&Card> for TesouroPaymentMethodDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &Card) -> Result<Self, Self::Error> {
        let card_data = TesouroCardWithPanDetails {
            expiration_month: value.get_card_expiry_month_2_digit()?,
            expiration_year: value.get_expiry_year_4_digit(),
            account_number: value.card_number.clone(),
            payment_entry_mode: TesouroPaymentEntryMode::PaymentMethodNotOnFile,
            security_code: TesouroSecurityCode {
                value: value.card_cvc.clone(),
            },
        };

        Ok(Self::CardWithPanDetails(card_data))
    }
}

fn get_apple_pay_data(
    apple_pay_wallet_data: &ApplePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Result<ApplePayPredecryptData, error_stack::Report<errors::ConnectorError>> {
    if let Some(PaymentMethodToken::ApplePayDecrypt(decrypted_data)) = payment_method_token {
        return Ok(*decrypted_data.clone());
    }

    match &apple_pay_wallet_data.payment_data {
        common_types::payments::ApplePayPaymentData::Decrypted(decrypted_data) => {
            Ok(decrypted_data.clone())
        }
        common_types::payments::ApplePayPaymentData::Encrypted(_) => {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "decrypted apple pay data",
            })?
        }
    }
}

fn get_goole_pay_data(
    google_pay_wallet_data: &GooglePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Result<GPayPredecryptData, error_stack::Report<errors::ConnectorError>> {
    if let Some(PaymentMethodToken::GooglePayDecrypt(decrypted_data)) = payment_method_token {
        return Ok(*decrypted_data.clone());
    }

    match &google_pay_wallet_data.tokenization_data {
        common_types::payments::GpayTokenizationData::Decrypted(decrypted_data) => {
            Ok(decrypted_data.clone())
        }
        common_types::payments::GpayTokenizationData::Encrypted(_) => {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "decrypted google pay data",
            })?
        }
    }
}

impl TryFrom<(&ApplePayWalletData, Option<&PaymentMethodToken>)> for TesouroPaymentMethodDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (wallet_data, payment_method_token): (&ApplePayWalletData, Option<&PaymentMethodToken>),
    ) -> Result<Self, Self::Error> {
        let apple_pay_data = get_apple_pay_data(wallet_data, payment_method_token)?;

        let network_token_details = TesouroNetworkTokenPassThroughDetails {
            expiration_year: apple_pay_data.get_four_digit_expiry_year(),
            cryptogram: apple_pay_data.payment_data.online_payment_cryptogram,
            token_value: apple_pay_data.application_primary_account_number,
            expiration_month: apple_pay_data.application_expiration_month,
            ecommerce_indicator: apple_pay_data.payment_data.eci_indicator,
            wallet_type: TesouroWalletType::ApplePay,
        };

        Ok(Self::NetworkTokenPassThroughDetails(network_token_details))
    }
}

impl TryFrom<(&GooglePayWalletData, Option<&PaymentMethodToken>)> for TesouroPaymentMethodDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (wallet_data, payment_method_token): (&GooglePayWalletData, Option<&PaymentMethodToken>),
    ) -> Result<Self, Self::Error> {
        let google_pay_data = get_goole_pay_data(wallet_data, payment_method_token)?;

        let network_token_details = TesouroNetworkTokenPassThroughDetails {
            expiration_year: google_pay_data
                .get_four_digit_expiry_year()
                .change_context(errors::ConnectorError::InvalidWalletToken {
                    wallet_name: "Google Pay".to_string(),
                })?,
            cryptogram: google_pay_data.cryptogram.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "google pay data cryptogram",
                },
            )?,
            token_value: google_pay_data.application_primary_account_number,
            expiration_month: google_pay_data.card_exp_month,
            ecommerce_indicator: google_pay_data.eci_indicator,
            wallet_type: TesouroWalletType::GooglePay,
        };

        Ok(Self::NetworkTokenPassThroughDetails(network_token_details))
    }
}

pub struct TesouroCaptureData {
    automatic_capture: TesouroAutomaticCapture,
    authorization_intent: TesouroAuthorizationIntent,
}

impl From<bool> for TesouroCaptureData {
    fn from(is_auto_capture: bool) -> Self {
        if is_auto_capture {
            Self {
                automatic_capture: TesouroAutomaticCapture::OnApproval,
                authorization_intent: TesouroAuthorizationIntent::FinalAuthorization,
            }
        } else {
            Self {
                automatic_capture: TesouroAutomaticCapture::Never,
                authorization_intent: TesouroAuthorizationIntent::PreAuthorization,
            }
        }
    }
}

impl From<&PaymentsAuthorizeRouterData> for BillToAddress {
    fn from(router_data: &PaymentsAuthorizeRouterData) -> Self {
        Self {
            address1: router_data.get_optional_billing_line1(),
            address2: router_data.get_optional_billing_line2(),
            address3: router_data.get_optional_billing_line3(),
            city: router_data.get_optional_billing_city(),
            country_code: router_data
                .get_optional_billing_country()
                .map(|billing_country| {
                    common_enums::CountryAlpha2::from_alpha2_to_alpha3(billing_country)
                }),
            first_name: router_data.get_optional_billing_first_name(),
            last_name: router_data.get_optional_billing_last_name(),
            postal_code: router_data.get_optional_billing_zip(),
            state: router_data.get_optional_billing_state(),
        }
    }
}

impl TryFrom<&TesouroRouterData<&PaymentsAuthorizeRouterData>> for TesouroAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TesouroRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Cards 3DS".to_string(),
                connector: "Tesouro",
            })?
        }

        let auth = TesouroAuthType::try_from(&item.router_data.connector_auth_type)?;
        let acceptor_id = auth.get_acceptor_id();
        let transaction_reference =
            get_valid_transaction_id(item.router_data.connector_request_reference_id.clone())?;
        let capture_data = TesouroCaptureData::from(item.router_data.request.is_auto_capture()?);

        let payment_method_details = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(card) => TesouroPaymentMethodDetails::try_from(card),
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::ApplePay(apple_pay_wallet_data) => {
                    let payment_method_token = item.router_data.payment_method_token.as_ref();
                    TesouroPaymentMethodDetails::try_from((
                        apple_pay_wallet_data,
                        payment_method_token,
                    ))
                }
                WalletData::GooglePay(google_pay_wallet_data) => {
                    let payment_method_token = item.router_data.payment_method_token.as_ref();
                    TesouroPaymentMethodDetails::try_from((
                        google_pay_wallet_data,
                        payment_method_token,
                    ))
                }
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPay(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::Paysera(_)
                | WalletData::Skrill(_)
                | WalletData::BluecodeRedirect {}
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
                | WalletData::GooglePayRedirect(_)
                | WalletData::GooglePayThirdPartySdk(_)
                | WalletData::MbWayRedirect(_)
                | WalletData::MobilePayRedirect(_)
                | WalletData::PaypalSdk(_)
                | WalletData::PaypalRedirect(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::RevolutPay(_)
                | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                    connector_utils::get_unimplemented_payment_method_error_message("Tesouro"),
                ))?,
            },
            PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::MobilePayment(_) => Err(errors::ConnectorError::NotImplemented(
                connector_utils::get_unimplemented_payment_method_error_message("tesouro"),
            )
            .into()),
        }?;

        let bill_to_address = BillToAddress::from(item.router_data);

        Ok(Self {
            query: tesouro_queries::AUTHORIZE_TRANSACTION.to_string(),
            variables: TesouroAuthorizeInput {
                authorize_customer_initiated_transaction_input:
                    AuthorizeCustomerInitiatedTransactionInput {
                        acceptor_id,
                        transaction_reference,
                        payment_method_details,
                        transaction_amount_details: TransactionAmountDetails {
                            total_amount: item.amount,
                            currency: item.router_data.request.currency,
                        },
                        automatic_capture: capture_data.automatic_capture,
                        authorization_intent: capture_data.authorization_intent,
                        bill_to_address,
                    },
            },
        })
    }
}

impl TryFrom<&TesouroRouterData<&PaymentsCaptureRouterData>> for TesouroCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TesouroRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = TesouroAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payment_metadata = item
            .router_data
            .request
            .connector_meta
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<TesouroTransactionMetadata>(Some(
                    payment_metadata,
                ))
            })
            .transpose()?;

        let payment_id = payment_metadata
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?
            .payment_id;

        let transaction_id =
            get_valid_transaction_id(item.router_data.connector_request_reference_id.clone())?;

        Ok(Self {
            query: tesouro_queries::CAPTURE_TRANSACTION.to_string(),
            variables: TesouroCaptureInput {
                capture_authorization_input: CaptureAuthorizationInput {
                    acceptor_id: auth.get_acceptor_id(),
                    payment_id,
                    transaction_reference: format!("capture_{transaction_id}"),
                    transaction_amount_details: TransactionAmountDetails {
                        total_amount: item.amount,
                        currency: item.router_data.request.currency,
                    },
                },
            },
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for TesouroVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = TesouroAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = get_valid_transaction_id(item.connector_request_reference_id.clone())?;
        Ok(Self {
            query: tesouro_queries::VOID_TRANSACTION.to_string(),
            variables: TesouroVoidInput {
                reverse_transaction_input: ReverseTransactionInput {
                    acceptor_id: auth.get_acceptor_id(),
                    transaction_id: item.request.connector_transaction_id.clone(),
                    transaction_reference: format!("rev_{transaction_id}"),
                },
            },
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroApprovalResponse {
    pub payment_id: String,
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroDeclineResponse {
    pub payment_id: Option<String>,
    pub transaction_id: Option<String>,
    pub decline_type: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroTransactionErrorResponseData {
    pub message: String,
    pub processor_response_code: Option<String>,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroTransactionMetadata {
    pub payment_id: String,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            TesouroAuthorizeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            TesouroAuthorizeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                if let Some(authorization_response) = response
                    .data
                    .authorize_customer_initiated_transaction
                    .authorization_response
                {
                    let transaction_id = authorization_response.transaction_id;
                    let connector_metadata = serde_json::json!(TesouroTransactionMetadata {
                        payment_id: authorization_response
                            .payment_id
                            .clone()
                            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                    });
                    match authorization_response.type_name {
                        Some(AuthorizeTransactionResponseType::AuthorizationApproval) => Ok(Self {
                            status: if item.data.request.is_auto_capture()? {
                                enums::AttemptStatus::Charged
                            } else {
                                enums::AttemptStatus::Authorized
                            },
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: Some(connector_metadata),
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..item.data
                        }),
                        Some(AuthorizeTransactionResponseType::AuthorizationDecline) => Ok(Self {
                            status: if item.data.request.is_auto_capture()? {
                                enums::AttemptStatus::Failure
                            } else {
                                enums::AttemptStatus::AuthorizationFailed
                            },
                            response: Err(ErrorResponse {
                                code: authorization_response
                                    .decline_type
                                    .clone()
                                    .unwrap_or(NO_ERROR_CODE.to_string()),
                                message: authorization_response
                                    .message
                                    .clone()
                                    .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                                reason: authorization_response.message.clone(),
                                status_code: item.http_code,
                                attempt_status: None,
                                connector_transaction_id: Some(transaction_id.clone()),
                                network_advice_code: None,
                                network_decline_code: None,
                                network_error_message: None,
                                connector_metadata: None,
                            }),
                            ..item.data
                        }),
                        None => Ok(Self {
                            status: enums::AttemptStatus::Pending,
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: Some(connector_metadata),
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..item.data
                        }),
                    }
                } else if let Some(errors) = response
                    .data
                    .authorize_customer_initiated_transaction
                    .errors
                {
                    let error_response = errors.first();
                    let error_code = error_response
                        .as_ref()
                        .and_then(|error_data| error_data.processor_response_code.clone())
                        .unwrap_or(NO_ERROR_CODE.to_string());
                    let error_message = error_response
                        .as_ref()
                        .map(|error_data| error_data.message.clone());
                    let connector_transaction_id = error_response
                        .as_ref()
                        .and_then(|error_data| error_data.transaction_id.clone());

                    Ok(Self {
                        status: if item.data.request.is_auto_capture()? {
                            enums::AttemptStatus::Failure
                        } else {
                            enums::AttemptStatus::AuthorizationFailed
                        },
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message
                                .clone()
                                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                            reason: error_message.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Err(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from(
                            "Expected either error or authorization_response".to_string(),
                        ),
                    ))?
                }
            }
            TesouroAuthorizeResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    status: if item.data.request.is_auto_capture()? {
                        enums::AttemptStatus::Failure
                    } else {
                        enums::AttemptStatus::AuthorizationFailed
                    },
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroAccessTokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroGraphQlErrorResponse {
    pub errors: Vec<TesouroGraphQlError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroGraphQlError {
    pub message: String,
    pub extensions: Option<TesouroGraphQlErrorExtensions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TesouroGraphQlErrorExtensions {
    pub code: Option<String>,
    pub reason: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, TesouroCaptureResponse, PaymentsCaptureData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            TesouroCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                if let Some(capture_authorization_response) = response
                    .data
                    .capture_authorization
                    .capture_authorization_response
                {
                    let transaction_id = capture_authorization_response.transaction_id;
                    match capture_authorization_response.type_name {
                        Some(CaptureTransactionResponseType::CaptureAuthorizationApproval) => {
                            Ok(Self {
                                status: enums::AttemptStatus::Charged,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(transaction_id),
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
                        }
                        Some(CaptureTransactionResponseType::CaptureAuthorizationDecline) => {
                            Ok(Self {
                                status: enums::AttemptStatus::CaptureFailed,
                                response: Err(ErrorResponse {
                                    code: capture_authorization_response
                                        .decline_type
                                        .clone()
                                        .unwrap_or(NO_ERROR_CODE.to_string()),
                                    message: capture_authorization_response
                                        .message
                                        .clone()
                                        .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                                    reason: capture_authorization_response.message.clone(),
                                    status_code: item.http_code,
                                    attempt_status: None,
                                    connector_transaction_id: Some(transaction_id.clone()),
                                    network_advice_code: None,
                                    network_decline_code: None,
                                    network_error_message: None,
                                    connector_metadata: None,
                                }),
                                ..item.data
                            })
                        }
                        None => Ok(Self {
                            status: enums::AttemptStatus::CaptureInitiated,
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..item.data
                        }),
                    }
                } else if let Some(errors) = response.data.capture_authorization.errors {
                    let error_response = errors.first();
                    let error_code = error_response
                        .as_ref()
                        .and_then(|error_data| error_data.processor_response_code.clone())
                        .unwrap_or(NO_ERROR_CODE.to_string());
                    let error_message = error_response
                        .as_ref()
                        .map(|error_data| error_data.message.clone());

                    Ok(Self {
                        status: enums::AttemptStatus::CaptureFailed,
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message
                                .clone()
                                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                            reason: error_message.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Err(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from(
                            "Expected either error or capture_authorization_response".to_string(),
                        ),
                    ))?
                }
            }
            TesouroApiResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    status: enums::AttemptStatus::CaptureFailed,
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, TesouroVoidResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TesouroVoidResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                if let Some(reverse_transaction) = response
                    .data
                    .reverse_transaction
                    .reverse_transaction_response
                {
                    let transaction_id = reverse_transaction.transaction_id;
                    match reverse_transaction.type_name {
                        Some(ReverseTransactionResponseType::ReverseTransactionApproval) => {
                            Ok(Self {
                                status: enums::AttemptStatus::Voided,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(transaction_id),
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
                        }
                        Some(ReverseTransactionResponseType::ReverseTransactionDecline) => {
                            Ok(Self {
                                status: enums::AttemptStatus::VoidFailed,
                                response: Err(ErrorResponse {
                                    code: reverse_transaction
                                        .decline_type
                                        .clone()
                                        .unwrap_or(NO_ERROR_CODE.to_string()),
                                    message: reverse_transaction
                                        .message
                                        .clone()
                                        .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                                    reason: reverse_transaction.message.clone(),
                                    status_code: item.http_code,
                                    attempt_status: None,
                                    connector_transaction_id: Some(transaction_id.clone()),
                                    network_advice_code: None,
                                    network_decline_code: None,
                                    network_error_message: None,
                                    connector_metadata: None,
                                }),
                                ..item.data
                            })
                        }
                        None => Ok(Self {
                            status: enums::AttemptStatus::VoidInitiated,
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..item.data
                        }),
                    }
                } else if let Some(errors) = response.data.reverse_transaction.errors {
                    let error_response = errors.first();
                    let error_code = error_response
                        .as_ref()
                        .and_then(|error_data| error_data.processor_response_code.clone())
                        .unwrap_or(NO_ERROR_CODE.to_string());
                    let error_message = error_response
                        .as_ref()
                        .map(|error_data| error_data.message.clone());

                    Ok(Self {
                        status: enums::AttemptStatus::VoidFailed,
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message
                                .clone()
                                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                            reason: error_message.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Err(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from(
                            "Expected either error or reverse_transaction_response".to_string(),
                        ),
                    ))?
                }
            }
            TesouroApiResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    status: enums::AttemptStatus::VoidFailed,
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F> TryFrom<&TesouroRouterData<&RefundsRouterData<F>>> for TesouroRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TesouroRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = TesouroAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payment_metadata = item
            .router_data
            .request
            .connector_metadata
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<TesouroTransactionMetadata>(Some(
                    payment_metadata,
                ))
            })
            .transpose()?;

        let payment_id = payment_metadata
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?
            .payment_id;

        let transaction_reference =
            get_valid_transaction_id(item.router_data.connector_request_reference_id.clone())?;

        Ok(Self {
            query: tesouro_queries::REFUND_TRANSACTION.to_string(),
            variables: TesouroRefundInput {
                refund_previous_payment_input: RefundPreviousPaymentInput {
                    acceptor_id: auth.get_acceptor_id(),
                    payment_id,
                    transaction_reference,
                    transaction_amount_details: TransactionAmountDetails {
                        total_amount: item.amount,
                        currency: item.router_data.request.currency,
                    },
                },
            },
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, TesouroRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, TesouroRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                if let Some(refund_previous_payment_response) = response
                    .data
                    .refund_previous_payment
                    .refund_previous_payment_response
                {
                    let transaction_id = refund_previous_payment_response.transaction_id;
                    match refund_previous_payment_response.type_name {
                        Some(RefundTransactionResponseType::RefundPreviousPaymentApproval) => {
                            Ok(Self {
                                response: Ok(RefundsResponseData {
                                    connector_refund_id: transaction_id,
                                    refund_status: enums::RefundStatus::Success,
                                }),
                                ..item.data
                            })
                        }
                        Some(RefundTransactionResponseType::RefundPreviousPaymentDecline) => {
                            Ok(Self {
                                response: Err(ErrorResponse {
                                    code: refund_previous_payment_response
                                        .decline_type
                                        .clone()
                                        .unwrap_or(NO_ERROR_CODE.to_string()),
                                    message: refund_previous_payment_response
                                        .message
                                        .clone()
                                        .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                                    reason: refund_previous_payment_response.message.clone(),
                                    status_code: item.http_code,
                                    attempt_status: None,
                                    connector_transaction_id: None,
                                    network_advice_code: None,
                                    network_decline_code: None,
                                    network_error_message: None,
                                    connector_metadata: None,
                                }),
                                ..item.data
                            })
                        }
                        None => Ok(Self {
                            response: Ok(RefundsResponseData {
                                connector_refund_id: transaction_id,
                                refund_status: enums::RefundStatus::Pending,
                            }),
                            ..item.data
                        }),
                    }
                } else if let Some(errors) = response.data.refund_previous_payment.errors {
                    let error_response = errors.first();
                    let error_code = error_response
                        .as_ref()
                        .and_then(|error_data| error_data.processor_response_code.clone())
                        .unwrap_or(NO_ERROR_CODE.to_string());
                    let error_message = error_response
                        .as_ref()
                        .map(|error_data| error_data.message.clone());

                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message
                                .clone()
                                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                            reason: error_message.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Err(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from(
                            "Expected either error or refund_previous_payment_response".to_string(),
                        ),
                    ))?
                }
            }
            TesouroApiResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroSyncResponseData {
    payment_transaction: TesouroPaymentTransactionResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesouroPaymentTransactionResponse {
    pub id: String,
    #[serde(rename = "__typename")]
    pub typename: TesouroSyncStatus,
    #[serde(rename = "processorResponseCode")]
    pub processor_response_code: Option<String>,
    #[serde(rename = "processorResponseMessage")]
    pub processor_response_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TesouroSyncStatus {
    AcceptedSale,
    ApprovedAuthorization,
    ApprovedCapture,
    ApprovedReversal,
    DeclinedAuthorization,
    DeclinedCapture,
    DeclinedReversal,
    GenericPaymentTransaction,
    Authorization,
    Capture,
    Reversal,
    ApprovedRefund,
    ApprovedRefundAuthorization,
    DeclinedRefund,
    DeclinedRefundAuthorization,
    Refund,
    RefundAuthorization,
}

fn get_payment_attempt_status(
    status: TesouroSyncStatus,
    is_auto_capture: bool,
    previous_status: enums::AttemptStatus,
) -> Result<enums::AttemptStatus, errors::ConnectorError> {
    match status {
        TesouroSyncStatus::AcceptedSale | TesouroSyncStatus::ApprovedCapture => {
            Ok(enums::AttemptStatus::Charged)
        }
        TesouroSyncStatus::ApprovedAuthorization => {
            if is_auto_capture {
                Ok(enums::AttemptStatus::Charged)
            } else {
                Ok(enums::AttemptStatus::Authorized)
            }
        }
        TesouroSyncStatus::DeclinedAuthorization => {
            if is_auto_capture {
                Ok(enums::AttemptStatus::AuthorizationFailed)
            } else {
                Ok(enums::AttemptStatus::Failure)
            }
        }
        TesouroSyncStatus::ApprovedReversal => Ok(enums::AttemptStatus::Voided),
        TesouroSyncStatus::DeclinedCapture => Ok(enums::AttemptStatus::Failure),
        TesouroSyncStatus::DeclinedReversal => Ok(enums::AttemptStatus::VoidFailed),
        TesouroSyncStatus::GenericPaymentTransaction => Ok(previous_status),
        TesouroSyncStatus::Authorization => Ok(enums::AttemptStatus::Authorizing),
        TesouroSyncStatus::Capture => Ok(enums::AttemptStatus::CaptureInitiated),
        TesouroSyncStatus::Reversal => Ok(enums::AttemptStatus::VoidInitiated),
        TesouroSyncStatus::ApprovedRefund
        | TesouroSyncStatus::ApprovedRefundAuthorization
        | TesouroSyncStatus::DeclinedRefund
        | TesouroSyncStatus::DeclinedRefundAuthorization
        | TesouroSyncStatus::Refund
        | TesouroSyncStatus::RefundAuthorization => {
            Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Invalid Status Recieved".to_string()),
            ))
        }
    }
}

impl TryFrom<&PaymentsSyncRouterData> for TesouroSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            query: tesouro_queries::SYNC_TRANSACTION.to_string(),
            variables: TesouroSyncInput {
                payment_transaction_id: item
                    .request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            },
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, TesouroSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TesouroSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                let status = get_payment_attempt_status(
                    response.data.payment_transaction.typename,
                    item.data.request.is_auto_capture()?,
                    item.data.status,
                )?;
                if connector_utils::is_payment_failure(status) {
                    let error_code = response
                        .data
                        .payment_transaction
                        .processor_response_code
                        .unwrap_or(NO_ERROR_CODE.to_string());

                    let error_message = response
                        .data
                        .payment_transaction
                        .processor_response_message
                        .unwrap_or(NO_ERROR_CODE.to_string());

                    let connector_transaction_id = response.data.payment_transaction.id.clone();

                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message.clone(),
                            reason: Some(error_message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(connector_transaction_id),
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                response.data.payment_transaction.id.to_string(),
                            ),
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
                }
            }
            TesouroApiResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    status: item.data.status,
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&RefundSyncRouterData> for TesouroSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            query: tesouro_queries::SYNC_TRANSACTION.to_string(),
            variables: TesouroSyncInput {
                payment_transaction_id: item.request.get_connector_refund_id()?,
            },
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, TesouroSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, TesouroSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TesouroApiResponse::TesouroApiSuccessResponse(response) => {
                let status = match response.data.payment_transaction.typename {
                    TesouroSyncStatus::ApprovedRefund => enums::RefundStatus::Success,
                    TesouroSyncStatus::DeclinedRefund
                    | TesouroSyncStatus::DeclinedRefundAuthorization => {
                        enums::RefundStatus::Failure
                    }
                    TesouroSyncStatus::GenericPaymentTransaction
                    | TesouroSyncStatus::Refund
                    | TesouroSyncStatus::RefundAuthorization
                    | TesouroSyncStatus::ApprovedRefundAuthorization => {
                        enums::RefundStatus::Pending
                    }
                    _ => {
                        return Err(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from("Invalid Status Recieved".to_string()),
                        )
                        .into())
                    }
                };

                if connector_utils::is_refund_failure(status) {
                    let error_code = response
                        .data
                        .payment_transaction
                        .processor_response_code
                        .unwrap_or(NO_ERROR_CODE.to_string());

                    let error_message = response
                        .data
                        .payment_transaction
                        .processor_response_message
                        .unwrap_or(NO_ERROR_CODE.to_string());

                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: error_code.clone(),
                            message: error_message.clone(),
                            reason: Some(error_message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        response: Ok(RefundsResponseData {
                            connector_refund_id: response.data.payment_transaction.id,
                            refund_status: enums::RefundStatus::Success,
                        }),
                        ..item.data
                    })
                }
            }
            TesouroApiResponse::TesouroErrorResponse(error_response) => {
                let message = error_response
                    .errors
                    .iter()
                    .map(|error| error.message.to_string())
                    .collect::<Vec<String>>();

                let error_message = match !message.is_empty() {
                    true => Some(message.join(" ")),
                    false => None,
                };
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: NO_ERROR_CODE.to_string(),
                        message: error_message
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

fn get_valid_transaction_id(
    id: String,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    if id.len() <= tesouro_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH {
        Ok(id.clone())
    } else {
        Err(errors::ConnectorError::MaxFieldLengthViolated {
            connector: "Tesouro".to_string(),
            field_name: "transaction_reference".to_string(),
            max_length: tesouro_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH,
            received_length: id.len(),
        }
        .into())
    }
}
