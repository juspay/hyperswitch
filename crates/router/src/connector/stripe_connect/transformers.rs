use common_utils::pii::Email;
use diesel_models::enums;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::RouterData,
    core::errors,
    types::{self, transformers::ForeignFrom},
    utils::OptionExt,
};

pub struct StripeConnectAuthType {
    pub(super) api_key: Secret<String>,
}

type Error = error_stack::Report<errors::ConnectorError>;

impl TryFrom<&types::ConnectorAuthType> for StripeConnectAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeConnectStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    Canceled,
    Consumed,
    Pending,
}

// StripeConnect error response
#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeConnectErrorResponse {
    pub error: ErrorDetails,
}

// Payouts
#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeConnectPayoutCreateRequest {
    amount: i64,
    currency: enums::Currency,
    destination: String,
    transfer_group: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct StripeConnectPayoutCreateResponse {
    id: String,
    object: String,
    amount: i64,
    amount_reversed: i64,
    balance_transaction: String,
    created: i32,
    currency: String,
    description: Option<String>,
    destination: String,
    destination_payment: String,
    livemode: bool,
    reversals: TransferReversals,
    reversed: bool,
    source_transaction: Option<String>,
    source_type: String,
    transfer_group: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct TransferReversals {
    object: String,
    has_more: bool,
    total_count: i32,
    url: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeConnectPayoutFulfillRequest {
    amount: i64,
    currency: enums::Currency,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct StripeConnectPayoutFulfillResponse {
    id: String,
    object: String,
    amount: i64,
    arrival_date: i32,
    automatic: bool,
    balance_transaction: String,
    created: i32,
    currency: String,
    description: Option<String>,
    destination: String,
    failure_balance_transaction: Option<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    livemode: bool,
    method: String,
    original_payout: Option<String>,
    reconciliation_status: String,
    reversed_by: Option<String>,
    source_type: String,
    statement_descriptor: Option<String>,
    status: StripeConnectStatus,
    #[serde(rename = "type")]
    account_type: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeConnectReversalRequest {
    amount: i64,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct StripeConnectReversalResponse {
    id: String,
    object: String,
    amount: i64,
    balance_transaction: String,
    created: i32,
    currency: String,
    destination_payment_refund: String,
    source_refund: Option<String>,
    transfer: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeConnectRecipientCreateRequest {
    #[serde(rename = "type")]
    account_type: String,
    country: enums::CountryAlpha2,
    email: Email,
    #[serde(rename = "capabilities[card_payments][requested]")]
    capabilities_card_payments: bool,
    #[serde(rename = "capabilities[transfers][requested]")]
    capabilities_transfers: bool,
    #[serde(rename = "tos_acceptance[date]")]
    tos_acceptance_date: i32,
    #[serde(rename = "tos_acceptance[ip]")]
    tos_acceptance_ip: String,
    business_type: String,
    #[serde(rename = "business_profile[mcc]")]
    business_profile_mcc: i32,
    #[serde(rename = "business_profile[url]")]
    business_profile_url: String,
    #[serde(rename = "business_profile[name]")]
    business_profile_name: Secret<String>,
    #[serde(rename = "company[address][line1]")]
    company_address_line1: Secret<String>,
    #[serde(rename = "company[address][line2]")]
    company_address_line2: Secret<String>,
    #[serde(rename = "company[address][postal_code]")]
    company_address_postal_code: Secret<String>,
    #[serde(rename = "company[address][city]")]
    company_address_city: String,
    #[serde(rename = "company[address][state]")]
    company_address_state: Secret<String>,
    #[serde(rename = "company[phone]")]
    company_phone: String,
    #[serde(rename = "company[tax_id]")]
    company_tax_id: String,
    #[serde(rename = "company[owners_provided]")]
    company_owners_provided: bool,
    #[serde(rename = "individual[first_name]")]
    individual_first_name: Secret<String>,
    #[serde(rename = "individual[last_name]")]
    individual_last_name: Secret<String>,
    #[serde(rename = "individual[dob][day]")]
    individual_dob_day: String,
    #[serde(rename = "individual[dob][month]")]
    individual_dob_month: String,
    #[serde(rename = "individual[dob][year]")]
    individual_dob_year: String,
    #[serde(rename = "individual[address][line1]")]
    individual_address_line1: Secret<String>,
    #[serde(rename = "individual[address][line2]")]
    individual_address_line2: Secret<String>,
    #[serde(rename = "individual[address][postal_code]")]
    individual_address_postal_code: Secret<String>,
    #[serde(rename = "individual[address][city]")]
    individual_address_city: String,
    #[serde(rename = "individual[address][state]")]
    individual_address_state: Secret<String>,
    #[serde(rename = "individual[email]")]
    individual_email: Email,
    #[serde(rename = "individual[phone]")]
    individual_phone: Secret<String>,
    #[serde(rename = "individual[id_number]")]
    individual_id_number: String,
    #[serde(rename = "individual[ssn_last_4]")]
    individual_ssn_last_4: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct StripeConnectRecipientCreateResponse {
    id: String,
    object: String,
    business_type: String,
    charges_enabled: bool,
    country: enums::CountryAlpha2,
    created: i32,
    default_currency: String,
    email: Email,
    payouts_enabled: bool,
    #[serde(rename = "type")]
    account_type: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeConnectRecipientAccountCreateRequest {
    Bank(RecipientBankAccountRequest),
    Card(RecipientCardAccountRequest),
    Token(RecipientTokenRequest),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct RecipientTokenRequest {
    external_account: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct RecipientCardAccountRequest {
    #[serde(rename = "external_account[object]")]
    external_account_object: String,
    #[serde(rename = "external_account[number]")]
    external_account_number: String,
    #[serde(rename = "external_account[exp_month]")]
    external_account_exp_month: String,
    #[serde(rename = "external_account[exp_year]")]
    external_account_exp_year: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct RecipientBankAccountRequest {
    #[serde(rename = "external_account[object]")]
    external_account_object: String,
    #[serde(rename = "external_account[country]")]
    external_account_country: enums::CountryAlpha2,
    #[serde(rename = "external_account[currency]")]
    external_account_currency: enums::Currency,
    #[serde(rename = "external_account[account_holder_name]")]
    external_account_account_holder_name: Secret<String>,
    #[serde(rename = "external_account[account_number]")]
    external_account_account_number: Secret<String>,
    #[serde(rename = "external_account[account_holder_type]")]
    external_account_account_holder_type: String,
    #[serde(rename = "external_account[routing_number]")]
    external_account_routing_number: Secret<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum StripeConnectRecipientAccountCreateResponse {
    Bank(RecipientBankAccountResponse),
    Card(RecipientCardAccountResponse),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct RecipientBankAccountResponse {
    id: String,
    object: String,
    account: String,
    account_holder_name: String,
    account_holder_type: String,
    account_type: Option<String>,
    bank_name: String,
    country: enums::CountryAlpha2,
    currency: String,
    default_for_currency: bool,
    fingerprint: String,
    last4: String,
    routing_number: String,
    status: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct RecipientCardAccountResponse {
    id: String,
    object: String,
    account: String,
    brand: String,
    country: enums::CountryAlpha2,
    currency: String,
    default_for_currency: bool,
    dynamic_last4: Option<String>,
    exp_month: i8,
    exp_year: i8,
    fingerprint: String,
    funding: String,
    last4: String,
    name: String,
    status: String,
}

// Payouts create/transfer request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectPayoutCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let connector_customer_id = item.get_connector_customer_id()?;
        Ok(Self {
            amount: request.amount,
            currency: request.destination_currency,
            destination: connector_customer_id,
            transfer_group: request.payout_id,
        })
    }
}

// Payouts create response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectPayoutCreateResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectPayoutCreateResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectPayoutCreateResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(enums::PayoutStatus::RequiresFulfillment),
                connector_payout_id: response.id,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

// Payouts fulfill request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectPayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        Ok(Self {
            amount: request.amount,
            currency: request.destination_currency,
        })
    }
}

// Payouts fulfill response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectPayoutFulfillResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectPayoutFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectPayoutFulfillResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(enums::PayoutStatus::foreign_from(response.status)),
                connector_payout_id: response.id,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

// Payouts reversal request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectReversalRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
    }
}

// Payouts reversal response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectReversalResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectReversalResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectReversalResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(enums::PayoutStatus::Cancelled),
                connector_payout_id: response.id,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

// Recipient creation request transform
// TODO: remove hardcoded data
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectRecipientCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let customer_details = request
            .customer_details
            .get_required_value("customer_details")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "customer_details",
            })?;
        let customer_email = customer_details
            .email
            .get_required_value("email")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "email",
            })?;
        let address = item.get_billing_address()?;
        let individual_first_name = address
            .first_name
            .clone()
            .get_required_value("first_name")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "address.first_name",
            })?;
        let individual_last_name = address
            .last_name
            .clone()
            .get_required_value("last_name")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "address.last_name",
            })?;
        Ok(Self {
            account_type: "custom".to_string(),
            country: request.country_code,
            email: customer_email.clone(),
            capabilities_card_payments: true,
            capabilities_transfers: true,
            tos_acceptance_date: 1680581051,
            tos_acceptance_ip: "103.159.11.202".to_string(),
            business_type: "individual".to_string(),
            business_profile_mcc: 5045,
            business_profile_url: "https://www.pastebin.com".to_string(),
            business_profile_name: individual_first_name.to_owned(),
            company_address_line1: Secret::new("address_full_match".to_string()),
            company_address_line2: Secret::new("Kimberly Way".to_string()),
            company_address_postal_code: Secret::new("31062".to_string()),
            company_address_city: "Milledgeville".to_string(),
            company_address_state: Secret::new("GA".to_string()),
            company_phone: "+16168205366".to_string(),
            company_tax_id: "000000000".to_string(),
            company_owners_provided: false,
            individual_first_name,
            individual_last_name,
            individual_dob_day: "01".to_string(),
            individual_dob_month: "01".to_string(),
            individual_dob_year: "1901".to_string(),
            individual_address_line1: address
                .line1
                .clone()
                .get_required_value("line1")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.line1",
                })?,
            individual_address_line2: address
                .line2
                .clone()
                .get_required_value("line2")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.line2",
                })?,
            individual_address_postal_code: address
                .zip
                .clone()
                .get_required_value("zip")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.zip",
                })?,
            individual_address_city: address
                .city
                .clone()
                .get_required_value("city")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.city",
                })?,
            individual_address_state: address
                .state
                .clone()
                .get_required_value("state")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.state",
                })?,
            individual_email: customer_email,
            individual_phone: customer_details
                .phone
                .get_required_value("phone")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "address.phone",
                })?,
            individual_id_number: "000000000".to_string(),
            individual_ssn_last_4: "0000".to_string(),
        })
    }
}

