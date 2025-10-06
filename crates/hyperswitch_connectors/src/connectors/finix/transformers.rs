pub mod request;
pub mod response;
use std::collections::HashMap;

use common_enums::{enums, AttemptStatus, CaptureMethod, CountryAlpha2, CountryAlpha3};
use common_utils::types::MinorUnit;
use hyperswitch_domain_models::{
    address::Address,
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_flow_types::{
        self as flows,
        refunds::{Execute, RSync},
        Authorize, Capture,
    },
    router_request_types::{
        ConnectorCustomerData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCaptureData, RefundsData, ResponseId,
    },
    router_response_types::{
        ConnectorCustomerResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types::RefundsRouterData,
};
use hyperswitch_interfaces::{consts, errors::ConnectorError};
use masking::{ExposeInterface, Secret};
pub use request::*;
pub use response::*;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::RouterData as _,
};

#[derive(Debug, Clone)]
pub enum FinixId {
    Auth(String),
    Transfer(String),
}

impl From<String> for FinixId {
    fn from(id: String) -> Self {
        if id.starts_with("AU") {
            Self::Auth(id)
        } else if id.starts_with("TR") {
            Self::Transfer(id)
        } else {
            // Default to Auth if the prefix doesn't match
            Self::Auth(id)
        }
    }
}

impl std::fmt::Display for FinixId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth(id) => write!(f, "{}", id),
            Self::Transfer(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixState {
    PENDING,
    SUCCEEDED,
    FAILED,
    CANCELED,
    #[serde(other)]
    UNKNOWN,
    // RETURNED
}
impl FinixState {
    pub fn is_failure(&self) -> bool {
        match self {
            FinixState::PENDING | FinixState::SUCCEEDED => false,
            FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixPaymentType {
    DEBIT,
    CREDIT,
    REVERSAL,
    FEE,
    ADJUSTMENT,
    DISPUTE,
    RESERVE,
    SETTLEMENT,
    #[serde(other)]
    UNKNOWN,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixPaymentInstrumentType {
    #[serde(rename = "PAYMENT_CARD")]
    PaymentCard,

    #[serde(rename = "BANK_ACCOUNT")]
    BankAccount,
}

/// Represents the type of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardType {
    DEBIT,
    CREDIT,
    PREPAID,
    UNKNOWN,
}

/// Represents the brand of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardBrand {
    Visa,
    Mastercard,
    AmericanExpress,
    Discover,
    JCB,
    DinersClub,
}

/// 3D Secure authentication details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixThreeDSecure {
    pub authenticated: Option<bool>,
    pub liability_shift: Option<String>,
    pub version: Option<String>,
    pub eci: Option<String>,
    pub cavv: Option<String>,
    pub xid: Option<String>,
}

/// Key-value pair tags.
pub type FinixTags = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixAddress {
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub city: Option<String>,
    pub region: Option<Secret<String>>,
    pub postal_code: Option<Secret<String>>,
    pub country: Option<CountryAlpha3>,
}

impl From<&Address> for FinixAddress {
    fn from(address: &Address) -> Self {
        let billing = address.address.as_ref();

        match billing {
            Some(address) => Self {
                line1: address.line1.clone(),
                line2: address.line2.clone(),
                city: address.city.clone(),
                region: address.state.clone(),
                postal_code: address.zip.clone(),
                country: address
                    .country
                    .clone()
                    .map(CountryAlpha2::from_alpha2_to_alpha3),
            },
            None => Self {
                line1: None,
                line2: None,
                city: None,
                region: None,
                postal_code: None,
                country: None,
            },
        }
    }
}

/// The type of the business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixBusinessType {
    #[serde(rename = "SOLE_PROPRIETORSHIP")]
    SoleProprietorship,
    PARTNERSHIP,
    LLC,
    CORPORATION,
}

/// The type of the business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixIdentityType {
    PERSONAL,
}

pub enum FinixFlow {
    Auth,
    Transfer,
}

