#[cfg(feature = "payouts")]
use api_models::payouts::Bank;
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
#[cfg(feature = "payouts")]
use common_enums::PayoutEntityType;
#[cfg(feature = "payouts")]
use common_enums::{CountryAlpha2, PayoutStatus, PayoutType};
#[cfg(feature = "payouts")]
use common_utils::pii::Email;
use common_utils::types::FloatMajorUnit;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsResponseData, PayoutsRouterData};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
#[cfg(feature = "payouts")]
use crate::utils::get_unimplemented_payment_method_error_message;
#[cfg(feature = "payouts")]
use crate::utils::{PayoutsData as _, RouterData as _};

type Error = error_stack::Report<ConnectorError>;

#[derive(Debug, Serialize)]
pub struct WiseRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for WiseRouterData<T> {
    fn from((amount, router_data): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

pub struct WiseAuthType {
    pub(super) api_key: Secret<String>,
    #[allow(dead_code)]
    pub(super) profile_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WiseAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_id: key1.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

// Wise error response
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub timestamp: Option<String>,
    pub errors: Option<Vec<SubError>>,
    pub status: Option<WiseHttpStatus>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub message: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WiseHttpStatus {
    String(String),
    Number(u16),
}

impl Default for WiseHttpStatus {
    fn default() -> Self {
        Self::String("".to_string())
    }
}

impl WiseHttpStatus {
    pub fn get_status(&self) -> String {
        match self {
            Self::String(val) => val.clone(),
            Self::Number(val) => val.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubError {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
    pub field: Option<String>,
}

// Payouts
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseRecipientCreateRequest {
    currency: String,
    #[serde(rename = "type")]
    recipient_type: RecipientType,
    profile: Secret<String>,
    account_holder_name: Secret<String>,
    details: WiseBankDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum RecipientType {
    Aba,
    Iban,
    SortCode,
    SwiftCode,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountType {
    Checking,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseBankDetails {
    legal_type: LegalType,
    account_type: Option<AccountType>,
    address: Option<WiseAddressDetails>,
    post_code: Option<String>,
    nationality: Option<String>,
    account_holder_name: Option<Secret<String>>,
    email: Option<Email>,
    account_number: Option<Secret<String>>,
    city: Option<String>,
    sort_code: Option<Secret<String>>,
    iban: Option<Secret<String>>,
    bic: Option<Secret<String>>,
    transit_number: Option<Secret<String>>,
    routing_number: Option<Secret<String>>,
    abartn: Option<Secret<String>>,
    swift_code: Option<Secret<String>>,
    payin_reference: Option<String>,
    psp_reference: Option<String>,
    tax_id: Option<String>,
    order_id: Option<String>,
    job: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LegalType {
    Business,
    #[default]
    Private,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseAddressDetails {
    country: Option<CountryAlpha2>,
    country_code: Option<CountryAlpha2>,
    first_line: Option<Secret<String>>,
    post_code: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseRecipientCreateResponse {
    id: i64,
    business: Option<i64>,
    profile: Option<i64>,
    account_holder_name: Secret<String>,
    currency: String,
    country: String,
    #[serde(rename = "type")]
    request_type: String,
    details: Option<WiseBankDetails>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WisePayoutQuoteRequest {
    source_currency: String,
    target_currency: String,
    source_amount: Option<FloatMajorUnit>,
    target_amount: Option<FloatMajorUnit>,
    pay_out: WisePayOutOption,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WisePayOutOption {
    Balance,
    #[default]
    BankTransfer,
    Swift,
    SwiftOur,
    Interac,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WisePayoutQuoteResponse {
    source_amount: f64,
    client_id: String,
    id: String,
    status: WiseStatus,
    profile: i64,
    rate: Option<i8>,
    source_currency: Option<String>,
    target_currency: Option<String>,
    user: Option<i64>,
    rate_type: Option<WiseRateType>,
    pay_out: Option<WisePayOutOption>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WiseRateType {
    #[default]
    Fixed,
    Floating,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WisePayoutCreateRequest {
    target_account: i64,
    quote_uuid: String,
    customer_transaction_id: String,
    details: WiseTransferDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseTransferDetails {
    transfer_purpose: Option<String>,
    source_of_funds: Option<String>,
    transfer_purpose_sub_transfer_purpose: Option<String>,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WisePayoutResponse {
    id: i64,
    user: i64,
    target_account: i64,
    source_account: Option<i64>,
    quote_uuid: String,
    status: WiseStatus,
    reference: Option<String>,
    rate: Option<f32>,
    business: Option<i64>,
    details: Option<WiseTransferDetails>,
    has_active_issues: Option<bool>,
    source_currency: Option<String>,
    source_value: Option<f64>,
    target_currency: Option<String>,
    target_value: Option<f64>,
    customer_transaction_id: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WisePayoutFulfillRequest {
    #[serde(rename = "type")]
    fund_type: FundType,
}

// NOTE - Only balance is allowed as time of incorporating this field - https://api-docs.transferwise.com/api-reference/transfer#fund
#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FundType {
    #[default]
    Balance,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WiseFulfillResponse {
    status: WiseStatus,
    error_code: Option<String>,
    error_message: Option<String>,
    balance_transaction_id: Option<i64>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WiseStatus {
    Completed,
    Pending,
    Rejected,

    #[serde(rename = "cancelled")]
    Cancelled,

    #[serde(rename = "processing")]
    #[default]
    Processing,

    #[serde(rename = "incoming_payment_waiting")]
    IncomingPaymentWaiting,
}

#[cfg(feature = "payouts")]
fn get_payout_address_details(
    address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<WiseAddressDetails> {
    address.and_then(|add| {
        add.address.as_ref().map(|a| WiseAddressDetails {
            country: a.country,
            country_code: a.country,
            first_line: a.line1.clone(),
            post_code: a.zip.clone(),
            city: a.city.clone(),
            state: a.state.clone(),
        })
    })
}

#[cfg(feature = "payouts")]
fn get_payout_bank_details(
    payout_method_data: PayoutMethodData,
    address: Option<&hyperswitch_domain_models::address::Address>,
    entity_type: PayoutEntityType,
) -> Result<WiseBankDetails, ConnectorError> {
    let wise_address_details = match get_payout_address_details(address) {
        Some(a) => Ok(a),
        None => Err(ConnectorError::MissingRequiredField {
            field_name: "address",
        }),
    }?;
    match payout_method_data {
        PayoutMethodData::Bank(Bank::Ach(b)) => Ok(WiseBankDetails {
            legal_type: LegalType::from(entity_type),
            address: Some(wise_address_details),
            account_number: Some(b.bank_account_number.to_owned()),
            abartn: Some(b.bank_routing_number),
            account_type: Some(AccountType::Checking),
            ..WiseBankDetails::default()
        }),
        PayoutMethodData::Bank(Bank::Bacs(b)) => Ok(WiseBankDetails {
            legal_type: LegalType::from(entity_type),
            address: Some(wise_address_details),
            account_number: Some(b.bank_account_number.to_owned()),
            sort_code: Some(b.bank_sort_code),
            ..WiseBankDetails::default()
        }),
        PayoutMethodData::Bank(Bank::Sepa(b)) => Ok(WiseBankDetails {
            legal_type: LegalType::from(entity_type),
            address: Some(wise_address_details),
            iban: Some(b.iban.to_owned()),
            bic: b.bic,
            ..WiseBankDetails::default()
        }),
        _ => Err(ConnectorError::NotImplemented(
            get_unimplemented_payment_method_error_message("Wise"),
        ))?,
    }
}

// Payouts recipient create request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&WiseRouterData<&PayoutsRouterData<F>>> for WiseRecipientCreateRequest {
    type Error = Error;
    fn try_from(item_data: &WiseRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let item = item_data.router_data;
        let request = item.request.to_owned();
        let customer_details = request.customer_details.to_owned();
        let payout_method_data = item.get_payout_method_data()?;
        let bank_details = get_payout_bank_details(
            payout_method_data.to_owned(),
            item.get_optional_billing(),
            item.request.entity_type,
        )?;
        let source_id = match item.connector_auth_type.to_owned() {
            ConnectorAuthType::BodyKey { api_key: _, key1 } => Ok(key1),
            _ => Err(ConnectorError::MissingRequiredField {
                field_name: "source_id for PayoutRecipient creation",
            }),
        }?;
        let payout_type = request.get_payout_type()?;
        match payout_type {
            PayoutType::Card | PayoutType::Wallet | PayoutType::BankRedirect => {
                Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Wise"),
                ))?
            }
            PayoutType::Bank => {
                let account_holder_name = customer_details
                    .ok_or(ConnectorError::MissingRequiredField {
                        field_name: "customer_details for PayoutRecipient creation",
                    })?
                    .name
                    .ok_or(ConnectorError::MissingRequiredField {
                        field_name: "customer_details.name for PayoutRecipient creation",
                    })?;
                Ok(Self {
                    profile: source_id,
                    currency: request.destination_currency.to_string(),
                    recipient_type: RecipientType::try_from(payout_method_data)?,
                    account_holder_name,
                    details: bank_details,
                })
            }
        }
    }
}

// Payouts recipient fulfill response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, WiseRecipientCreateResponse>>
    for PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, WiseRecipientCreateResponse>,
    ) -> Result<Self, Self::Error> {
        let response: WiseRecipientCreateResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresCreation),
                connector_payout_id: Some(response.id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Payouts quote request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&WiseRouterData<&PayoutsRouterData<F>>> for WisePayoutQuoteRequest {
    type Error = Error;
    fn try_from(item_data: &WiseRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let item = item_data.router_data;
        let request = item.request.to_owned();
        let payout_type = request.get_payout_type()?;
        match payout_type {
            PayoutType::Bank => Ok(Self {
                source_amount: Some(item_data.amount),
                source_currency: request.source_currency.to_string(),
                target_amount: None,
                target_currency: request.destination_currency.to_string(),
                pay_out: WisePayOutOption::default(),
            }),
            PayoutType::Card | PayoutType::Wallet | PayoutType::BankRedirect => {
                Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Wise"),
                ))?
            }
        }
    }
}

// Payouts quote response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, WisePayoutQuoteResponse>> for PayoutsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, WisePayoutQuoteResponse>,
    ) -> Result<Self, Self::Error> {
        let response: WisePayoutQuoteResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresCreation),
                connector_payout_id: Some(response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Payouts transfer creation request
#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for WisePayoutCreateRequest {
    type Error = Error;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let payout_type = request.get_payout_type()?;
        match payout_type {
            PayoutType::Bank => {
                let connector_customer_id = item.get_connector_customer_id()?;
                let quote_uuid = item.get_quote_id()?;
                let wise_transfer_details = WiseTransferDetails {
                    transfer_purpose: None,
                    source_of_funds: None,
                    transfer_purpose_sub_transfer_purpose: None,
                };
                let target_account: i64 = connector_customer_id.trim().parse().map_err(|_| {
                    ConnectorError::MissingRequiredField {
                        field_name: "profile",
                    }
                })?;
                Ok(Self {
                    target_account,
                    quote_uuid,
                    customer_transaction_id: uuid::Uuid::new_v4().to_string(),
                    details: wise_transfer_details,
                })
            }
            PayoutType::Card | PayoutType::Wallet | PayoutType::BankRedirect => {
                Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Wise"),
                ))?
            }
        }
    }
}

// Payouts transfer creation response
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, WisePayoutResponse>> for PayoutsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, WisePayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response: WisePayoutResponse = item.response;
        let status = match PayoutStatus::from(response.status) {
            PayoutStatus::Cancelled => PayoutStatus::Cancelled,
            _ => PayoutStatus::RequiresFulfillment,
        };

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(status),
                connector_payout_id: Some(response.id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Payouts fulfill request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for WisePayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let payout_type = item.request.get_payout_type()?;
        match payout_type {
            PayoutType::Bank => Ok(Self {
                fund_type: FundType::default(),
            }),
            PayoutType::Card | PayoutType::Wallet | PayoutType::BankRedirect => {
                Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Wise"),
                ))?
            }
        }
    }
}

// Payouts fulfill response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, WiseFulfillResponse>> for PayoutsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, WiseFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        let response: WiseFulfillResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::from(response.status)),
                connector_payout_id: Some(
                    item.data
                        .request
                        .connector_payout_id
                        .clone()
                        .ok_or(ConnectorError::MissingConnectorTransactionID)?,
                ),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
impl From<WiseStatus> for PayoutStatus {
    fn from(wise_status: WiseStatus) -> Self {
        match wise_status {
            WiseStatus::Completed => Self::Initiated,
            WiseStatus::Rejected => Self::Failed,
            WiseStatus::Cancelled => Self::Cancelled,
            WiseStatus::Pending | WiseStatus::Processing | WiseStatus::IncomingPaymentWaiting => {
                Self::Pending
            }
        }
    }
}

#[cfg(feature = "payouts")]
impl From<PayoutEntityType> for LegalType {
    fn from(entity_type: PayoutEntityType) -> Self {
        match entity_type {
            PayoutEntityType::Individual
            | PayoutEntityType::Personal
            | PayoutEntityType::NonProfit
            | PayoutEntityType::NaturalPerson => Self::Private,
            PayoutEntityType::Company
            | PayoutEntityType::PublicSector
            | PayoutEntityType::Business => Self::Business,
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutMethodData> for RecipientType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(payout_method_type: PayoutMethodData) -> Result<Self, Self::Error> {
        match payout_method_type {
            PayoutMethodData::Bank(Bank::Ach(_)) => Ok(Self::Aba),
            PayoutMethodData::Bank(Bank::Bacs(_)) => Ok(Self::SortCode),
            PayoutMethodData::Bank(Bank::Sepa(_)) => Ok(Self::Iban),
            _ => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Wise"),
            )
            .into()),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct WisePayoutSyncResponse {
    id: u64,
    status: WiseSyncStatus,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WiseSyncStatus {
    IncomingPaymentWaiting,
    IncomingPaymentInitiated,
    Processing,
    FundsConverted,
    OutgoingPaymentSent,
    Cancelled,
    FundsRefunded,
    BouncedBack,
    ChargedBack,
    Unknown,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, WisePayoutSyncResponse>> for PayoutsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, WisePayoutSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::from(item.response.status)),
                connector_payout_id: Some(item.response.id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
impl From<WiseSyncStatus> for PayoutStatus {
    fn from(status: WiseSyncStatus) -> Self {
        match status {
            WiseSyncStatus::IncomingPaymentWaiting => Self::Pending,
            WiseSyncStatus::IncomingPaymentInitiated => Self::Pending,
            WiseSyncStatus::Processing => Self::Pending,
            WiseSyncStatus::FundsConverted => Self::Pending,
            WiseSyncStatus::OutgoingPaymentSent => Self::Success,
            WiseSyncStatus::Cancelled => Self::Cancelled,
            WiseSyncStatus::FundsRefunded => Self::Reversed,
            WiseSyncStatus::BouncedBack => Self::Pending,
            WiseSyncStatus::ChargedBack => Self::Reversed,
            WiseSyncStatus::Unknown => Self::Ineligible,
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct WisePayoutsWebhookBody {
    pub data: WisePayoutsWebhookData,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct WisePayoutsWebhookData {
    pub resource: WisePayoutsWebhookResource,
    pub current_state: WiseSyncStatus,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct WisePayoutsWebhookResource {
    pub id: u64,
}

#[cfg(feature = "payouts")]
impl From<WisePayoutsWebhookData> for WisePayoutSyncResponse {
    fn from(data: WisePayoutsWebhookData) -> Self {
        Self {
            id: data.resource.id,
            status: data.current_state,
        }
    }
}

#[cfg(feature = "payouts")]
pub fn get_wise_webhooks_event(
    state: WiseSyncStatus,
) -> api_models::webhooks::IncomingWebhookEvent {
    match state {
        WiseSyncStatus::IncomingPaymentWaiting => {
            api_models::webhooks::IncomingWebhookEvent::PayoutProcessing
        }
        WiseSyncStatus::IncomingPaymentInitiated => {
            api_models::webhooks::IncomingWebhookEvent::PayoutProcessing
        }
        WiseSyncStatus::Processing => api_models::webhooks::IncomingWebhookEvent::PayoutProcessing,
        WiseSyncStatus::FundsConverted => {
            api_models::webhooks::IncomingWebhookEvent::PayoutProcessing
        }
        WiseSyncStatus::OutgoingPaymentSent => {
            api_models::webhooks::IncomingWebhookEvent::PayoutSuccess
        }
        WiseSyncStatus::Cancelled => api_models::webhooks::IncomingWebhookEvent::PayoutCancelled,
        WiseSyncStatus::FundsRefunded => api_models::webhooks::IncomingWebhookEvent::PayoutReversed,
        WiseSyncStatus::BouncedBack => api_models::webhooks::IncomingWebhookEvent::PayoutProcessing,
        WiseSyncStatus::ChargedBack => api_models::webhooks::IncomingWebhookEvent::PayoutReversed,
        WiseSyncStatus::Unknown => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}