// Recipient creation response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectRecipientCreateResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectRecipientCreateResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectRecipientCreateResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(enums::PayoutStatus::RequiresPayoutMethodData),
                connector_payout_id: response.id,
                payout_eligible: None,
            }),
            ..item.data
        })
    }
}

// Recipient account's creation request
// TODO: remove hardcoded fields
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectRecipientAccountCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let payout_method_data = item.get_payout_method_data()?;
        let customer_details = request
            .customer_details
            .get_required_value("customer_details")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "customer_details",
            })?;
        let customer_name = customer_details
            .name
            .get_required_value("name")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "customer_details.name",
            })?;
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Card(c) => {
                Ok(Self::Token(RecipientTokenRequest {
                    external_account: "tok_visa_debit".to_string(),
                }))
            }
            api_models::payouts::PayoutMethodData::Bank(bank) => match bank {
                api_models::payouts::Bank::Ach(bank_details) => {
                    Ok(Self::Bank(RecipientBankAccountRequest {
                        external_account_object: "bank_account".to_string(),
                        external_account_country: request.country_code.to_owned(),
                        external_account_currency: request.destination_currency.to_owned(),
                        external_account_account_holder_name: customer_name,
                        external_account_account_holder_type: "individual".to_string(),
                        external_account_account_number: bank_details.bank_account_number,
                        external_account_routing_number: bank_details.bank_routing_number,
                    }))
                }
                api_models::payouts::Bank::Bacs(_) => Err(errors::ConnectorError::NotSupported {
                    message: "BACS payouts are not supported".to_string(),
                    connector: "StripeConnect",
                }
                .into()),
                api_models::payouts::Bank::Sepa(_) => Err(errors::ConnectorError::NotSupported {
                    message: "SEPA payouts are not supported".to_string(),
                    connector: "StripeConnect",
                }
                .into()),
            },
        }
    }
}