impl FinixFlow {
    pub fn get_flow_for_auth(capture_method: CaptureMethod) -> Self {
        match capture_method {
            CaptureMethod::SequentialAutomatic | CaptureMethod::Automatic => Self::Transfer,
            CaptureMethod::Manual | CaptureMethod::ManualMultiple | CaptureMethod::Scheduled => {
                Self::Auth
            }
        }
    }
}

pub struct FinixRouterData<'a, Flow, Req, Res> {
    pub amount: MinorUnit,
    pub router_data: &'a RouterData<Flow, Req, Res>,
    pub merchant_id: Secret<String>,
}

impl<'a, Flow, Req, Res> TryFrom<(MinorUnit, &'a RouterData<Flow, Req, Res>)>
    for FinixRouterData<'a, Flow, Req, Res>
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(value: (MinorUnit, &'a RouterData<Flow, Req, Res>)) -> Result<Self, Self::Error> {
        let (amount, router_data) = value;
        let auth = FinixAuthType::try_from(&router_data.connector_auth_type)?;

        Ok(Self {
            amount,
            router_data,
            merchant_id: auth.merchant_id,
        })
    }
}

impl
    TryFrom<
        &FinixRouterData<
            '_,
            flows::CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
    > for FinixCreateIdentityRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            '_,
            flows::CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let customer_data: &ConnectorCustomerData = &item.router_data.request;
        let entity = FinixIdentityEntity {
            phone: customer_data.phone.clone(),
            first_name: item.router_data.get_optional_billing_first_name(),
            last_name: item.router_data.get_optional_billing_last_name(),
            email: item.router_data.get_optional_billing_email(),
            personal_address: item
                .router_data
                .get_optional_billing()
                .map(FinixAddress::from),
        };

        Ok(Self {
            entity,
            tags: None,
            identity_type: FinixIdentityType::PERSONAL,
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}

impl TryFrom<&FinixRouterData<'_, Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for FinixPaymentsRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => {
                // Check if we have a payment instrument token already
                let source = item.router_data.get_payment_method_token()?;

                Ok(Self {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                    source: match source {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Stax"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Stax"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Stax"))?
                        }
                    },
                    auth_type: Some("AUTHORIZATION".to_string()),
                    merchant: item.merchant_id.clone(), // todo
                    tags: None,
                    three_d_secure: None,
                })
            }
            _ => Err(
                ConnectorError::NotImplemented("Payment method not supported".to_string()).into(),
            ),
        }
    }
}

impl TryFrom<&FinixRouterData<'_, Capture, PaymentsCaptureData, PaymentsResponseData>>
    for FinixCaptureRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, Capture, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.router_data.request.minor_amount_to_capture,
        })
    }
}

impl
    TryFrom<
        &FinixRouterData<
            '_,
            flows::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    > for FinixCreatePaymentInstrumentRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            '_,
            flows::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let tokenization_data = &item.router_data.request;

        match &tokenization_data.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                // let address = item
                //     .router_data
                //     .get_billing_address()
                //     .map(FinixAddress::from);

                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::PaymentCard,
                    name: card_data.card_holder_name.clone(),
                    number: Some(Secret::new(card_data.card_number.clone().get_card_no())),
                    security_code: Some(card_data.card_cvc.clone()),
                    expiration_month: Some(Secret::new(
                        card_data
                            .card_exp_month
                            .clone()
                            .expose()
                            .parse::<i32>()
                            .unwrap_or(0),
                    )),
                    expiration_year: Some(Secret::new(
                        card_data
                            .card_exp_year
                            .clone()
                            .expose()
                            .parse::<i32>()
                            .unwrap_or(0),
                    )),
                    identity: item.router_data.get_connector_customer_id()?, // This would come from a previously created identity
                    tags: None,
                    address: None,
                    card_brand: None, // Finix determines this from the card number
                    card_type: None,  // Finix determines this from the card number
                    additional_data: None,
                })
            }
            _ => Err(ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        }
    }
}

// Implement response handling for tokenization
impl<F, T> TryFrom<ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: AttemptStatus::Charged,
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

