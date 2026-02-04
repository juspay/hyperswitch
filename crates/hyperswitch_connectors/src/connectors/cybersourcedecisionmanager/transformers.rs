use api_models::payments::AdditionalPaymentData;
use common_enums::enums;
use common_utils::{pii, types::StringMajorUnit};
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{FrmCheckoutRouterData, FrmTransactionRouterData, ResponseRouterData},
    utils::{
        AddressDetailsData as _, FrmTransactionRouterDataRequest, RouterData as OtherRouterData,
    },
};

//TODO: Fill the struct with respective fields
pub struct CybersourcedecisionmanagerRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for CybersourcedecisionmanagerRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcedecisionmanagerTransactionRequest {
    decision_information: DecisionInformation,
    processing_information: TransactionProcessingInformation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionInformation {
    decision: TransactionDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProcessingInformation {
    action: Vec<ActionList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionList {
    Capture,
    Reverse,
}

// Auth Struct
pub struct CybersourcedecisionmanagerAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CybersourcedecisionmanagerAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// Fraud Response status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcedecisionmanagerStatus {
    Accepted,
    Rejected,
    PendingReview,
    Declined,
    PendingAuthentication,
    InvalidRequest,
    Challenge,
    AuthenticationFailed,
}

impl From<CybersourcedecisionmanagerStatus> for common_enums::FraudCheckStatus {
    fn from(item: CybersourcedecisionmanagerStatus) -> Self {
        match item {
            CybersourcedecisionmanagerStatus::Accepted => Self::Legit,
            CybersourcedecisionmanagerStatus::Rejected
            | CybersourcedecisionmanagerStatus::Declined => Self::Fraud,
            CybersourcedecisionmanagerStatus::PendingReview
            | CybersourcedecisionmanagerStatus::Challenge
            | CybersourcedecisionmanagerStatus::PendingAuthentication => Self::ManualReview,
            CybersourcedecisionmanagerStatus::InvalidRequest
            | CybersourcedecisionmanagerStatus::AuthenticationFailed => Self::TransactionFailure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcedecisionmanagerResponse {
    status: CybersourcedecisionmanagerStatus,
    id: String,
    error_information: Option<CybersourcedecisionmanagerErrorInformation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CybersourcedecisionmanagerErrorInformation {
    reason: Option<String>,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, CybersourcedecisionmanagerResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CybersourcedecisionmanagerResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                status: common_enums::FraudCheckStatus::from(item.response.status),
                connector_metadata: None,
                score: None,
                reason: item
                    .response
                    .error_information
                    .and_then(|info| info.reason.map(serde_json::Value::from)),
            }),
            ..item.data
        })
    }
}

impl TryFrom<&FrmTransactionRouterData> for CybersourcedecisionmanagerTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FrmTransactionRouterData) -> Result<Self, Self::Error> {
        let decision = match item.is_payment_successful() {
            Some(true) => TransactionDecision::Accept,
            Some(false) => TransactionDecision::Reject,
            //needs to be tested
            None => TransactionDecision::Reject,
        };
        let action = match decision {
            TransactionDecision::Accept => vec![ActionList::Capture],
            TransactionDecision::Reject => vec![ActionList::Reverse],
        };
        Ok(Self {
            decision_information: DecisionInformation { decision },
            processing_information: TransactionProcessingInformation { action },
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcedecisionmanagerTransactionResponse {
    pub id: String,
    pub status: CybersourcedecisionmanagerTransactionStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcedecisionmanagerTransactionStatus {
    Accepted,
    Rejected,
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            CybersourcedecisionmanagerTransactionResponse,
            T,
            FraudCheckResponseData,
        >,
    > for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcedecisionmanagerTransactionResponse,
            T,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                status: common_enums::FraudCheckStatus::from(item.response.status),
                connector_metadata: None,
                score: None,
                reason: None,
            }),
            ..item.data
        })
    }
}