// Recipient account's creation response
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectRecipientAccountCreateResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectRecipientAccountCreateResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectRecipientAccountCreateResponse = item.response;

        match response {
            StripeConnectRecipientAccountCreateResponse::Bank(bank_response) => Ok(Self {
                response: Ok(types::PayoutsResponseData {
                    status: Some(enums::PayoutStatus::RequiresCreation),
                    connector_payout_id: bank_response.id,
                    payout_eligible: None,
                }),
                ..item.data
            }),
            StripeConnectRecipientAccountCreateResponse::Card(card_response) => Ok(Self {
                response: Ok(types::PayoutsResponseData {
                    status: Some(enums::PayoutStatus::RequiresCreation),
                    connector_payout_id: card_response.id,
                    payout_eligible: None,
                }),
                ..item.data
            }),
        }
    }
}

#[cfg(feature = "payouts")]
impl ForeignFrom<StripeConnectStatus> for enums::PayoutStatus {
    fn foreign_from(stripe_connect_status: StripeConnectStatus) -> Self {
        match stripe_connect_status {
            StripeConnectStatus::Succeeded => Self::Success,
            StripeConnectStatus::Failed => Self::Failed,
            StripeConnectStatus::Canceled => Self::Cancelled,
            StripeConnectStatus::Pending
            | StripeConnectStatus::Processing
            | StripeConnectStatus::Consumed => Self::Pending,
        }
    }
}