// Auth Struct
pub struct FinixAuthType {
    pub finix_user_name: Secret<String>,
    pub finix_password: Secret<String>,
    pub merchant_id: Secret<String>,
}
impl TryFrom<&ConnectorAuthType> for FinixAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                finix_user_name: api_key.clone(),
                finix_password: api_secret.clone(),
                merchant_id: key1.clone(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FinixPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

fn get_attempt_status(state: FinixState, flow: FinixFlow, is_void: Option<bool>) -> AttemptStatus {
    if is_void == Some(true) {
        return match state {
            FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => {
                AttemptStatus::VoidFailed
            }
            FinixState::PENDING => AttemptStatus::Voided,
            FinixState::SUCCEEDED => AttemptStatus::Voided,
        };
    }
    match (flow, state) {
        (FinixFlow::Auth, FinixState::PENDING) => AttemptStatus::Authorizing,
        (FinixFlow::Auth, FinixState::SUCCEEDED) => AttemptStatus::Authorized,
        (FinixFlow::Auth, FinixState::FAILED) => AttemptStatus::AuthorizationFailed,
        (FinixFlow::Auth, FinixState::CANCELED) | (FinixFlow::Auth, FinixState::UNKNOWN) => {
            AttemptStatus::AuthorizationFailed
        }

        (FinixFlow::Transfer, FinixState::PENDING) => AttemptStatus::CaptureInitiated,
        (FinixFlow::Transfer, FinixState::SUCCEEDED) => AttemptStatus::Charged,
        (FinixFlow::Transfer, FinixState::FAILED)
        | (FinixFlow::Transfer, FinixState::CANCELED)
        | (FinixFlow::Transfer, FinixState::UNKNOWN) => AttemptStatus::CaptureFailed,
    }
}

pub(crate) fn get_finix_response<F, T>(
    router_data: ResponseRouterData<F, FinixPaymentsResponse, T, PaymentsResponseData>,
    finix_flow: FinixFlow,
) -> Result<RouterData<F, T, PaymentsResponseData>, error_stack::Report<ConnectorError>> {
    let status = get_attempt_status(
        router_data.response.state.clone(),
        finix_flow,
        router_data.response.is_void,
    );
    Ok(RouterData {
        status: status.clone(),
        response: if router_data.response.state.is_failure() {
            Err(ErrorResponse {
                code: router_data
                    .response
                    .failure_code
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: router_data
                    .response
                    .messages
                    .map_or(consts::NO_ERROR_MESSAGE.to_string(), |msg| msg.join(",")),
                reason: router_data.response.failure_message,
                status_code: router_data.http_code,
                attempt_status: Some(status.clone()),
                connector_transaction_id: Some(router_data.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(router_data.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        },
        ..router_data.data
    })
}

impl<F> TryFrom<&FinixRouterData<'_, F, RefundsData, RefundsResponseData>>
    for FinixCreateRefundRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, F, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let id = item.router_data.request.connector_transaction_id.clone();
        let refund_amount = item.router_data.request.minor_refund_amount;
        let currency = item.router_data.request.currency;
        Ok(Self::new(id, refund_amount, currency))
    }
}

impl From<FinixState> for enums::RefundStatus {
    fn from(item: FinixState) -> Self {
        match item {
            FinixState::PENDING => Self::Pending,
            FinixState::SUCCEEDED => Self::Success,
            FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => Self::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, FinixPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FinixPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FinixPaymentsResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FinixPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixErrorResponse {
    // pub status_code: u16,
    pub total: Option<i64>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<FinixErrorEmbedded>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixErrorEmbedded {
    pub errors: Option<Vec<FinixError>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixError {
    // pub logref: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

impl FinixErrorResponse {
    pub fn get_message(&self) -> String {
        self.embedded
            .as_ref()
            .and_then(|embedded| embedded.errors.as_ref())
            .and_then(|errors| errors.first())
            .and_then(|error| error.message.clone())
            .unwrap_or("".to_string())
    }

    pub fn get_code(&self) -> String {
        self.embedded
            .as_ref()
            .and_then(|embedded| embedded.errors.as_ref())
            .and_then(|errors| errors.first())
            .and_then(|error| error.code.clone())
            .unwrap_or("".to_string())
    }
}
