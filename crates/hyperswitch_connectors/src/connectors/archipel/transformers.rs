use bytes::Bytes;
use common_enums::{
    self, AttemptStatus, AuthorizationStatus, CaptureMethod, Currency, FutureUsage,
    PaymentMethodStatus, RefundStatus,
};
use common_utils::{date_time, ext_traits::Encode, pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::AddressDetails,
    payment_method_data::{Card, PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        AuthenticationData, PaymentsIncrementalAuthorizationData, ResponseId,
        SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsIncrementalAuthorizationRouterData, PaymentsSyncRouterData, RefundsRouterData,
        SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsResponseRouterData, PaymentsSyncResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    unimplemented_payment_method,
    utils::{
        self, AddressData, AddressDetailsData, CardData, CardIssuer, PaymentsAuthorizeRequestData,
        RouterData as _,
    },
};

const THREE_DS_MAX_SUPPORTED_VERSION: &str = "2.2.0";

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct ArchipelTenantId(pub String);

impl From<String> for ArchipelTenantId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub struct ArchipelRouterData<T> {
    pub amount: MinorUnit,
    pub tenant_id: ArchipelTenantId,
    pub router_data: T,
}

impl<T> From<(MinorUnit, ArchipelTenantId, T)> for ArchipelRouterData<T> {
    fn from((amount, tenant_id, router_data): (MinorUnit, ArchipelTenantId, T)) -> Self {
        Self {
            amount,
            tenant_id,
            router_data,
        }
    }
}

pub struct ArchipelAuthType {
    pub(super) ca_certificate: Option<Secret<String>>,
}

impl TryFrom<&ConnectorAuthType> for ArchipelAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                ca_certificate: Some(api_key.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ArchipelConfigData {
    pub tenant_id: ArchipelTenantId,
    pub platform_url: String,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for ArchipelConfigData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(connector_metadata: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let config_data = utils::to_connector_meta_from_secret::<Self>(connector_metadata.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata. Required fields: tenant_id, platform_url",
            })?;
        Ok(config_data)
    }
}

#[derive(Debug, Default, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArchipelPaymentInitiator {
    #[default]
    Customer,
    Merchant,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArchipelWalletProvider {
    ApplePay,
    GooglePay,
    SamsungPay,
}

#[derive(Debug, Default, Serialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArchipelPaymentCertainty {
    #[default]
    Final,
    Estimated,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelOrderRequest {
    amount: MinorUnit,
    currency: String,
    certainty: ArchipelPaymentCertainty,
    initiator: ArchipelPaymentInitiator,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct CardExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Debug, Serialize, Default, Eq, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplicationSelectionIndicator {
    #[default]
    ByDefault,
    CustomerChoice,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Archipel3DS {
    #[serde(rename = "acsTransID")]
    acs_trans_id: Option<Secret<String>>,
    #[serde(rename = "dsTransID")]
    ds_trans_id: Option<Secret<String>>,
    #[serde(rename = "3DSRequestorName")]
    three_ds_requestor_name: Option<Secret<String>>,
    #[serde(rename = "3DSAuthDate")]
    three_ds_auth_date: Option<String>,
    #[serde(rename = "3DSAuthAmt")]
    three_ds_auth_amt: Option<MinorUnit>,
    #[serde(rename = "3DSAuthStatus")]
    three_ds_auth_status: Option<String>,
    #[serde(rename = "3DSMaxSupportedVersion")]
    three_ds_max_supported_version: String,
    #[serde(rename = "3DSVersion")]
    three_ds_version: Option<common_utils::types::SemanticVersion>,
    authentication_value: Secret<String>,
    authentication_method: Option<Secret<String>>,
    eci: Option<String>,
}

impl From<AuthenticationData> for Archipel3DS {
    fn from(three_ds_data: AuthenticationData) -> Self {
        let now = date_time::date_as_yyyymmddthhmmssmmmz().ok();
        Self {
            acs_trans_id: None,
            ds_trans_id: three_ds_data.ds_trans_id.map(Secret::new),
            three_ds_requestor_name: None,
            three_ds_auth_date: now,
            three_ds_auth_amt: None,
            three_ds_auth_status: None,
            three_ds_max_supported_version: THREE_DS_MAX_SUPPORTED_VERSION.into(),
            three_ds_version: three_ds_data.message_version,
            authentication_value: three_ds_data.cavv,
            authentication_method: None,
            eci: three_ds_data.eci,
        }
    }
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelCardHolder {
    billing_address: Option<ArchipelBillingAddress>,
}

impl From<Option<ArchipelBillingAddress>> for ArchipelCardHolder {
    fn from(value: Option<ArchipelBillingAddress>) -> Self {
        Self {
            billing_address: value,
        }
    }
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelBillingAddress {
    address: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
}

pub trait ToArchipelBillingAddress {
    fn to_archipel_billing_address(&self) -> Option<ArchipelBillingAddress>;
}

impl ToArchipelBillingAddress for AddressDetails {
    fn to_archipel_billing_address(&self) -> Option<ArchipelBillingAddress> {
        let address = self.get_combined_address_line().ok();
        let postal_code = self.get_optional_zip();

        match (address, postal_code) {
            (None, None) => None,
            (addr, zip) => Some(ArchipelBillingAddress {
                address: addr,
                postal_code: zip,
            }),
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArchipelCredentialIndicatorStatus {
    Initial,
    Subsequent,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelCredentialIndicator {
    status: ArchipelCredentialIndicatorStatus,
    recurring: Option<bool>,
    transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardData {
    card_data: ArchipelTokenizedCard,
    wallet_information: ArchipelWalletInformation,
}

impl TryFrom<(&WalletData, &Option<PaymentMethodToken>)> for TokenizedCardData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (wallet_data, pm_token): (&WalletData, &Option<PaymentMethodToken>),
    ) -> Result<Self, Self::Error> {
        let WalletData::ApplePay(apple_pay_data) = wallet_data else {
            return Err(error_stack::Report::from(
                errors::ConnectorError::NotSupported {
                    message: "Wallet type used".to_string(),
                    connector: "Archipel",
                },
            ));
        };

        let Some(PaymentMethodToken::ApplePayDecrypt(apple_pay_decrypt_data)) = pm_token else {
            return Err(error_stack::Report::from(unimplemented_payment_method!(
                "Apple Pay",
                "Manual",
                "Archipel"
            )));
        };

        let card_number = apple_pay_decrypt_data
            .application_primary_account_number
            .clone();

        let expiry_year_2_digit = apple_pay_decrypt_data
            .get_two_digit_expiry_year()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "Apple pay expiry year",
            })?;
        let expiry_month = apple_pay_decrypt_data.get_expiry_month().change_context(
            errors::ConnectorError::InvalidDataFormat {
                field_name: "expiration_month",
            },
        )?;

        Ok(Self {
            card_data: ArchipelTokenizedCard {
                expiry: CardExpiryDate {
                    year: expiry_year_2_digit,
                    month: expiry_month,
                },
                number: card_number,
                scheme: ArchipelCardScheme::from(apple_pay_data.payment_method.network.as_str()),
            },
            wallet_information: {
                ArchipelWalletInformation {
                    wallet_provider: ArchipelWalletProvider::ApplePay,
                    wallet_indicator: apple_pay_decrypt_data.payment_data.eci_indicator.clone(),
                    wallet_cryptogram: apple_pay_decrypt_data
                        .payment_data
                        .online_payment_cryptogram
                        .clone(),
                }
            },
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelTokenizedCard {
    number: cards::CardNumber,
    expiry: CardExpiryDate,
    scheme: ArchipelCardScheme,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelCard {
    number: cards::CardNumber,
    expiry: CardExpiryDate,
    security_code: Option<Secret<String>>,
    card_holder_name: Option<Secret<String>>,
    application_selection_indicator: ApplicationSelectionIndicator,
    scheme: ArchipelCardScheme,
}

impl TryFrom<(Option<Secret<String>>, Option<ArchipelCardHolder>, &Card)> for ArchipelCard {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card_holder_name, card_holder_billing, ccard): (
            Option<Secret<String>>,
            Option<ArchipelCardHolder>,
            &Card,
        ),
    ) -> Result<Self, Self::Error> {
        // NOTE: Archipel does not accept `card.card_holder_name` field without `cardholder` field.
        // So if `card_holder` is None, `card.card_holder_name` must also be None.
        // However, the reverse is allowed — the `cardholder` field can exist without `card.card_holder_name`.
        let card_holder_name = card_holder_billing
            .as_ref()
            .and_then(|_| ccard.card_holder_name.clone().or(card_holder_name.clone()));

        let scheme: ArchipelCardScheme = ccard.get_card_issuer().ok().into();
        Ok(Self {
            number: ccard.card_number.clone(),
            expiry: CardExpiryDate {
                month: ccard.card_exp_month.clone(),
                year: ccard.get_card_expiry_year_2_digit()?,
            },
            security_code: Some(ccard.card_cvc.clone()),
            application_selection_indicator: ApplicationSelectionIndicator::ByDefault,
            card_holder_name,
            scheme,
        })
    }
}

impl
    TryFrom<(
        Option<Secret<String>>,
        Option<ArchipelCardHolder>,
        &hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
    )> for ArchipelCard
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card_holder_name, card_holder_billing, card_details): (
            Option<Secret<String>>,
            Option<ArchipelCardHolder>,
            &hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
        ),
    ) -> Result<Self, Self::Error> {
        // NOTE: Archipel does not accept `card.card_holder_name` field without `cardholder` field.
        // So if `card_holder` is None, `card.card_holder_name` must also be None.
        // However, the reverse is allowed — the `cardholder` field can exist without `card.card_holder_name`.
        let card_holder_name = card_holder_billing.as_ref().and_then(|_| {
            card_details
                .card_holder_name
                .clone()
                .or(card_holder_name.clone())
        });

        let scheme: ArchipelCardScheme = card_details.get_card_issuer().ok().into();
        Ok(Self {
            number: card_details.card_number.clone(),
            expiry: CardExpiryDate {
                month: card_details.card_exp_month.clone(),
                year: card_details.get_card_expiry_year_2_digit()?,
            },
            security_code: None,
            application_selection_indicator: ApplicationSelectionIndicator::ByDefault,
            card_holder_name,
            scheme,
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelWalletInformation {
    wallet_indicator: Option<String>,
    wallet_provider: ArchipelWalletProvider,
    wallet_cryptogram: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelPaymentInformation {
    order: ArchipelOrderRequest,
    cardholder: Option<ArchipelCardHolder>,
    card_holder_name: Option<Secret<String>>,
    credential_indicator: Option<ArchipelCredentialIndicator>,
    stored_on_file: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelWalletAuthorizationRequest {
    order: ArchipelOrderRequest,
    card: ArchipelTokenizedCard,
    cardholder: Option<ArchipelCardHolder>,
    wallet: ArchipelWalletInformation,
    #[serde(rename = "3DS")]
    three_ds: Option<Archipel3DS>,
    credential_indicator: Option<ArchipelCredentialIndicator>,
    stored_on_file: bool,
    tenant_id: ArchipelTenantId,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelCardAuthorizationRequest {
    order: ArchipelOrderRequest,
    card: ArchipelCard,
    cardholder: Option<ArchipelCardHolder>,
    #[serde(rename = "3DS")]
    three_ds: Option<Archipel3DS>,
    credential_indicator: Option<ArchipelCredentialIndicator>,
    stored_on_file: bool,
    tenant_id: ArchipelTenantId,
}

// PaymentsResponse

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArchipelCardScheme {
    Amex,
    Mastercard,
    Visa,
    Discover,
    Diners,
    Unknown,
}

impl From<&str> for ArchipelCardScheme {
    fn from(input: &str) -> Self {
        match input {
            "Visa" => Self::Visa,
            "Amex" => Self::Amex,
            "Diners" => Self::Diners,
            "MasterCard" => Self::Mastercard,
            "Discover" => Self::Discover,
            _ => Self::Unknown,
        }
    }
}

impl From<Option<CardIssuer>> for ArchipelCardScheme {
    fn from(card_issuer: Option<CardIssuer>) -> Self {
        match card_issuer {
            Some(CardIssuer::Visa) => Self::Visa,
            Some(CardIssuer::Master | CardIssuer::Maestro) => Self::Mastercard,
            Some(CardIssuer::AmericanExpress) => Self::Amex,
            Some(CardIssuer::Discover) => Self::Discover,
            Some(CardIssuer::DinersClub) => Self::Diners,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArchipelPaymentStatus {
    #[default]
    Succeeded,
    Failed,
}

impl TryFrom<(AttemptStatus, CaptureMethod)> for ArchipelPaymentFlow {
    type Error = errors::ConnectorError;

    fn try_from(
        (status, capture_method): (AttemptStatus, CaptureMethod),
    ) -> Result<Self, Self::Error> {
        let is_auto_capture = matches!(capture_method, CaptureMethod::Automatic);

        match status {
            AttemptStatus::AuthenticationFailed => Ok(Self::Verify),
            AttemptStatus::Authorizing
            | AttemptStatus::Authorized
            | AttemptStatus::AuthorizationFailed => Ok(Self::Authorize),
            AttemptStatus::Voided | AttemptStatus::VoidInitiated | AttemptStatus::VoidFailed => {
                Ok(Self::Cancel)
            }
            AttemptStatus::CaptureInitiated | AttemptStatus::CaptureFailed => {
                if is_auto_capture {
                    Ok(Self::Pay)
                } else {
                    Ok(Self::Capture)
                }
            }
            AttemptStatus::PaymentMethodAwaited | AttemptStatus::ConfirmationAwaited => {
                if is_auto_capture {
                    Ok(Self::Pay)
                } else {
                    Ok(Self::Authorize)
                }
            }
            _ => Err(errors::ConnectorError::ProcessingStepFailed(Some(
                Bytes::from_static(
                    "Impossible to determine Archipel flow from AttemptStatus".as_bytes(),
                ),
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchipelPaymentFlow {
    Verify,
    Authorize,
    Pay,
    Capture,
    Cancel,
}

struct ArchipelFlowStatus {
    status: ArchipelPaymentStatus,
    flow: ArchipelPaymentFlow,
}
impl ArchipelFlowStatus {
    fn new(status: ArchipelPaymentStatus, flow: ArchipelPaymentFlow) -> Self {
        Self { status, flow }
    }
}

impl From<ArchipelFlowStatus> for AttemptStatus {
    fn from(ArchipelFlowStatus { status, flow }: ArchipelFlowStatus) -> Self {
        match status {
            ArchipelPaymentStatus::Succeeded => match flow {
                ArchipelPaymentFlow::Authorize => Self::Authorized,
                ArchipelPaymentFlow::Pay
                | ArchipelPaymentFlow::Verify
                | ArchipelPaymentFlow::Capture => Self::Charged,
                ArchipelPaymentFlow::Cancel => Self::Voided,
            },
            ArchipelPaymentStatus::Failed => match flow {
                ArchipelPaymentFlow::Authorize | ArchipelPaymentFlow::Pay => {
                    Self::AuthorizationFailed
                }
                ArchipelPaymentFlow::Verify => Self::AuthenticationFailed,
                ArchipelPaymentFlow::Capture => Self::CaptureFailed,
                ArchipelPaymentFlow::Cancel => Self::VoidFailed,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelOrderResponse {
    id: String,
    amount: Option<i64>,
    currency: Option<Currency>,
    captured_amount: Option<i64>,
    authorized_amount: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchipelErrorMessage {
    pub code: String,
    pub description: Option<String>,
}

impl Default for ArchipelErrorMessage {
    fn default() -> Self {
        Self {
            code: consts::NO_ERROR_CODE.to_string(),
            description: Some(consts::NO_ERROR_MESSAGE.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ArchipelErrorMessageWithHttpCode {
    error_message: ArchipelErrorMessage,
    http_code: u16,
}
impl ArchipelErrorMessageWithHttpCode {
    fn new(error_message: ArchipelErrorMessage, http_code: u16) -> Self {
        Self {
            error_message,
            http_code,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelTransactionMetadata {
    pub transaction_id: String,
    pub transaction_date: String,
    pub financial_network_code: Option<String>,
    pub issuer_transaction_id: Option<String>,
    pub response_code: Option<String>,
    pub authorization_code: Option<String>,
    pub payment_account_reference: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelPaymentsResponse {
    order: ArchipelOrderResponse,
    transaction_id: String,
    transaction_date: String,
    transaction_result: ArchipelPaymentStatus,
    error: Option<ArchipelErrorMessage>,
    financial_network_code: Option<String>,
    issuer_transaction_id: Option<String>,
    response_code: Option<String>,
    authorization_code: Option<String>,
    payment_account_reference: Option<Secret<String>>,
}

impl From<&ArchipelPaymentsResponse> for ArchipelTransactionMetadata {
    fn from(payment_response: &ArchipelPaymentsResponse) -> Self {
        Self {
            transaction_id: payment_response.transaction_id.clone(),
            transaction_date: payment_response.transaction_date.clone(),
            financial_network_code: payment_response.financial_network_code.clone(),
            issuer_transaction_id: payment_response.issuer_transaction_id.clone(),
            response_code: payment_response.response_code.clone(),
            authorization_code: payment_response.authorization_code.clone(),
            payment_account_reference: payment_response.payment_account_reference.clone(),
        }
    }
}

// AUTHORIZATION FLOW
impl TryFrom<(MinorUnit, &PaymentsAuthorizeRouterData)> for ArchipelPaymentInformation {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (amount, router_data): (MinorUnit, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        let is_recurring_payment = router_data
            .request
            .mandate_id
            .as_ref()
            .and_then(|mandate_ids| mandate_ids.mandate_id.as_ref())
            .is_some();

        let is_subsequent_trx = router_data
            .request
            .mandate_id
            .as_ref()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id.as_ref())
            .is_some();

        let is_saved_card_payment = (router_data.request.is_mandate_payment())
            | (router_data.request.setup_future_usage == Some(FutureUsage::OnSession))
            | (router_data.payment_method_status == Some(PaymentMethodStatus::Active));

        let certainty = if router_data.request.request_incremental_authorization {
            if is_recurring_payment {
                ArchipelPaymentCertainty::Final
            } else {
                ArchipelPaymentCertainty::Estimated
            }
        } else {
            ArchipelPaymentCertainty::Final
        };

        let transaction_initiator = if is_recurring_payment {
            ArchipelPaymentInitiator::Merchant
        } else {
            ArchipelPaymentInitiator::Customer
        };

        let order = ArchipelOrderRequest {
            amount,
            currency: router_data.request.currency.to_string(),
            certainty,
            initiator: transaction_initiator.clone(),
        };

        let cardholder = router_data
            .get_billing_address()
            .ok()
            .and_then(|address| address.to_archipel_billing_address())
            .map(|billing_address| ArchipelCardHolder {
                billing_address: Some(billing_address),
            });

        // NOTE: Archipel does not accept `card.card_holder_name` field without `cardholder` field.
        // So if `card_holder` is None, `card.card_holder_name` must also be None.
        // However, the reverse is allowed — the `cardholder` field can exist without `card.card_holder_name`.
        let card_holder_name = cardholder.as_ref().and_then(|_| {
            router_data
                .get_billing()
                .ok()
                .and_then(|billing| billing.get_optional_full_name())
        });

        let indicator_status = if is_subsequent_trx {
            ArchipelCredentialIndicatorStatus::Subsequent
        } else {
            ArchipelCredentialIndicatorStatus::Initial
        };

        let stored_on_file =
            is_saved_card_payment | router_data.request.is_customer_initiated_mandate_payment();

        let credential_indicator = stored_on_file.then(|| ArchipelCredentialIndicator {
            status: indicator_status.clone(),
            recurring: Some(is_recurring_payment),
            transaction_id: match indicator_status {
                ArchipelCredentialIndicatorStatus::Initial => None,
                ArchipelCredentialIndicatorStatus::Subsequent => {
                    router_data.request.get_optional_network_transaction_id()
                }
            },
        });

        Ok(Self {
            order,
            cardholder,
            card_holder_name,
            credential_indicator,
            stored_on_file,
        })
    }
}

impl TryFrom<ArchipelRouterData<&PaymentsAuthorizeRouterData>>
    for ArchipelCardAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ArchipelRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ArchipelRouterData {
            amount,
            tenant_id,
            router_data,
        } = item;

        let payment_information: ArchipelPaymentInformation =
            ArchipelPaymentInformation::try_from((amount, router_data))?;
        let payment_method_data = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ccard) => ArchipelCard::try_from((
                payment_information.card_holder_name,
                payment_information.cardholder.clone(),
                ccard,
            ))?,
            PaymentMethodData::CardDetailsForNetworkTransactionId(card_details) => {
                ArchipelCard::try_from((
                    payment_information.card_holder_name,
                    payment_information.cardholder.clone(),
                    card_details,
                ))?
            }
            PaymentMethodData::CardRedirect(..)
            | PaymentMethodData::Wallet(..)
            | PaymentMethodData::PayLater(..)
            | PaymentMethodData::BankRedirect(..)
            | PaymentMethodData::BankDebit(..)
            | PaymentMethodData::BankTransfer(..)
            | PaymentMethodData::Crypto(..)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(..)
            | PaymentMethodData::Upi(..)
            | PaymentMethodData::Voucher(..)
            | PaymentMethodData::GiftCard(..)
            | PaymentMethodData::CardToken(..)
            | PaymentMethodData::OpenBanking(..)
            | PaymentMethodData::NetworkToken(..)
            | PaymentMethodData::MobilePayment(..) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Archipel"),
            ))?,
        };

        let three_ds: Option<Archipel3DS> = if item.router_data.is_three_ds() {
            let auth_data = item
                .router_data
                .request
                .get_authentication_data()
                .change_context(errors::ConnectorError::NotSupported {
                    message: "Selected 3DS authentication method".to_string(),
                    connector: "archipel",
                })?;
            Some(Archipel3DS::from(auth_data))
        } else {
            None
        };

        Ok(Self {
            order: payment_information.order,
            cardholder: payment_information.cardholder,
            card: payment_method_data,
            three_ds,
            credential_indicator: payment_information.credential_indicator,
            stored_on_file: payment_information.stored_on_file,
            tenant_id,
        })
    }
}

impl TryFrom<ArchipelRouterData<&PaymentsAuthorizeRouterData>>
    for ArchipelWalletAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ArchipelRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ArchipelRouterData {
            amount,
            tenant_id,
            router_data,
        } = item;

        let payment_information = ArchipelPaymentInformation::try_from((amount, router_data))?;
        let payment_method_data = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Wallet(wallet_data) => {
                TokenizedCardData::try_from((wallet_data, &item.router_data.payment_method_token))?
            }
            PaymentMethodData::Card(..)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(..)
            | PaymentMethodData::CardRedirect(..)
            | PaymentMethodData::PayLater(..)
            | PaymentMethodData::BankRedirect(..)
            | PaymentMethodData::BankDebit(..)
            | PaymentMethodData::BankTransfer(..)
            | PaymentMethodData::Crypto(..)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(..)
            | PaymentMethodData::Upi(..)
            | PaymentMethodData::Voucher(..)
            | PaymentMethodData::GiftCard(..)
            | PaymentMethodData::CardToken(..)
            | PaymentMethodData::OpenBanking(..)
            | PaymentMethodData::NetworkToken(..)
            | PaymentMethodData::MobilePayment(..) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Archipel"),
            ))?,
        };

        Ok(Self {
            order: payment_information.order,
            cardholder: payment_information.cardholder,
            card: payment_method_data.card_data.clone(),
            wallet: payment_method_data.wallet_information.clone(),
            three_ds: None,
            credential_indicator: payment_information.credential_indicator,
            stored_on_file: payment_information.stored_on_file,
            tenant_id,
        })
    }
}

// Responses for AUTHORIZATION FLOW
impl TryFrom<PaymentsResponseRouterData<ArchipelPaymentsResponse>> for PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<ArchipelPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = item.response.error {
            return Ok(Self {
                response: Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
                ..item.data
            });
        };

        let capture_method = item
            .data
            .request
            .capture_method
            .ok_or_else(|| errors::ConnectorError::CaptureMethodNotSupported)?;

        let (archipel_flow, is_incremental_allowed) = match capture_method {
            CaptureMethod::Automatic => (ArchipelPaymentFlow::Pay, false),
            _ => (
                ArchipelPaymentFlow::Authorize,
                item.data.request.request_incremental_authorization,
            ),
        };

        let connector_metadata: Option<serde_json::Value> =
            ArchipelTransactionMetadata::from(&item.response)
                .encode_to_value()
                .ok();

        let status: AttemptStatus =
            ArchipelFlowStatus::new(item.response.transaction_result, archipel_flow).into();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                charges: None,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                // Save archipel initial transaction uuid for network transaction mit/cit
                network_txn_id: item
                    .data
                    .request
                    .is_customer_initiated_mandate_payment()
                    .then_some(item.response.transaction_id),
                connector_response_reference_id: None,
                incremental_authorization_allowed: Some(is_incremental_allowed),
            }),
            ..item.data
        })
    }
}

// PSYNC FLOW
impl TryFrom<PaymentsSyncResponseRouterData<ArchipelPaymentsResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<ArchipelPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = item.response.error {
            return Ok(Self {
                response: Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
                ..item.data
            });
        };

        let connector_metadata: Option<serde_json::Value> =
            ArchipelTransactionMetadata::from(&item.response)
                .encode_to_value()
                .ok();

        let capture_method = item
            .data
            .request
            .capture_method
            .ok_or_else(|| errors::ConnectorError::CaptureMethodNotSupported)?;

        let archipel_flow: ArchipelPaymentFlow = (item.data.status, capture_method).try_into()?;

        let status: AttemptStatus =
            ArchipelFlowStatus::new(item.response.transaction_result, archipel_flow).into();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                charges: None,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

/* CAPTURE FLOW */

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ArchipelCaptureRequest {
    order: ArchipelCaptureOrderRequest,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ArchipelCaptureOrderRequest {
    amount: MinorUnit,
}

impl From<ArchipelRouterData<&PaymentsCaptureRouterData>> for ArchipelCaptureRequest {
    fn from(item: ArchipelRouterData<&PaymentsCaptureRouterData>) -> Self {
        Self {
            order: ArchipelCaptureOrderRequest {
                amount: item.amount,
            },
        }
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<ArchipelPaymentsResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<ArchipelPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = item.response.error {
            return Ok(Self {
                response: Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
                ..item.data
            });
        };

        let connector_metadata: Option<serde_json::Value> =
            ArchipelTransactionMetadata::from(&item.response)
                .encode_to_value()
                .ok();

        let status: AttemptStatus = ArchipelFlowStatus::new(
            item.response.transaction_result,
            ArchipelPaymentFlow::Capture,
        )
        .into();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                charges: None,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

// Setup Mandate FLow
impl TryFrom<ArchipelRouterData<&SetupMandateRouterData>> for ArchipelCardAuthorizationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ArchipelRouterData<&SetupMandateRouterData>) -> Result<Self, Self::Error> {
        let order = ArchipelOrderRequest {
            amount: item.amount,
            currency: item.router_data.request.currency.to_string(),
            certainty: ArchipelPaymentCertainty::Final,
            initiator: ArchipelPaymentInitiator::Customer,
        };

        let cardholder = Some(ArchipelCardHolder {
            billing_address: item
                .router_data
                .get_billing_address()
                .ok()
                .and_then(|address| address.to_archipel_billing_address()),
        });

        // NOTE: Archipel does not accept `card.card_holder_name` field without `cardholder` field.
        // So if `card_holder` is None, `card.card_holder_name` must also be None.
        // However, the reverse is allowed — the `cardholder` field can exist without `card.card_holder_name`.
        let card_holder_name = cardholder.as_ref().and_then(|_| {
            item.router_data
                .get_billing()
                .ok()
                .and_then(|billing| billing.get_optional_full_name())
        });

        let stored_on_file = true;

        let credential_indicator = Some(ArchipelCredentialIndicator {
            status: ArchipelCredentialIndicatorStatus::Initial,
            recurring: Some(false),
            transaction_id: None,
        });

        let payment_information = ArchipelPaymentInformation {
            order,
            cardholder,
            card_holder_name,
            stored_on_file,
            credential_indicator,
        };

        let card_data = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ccard) => ArchipelCard::try_from((
                payment_information.card_holder_name,
                payment_information.cardholder.clone(),
                ccard,
            ))?,
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Archipel"),
            ))?,
        };

        Ok(Self {
            order: payment_information.order,
            cardholder: payment_information.cardholder.clone(),
            card: card_data,
            three_ds: None,
            credential_indicator: payment_information.credential_indicator,
            stored_on_file: payment_information.stored_on_file,
            tenant_id: item.tenant_id,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            ArchipelPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ArchipelPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = item.response.error {
            return Ok(Self {
                response: Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
                ..item.data
            });
        };

        let connector_metadata: Option<serde_json::Value> =
            ArchipelTransactionMetadata::from(&item.response)
                .encode_to_value()
                .ok();

        let status: AttemptStatus = ArchipelFlowStatus::new(
            item.response.transaction_result,
            ArchipelPaymentFlow::Verify,
        )
        .into();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                charges: None,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: Some(item.response.transaction_id.clone()),
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: Some(false),
            }),
            ..item.data
        })
    }
}

//      Void Flow => /cancel/{order_id}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelPaymentsCancelRequest {
    tenant_id: ArchipelTenantId,
}

impl From<ArchipelRouterData<&PaymentsCancelRouterData>> for ArchipelPaymentsCancelRequest {
    fn from(item: ArchipelRouterData<&PaymentsCancelRouterData>) -> Self {
        Self {
            tenant_id: item.tenant_id,
        }
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<ArchipelPaymentsResponse>>
    for PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<ArchipelPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = item.response.error {
            return Ok(Self {
                response: Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
                ..item.data
            });
        };

        let connector_metadata: Option<serde_json::Value> =
            ArchipelTransactionMetadata::from(&item.response)
                .encode_to_value()
                .ok();

        let status: AttemptStatus = ArchipelFlowStatus::new(
            item.response.transaction_result,
            ArchipelPaymentFlow::Cancel,
        )
        .into();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                charges: None,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelIncrementalAuthorizationRequest {
    order: ArchipelOrderRequest,
    tenant_id: ArchipelTenantId,
}

// Incremental Authorization status mapping
impl From<ArchipelPaymentStatus> for AuthorizationStatus {
    fn from(status: ArchipelPaymentStatus) -> Self {
        match status {
            ArchipelPaymentStatus::Succeeded => Self::Success,
            ArchipelPaymentStatus::Failed => Self::Failure,
        }
    }
}

impl From<ArchipelRouterData<&PaymentsIncrementalAuthorizationRouterData>>
    for ArchipelIncrementalAuthorizationRequest
{
    fn from(item: ArchipelRouterData<&PaymentsIncrementalAuthorizationRouterData>) -> Self {
        Self {
            order: ArchipelOrderRequest {
                amount: item.amount,
                currency: item.router_data.request.currency.to_string(),
                certainty: ArchipelPaymentCertainty::Estimated,
                initiator: ArchipelPaymentInitiator::Customer,
            },
            tenant_id: item.tenant_id,
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            ArchipelPaymentsResponse,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsIncrementalAuthorizationData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ArchipelPaymentsResponse,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = AuthorizationStatus::from(item.response.transaction_result);

        let (error_code, error_message) = match (&status, item.response.error) {
            (AuthorizationStatus::Success, _) | (_, None) => (None, None),
            (_, Some(err)) => (Some(err.code), err.description),
        };

        Ok(Self {
            response: Ok(PaymentsResponseData::IncrementalAuthorizationResponse {
                status,
                error_code,
                error_message,
                connector_authorization_id: None,
            }),
            ..item.data
        })
    }
}

/* REFUND FLOW */
#[derive(Debug, Serialize)]
pub struct ArchipelRefundOrder {
    pub amount: MinorUnit,
    pub currency: Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelRefundRequest {
    pub order: ArchipelRefundOrder,
    pub tenant_id: ArchipelTenantId,
}

impl<F> From<ArchipelRouterData<&RefundsRouterData<F>>> for ArchipelRefundRequest {
    fn from(item: ArchipelRouterData<&RefundsRouterData<F>>) -> Self {
        Self {
            order: ArchipelRefundOrder {
                amount: item.amount,
                currency: item.router_data.request.currency,
            },
            tenant_id: item.tenant_id,
        }
    }
}

// Type definition for Refund Response
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArchipelRefundStatus {
    Accepted,
    Failed,
    #[default]
    Pending,
}

impl From<ArchipelPaymentStatus> for RefundStatus {
    fn from(item: ArchipelPaymentStatus) -> Self {
        match item {
            ArchipelPaymentStatus::Succeeded => Self::Success,
            ArchipelPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArchipelRefundOrderResponse {
    id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchipelRefundResponse {
    order: ArchipelRefundOrderResponse,
    status: ArchipelRefundStatus,
    transaction_result: ArchipelPaymentStatus,
    transaction_id: Option<String>,
    transaction_date: Option<String>,
    error: Option<ArchipelErrorMessage>,
}

impl TryFrom<ArchipelRefundResponse> for RefundsResponseData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(resp: ArchipelRefundResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_refund_id: resp
                .transaction_id
                .ok_or_else(|| errors::ConnectorError::ParsingFailed)?,
            refund_status: RefundStatus::from(resp.transaction_result),
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, ArchipelRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, ArchipelRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response.error {
            None => Ok(RefundsResponseData::try_from(item.response)?),
            Some(error) => Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, ArchipelRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, ArchipelRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response.error {
            None => Ok(RefundsResponseData::try_from(item.response)?),
            Some(error) => Err(ArchipelErrorMessageWithHttpCode::new(error, item.http_code).into()),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl From<ArchipelErrorMessageWithHttpCode> for ErrorResponse {
    fn from(
        ArchipelErrorMessageWithHttpCode {
            error_message,
            http_code,
        }: ArchipelErrorMessageWithHttpCode,
    ) -> Self {
        Self {
            status_code: http_code,
            code: error_message.code,
            attempt_status: None,
            connector_transaction_id: None,
            message: error_message
                .description
                .clone()
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: error_message.description,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}