impl From<CybersourcedecisionmanagerTransactionStatus> for common_enums::FraudCheckStatus {
    fn from(item: CybersourcedecisionmanagerTransactionStatus) -> Self {
        match item {
            CybersourcedecisionmanagerTransactionStatus::Accepted => Self::Legit,
            CybersourcedecisionmanagerTransactionStatus::Rejected => Self::Fraud,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceDecisionManagerErrorResponse {
    AuthenticationError(Box<CybersourceDecisionManagerAuthenticationErrorResponse>),
    //If the request resource is not available/exists in cybersource
    NotAvailableError(Box<CybersourceDecisionManagerNotAvailableErrorResponse>),
    StandardError(Box<CybersourceDecisionManagerStandardErrorResponse>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CybersourceDecisionManagerAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceDecisionManagerNotAvailableErrorResponse {
    pub errors: Vec<CybersourceNotAvailableErrorObject>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceNotAvailableErrorObject {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceDecisionManagerStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

pub fn get_error_reason(
    error_info: Option<String>,
    detailed_error_info: Option<String>,
    avs_error_info: Option<String>,
) -> Option<String> {
    match (error_info, detailed_error_info, avs_error_info) {
        (Some(message), Some(details), Some(avs_message)) => Some(format!(
            "{message}, detailed_error_information: {details}, avs_message: {avs_message}",
        )),
        (Some(message), Some(details), None) => {
            Some(format!("{message}, detailed_error_information: {details}"))
        }
        (Some(message), None, Some(avs_message)) => {
            Some(format!("{message}, avs_message: {avs_message}"))
        }
        (None, Some(details), Some(avs_message)) => {
            Some(format!("{details}, avs_message: {avs_message}"))
        }
        (Some(message), None, None) => Some(message),
        (None, Some(details), None) => Some(details),
        (None, None, Some(avs_message)) => Some(avs_message),
        (None, None, None) => None,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcedecisionmanagerCheckoutRequest {
    client_reference_information: ClientReferenceInformation,
    payment_information: Option<PaymentInformation>,
    order_information: OrderInformationWithBill,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(Box<CardPaymentInformation>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    expiration_month: Option<Secret<String>>,
    expiration_year: Option<Secret<String>>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: StringMajorUnit,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address1: Option<Secret<String>>,
    locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    administrative_area: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<Secret<String>>,
    country: Option<enums::CountryAlpha2>,
    email: Option<pii::Email>,
}

impl TryFrom<&CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>>
    for CybersourcedecisionmanagerCheckoutRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>,
    ) -> Result<Self, Self::Error> {
        let client_reference_information = ClientReferenceInformation::from(item);
        let email = item.router_data.request.email.clone();
        let address = item.router_data.get_optional_billing();
        let bill_to = address.and_then(|addr| {
            addr.address.as_ref().map(|addr| BillTo {
                first_name: addr.first_name.remove_new_line(),
                last_name: addr.last_name.remove_new_line(),
                address1: addr.line1.remove_new_line(),
                locality: addr.city.remove_new_line(),
                administrative_area: addr.to_state_code_as_optional().unwrap_or_else(|_| {
                    addr.state
                        .remove_new_line()
                        .as_ref()
                        .map(|state| truncate_string(state, 20)) //NOTE: Cybersource connector throws error if billing state exceeds 20 characters, so truncation is done to avoid payment failure
                }),
                postal_code: addr.zip.remove_new_line(),
                country: addr.country,
                email,
            })
        });
        let order_information = OrderInformationWithBill::try_from((item, bill_to))?;
        let payment_information = match item.router_data.request.payment_method_data.as_ref() {
            Some(AdditionalPaymentData::Card(card_info)) => Some(PaymentInformation::Cards(
                Box::new(CardPaymentInformation {
                    card: Card {
                        expiration_month: card_info.card_exp_month.clone(),
                        expiration_year: card_info.card_exp_year.clone(),
                        card_type: card_info
                            .card_network
                            .clone()
                            .and_then(|network| get_cybersource_card_type(network))
                            .map(|s| s.to_string()),
                    },
                }),
            )),
            Some(_) | None => None,
        };

        Ok(Self {
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}
impl From<&CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>>
    for ClientReferenceInformation
{
    fn from(item: &CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl
    TryFrom<(
        &CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>,
        Option<BillTo>,
    )> for OrderInformationWithBill
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, bill_to): (
            &CybersourcedecisionmanagerRouterData<&FrmCheckoutRouterData>,
            Option<BillTo>,
        ),
    ) -> Result<Self, Self::Error> {
        let currency = item.router_data.request.currency.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            },
        )?;
        Ok(Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency,
            },
            bill_to,
        })
    }
}

pub trait RemoveNewLine {
    fn remove_new_line(&self) -> Self;
}

impl RemoveNewLine for Option<Secret<String>> {
    fn remove_new_line(&self) -> Self {
        self.clone().map(|masked_value| {
            let new_string = masked_value.expose().replace("\n", " ");
            Secret::new(new_string)
        })
    }
}

impl RemoveNewLine for Option<String> {
    fn remove_new_line(&self) -> Self {
        self.clone().map(|value| value.replace("\n", " "))
    }
}

fn truncate_string(state: &Secret<String>, max_len: usize) -> Secret<String> {
    let exposed = state.clone().expose();
    let truncated = exposed.get(..max_len).unwrap_or(&exposed);
    Secret::new(truncated.to_string())
}

fn get_cybersource_card_type(card_network: common_enums::CardNetwork) -> Option<&'static str> {
    match card_network {
        common_enums::CardNetwork::Visa => Some("001"),
        common_enums::CardNetwork::Mastercard => Some("002"),
        common_enums::CardNetwork::AmericanExpress => Some("003"),
        common_enums::CardNetwork::JCB => Some("007"),
        common_enums::CardNetwork::DinersClub => Some("005"),
        common_enums::CardNetwork::Discover => Some("004"),
        common_enums::CardNetwork::CartesBancaires => Some("036"),
        common_enums::CardNetwork::UnionPay => Some("062"),
        //"042" is the type code for Masetro Cards(International). For Maestro Cards(UK-Domestic) the mapping should be "024"
        common_enums::CardNetwork::Maestro => Some("042"),
        common_enums::CardNetwork::Interac
        | common_enums::CardNetwork::RuPay
        | common_enums::CardNetwork::Star
        | common_enums::CardNetwork::Accel
        | common_enums::CardNetwork::Pulse
        | common_enums::CardNetwork::Nyce => None,
    }
}
