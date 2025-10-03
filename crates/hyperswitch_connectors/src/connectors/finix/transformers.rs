pub mod finix_common;
pub mod request;
pub mod response;
use common_enums::{enums, AttemptStatus, CaptureMethod};
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
pub use finix_common::*;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
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
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors::{self, ConnectorError};
use masking::{ExposeInterface, Secret};
pub use request::*;
pub use response::*;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::RouterData as _,
};

pub enum FinixFlow {
    // CreateConnectorCustomer,
    // Tokenization,
    Auth,
    Transfer,
    Void,
    Refund,
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
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
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

//------------------------
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
        // Create entity data
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

        // Create the request
        Ok(Self {
            entity,
            tags: None,
            identity_type: FinixIdentityType::PERSONAL,
        })
    }
}

// Implement response handling for Identity creation
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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FinixCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
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
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FinixPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

fn get_finix_status(_state: FinixState, flow: FinixFlow) -> AttemptStatus {
    match (flow, _state) {
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

        (FinixFlow::Void, FinixState::PENDING) => AttemptStatus::VoidInitiated,
        (FinixFlow::Void, FinixState::SUCCEEDED) => AttemptStatus::Voided,
        (FinixFlow::Void, FinixState::FAILED)
        | (FinixFlow::Void, FinixState::CANCELED)
        | (FinixFlow::Void, FinixState::UNKNOWN) => AttemptStatus::VoidFailed,

        (FinixFlow::Refund, FinixState::PENDING) => todo!(),
        (FinixFlow::Refund, FinixState::SUCCEEDED) => todo!(),
        (FinixFlow::Refund, FinixState::FAILED) => todo!(),
        (FinixFlow::Refund, FinixState::CANCELED) => todo!(),
        (FinixFlow::Refund, FinixState::UNKNOWN) => todo!(),
    }
}
//TODO: Fill the struct with respective fields

pub(crate) fn get_finix_response<F, T>(
    router_data: ResponseRouterData<F, FinixPaymentsResponse, T, PaymentsResponseData>,
    finix_flow: FinixFlow,
) -> Result<RouterData<F, T, PaymentsResponseData>, error_stack::Report<ConnectorError>> {
    Ok(RouterData {
        status: get_finix_status(router_data.response.state, finix_flow),
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(router_data.response.id),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..router_data.data
    })
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct FinixRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&FinixRouterData<'_, F, RefundsData, RefundsResponseData>> for FinixRefundRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, F, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
