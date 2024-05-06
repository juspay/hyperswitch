use api_models;
use common_utils::pii::Email;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::ErrorDetails;
use crate::{
    connector::utils::{PayoutsData, RouterData},
    core::{errors, payments::CustomerDetailsExt},
    types::{self, storage::enums, PayoutIndividualDetailsExt},
    utils::OptionExt,
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeConnectPayoutStatus {
    Canceled,
    Failed,
    InTransit,
    Paid,
    Pending,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StripeConnectErrorResponse {
    pub error: ErrorDetails,
}

// Payouts
#[derive(Clone, Debug, Serialize)]
pub struct StripeConnectPayoutCreateRequest {
    amount: i64,
    currency: enums::Currency,
    destination: String,
    transfer_group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StripeConnectPayoutCreateResponse {
    id: String,
    description: Option<String>,
    source_transaction: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferReversals {
    object: String,
    has_more: bool,
    total_count: i32,
    url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StripeConnectPayoutFulfillRequest {
    amount: i64,
    currency: enums::Currency,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StripeConnectPayoutFulfillResponse {
    id: String,
    currency: String,
    description: Option<String>,
    failure_balance_transaction: Option<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    original_payout: Option<String>,
    reversed_by: Option<String>,
    statement_descriptor: Option<String>,
    status: StripeConnectPayoutStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct StripeConnectReversalRequest {
    amount: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StripeConnectReversalResponse {
    id: String,
    source_refund: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StripeConnectRecipientCreateRequest {
    #[serde(rename = "type")]
    account_type: String,
    country: Option<enums::CountryAlpha2>,
    email: Option<Email>,
    #[serde(rename = "capabilities[card_payments][requested]")]
    capabilities_card_payments: Option<bool>,
    #[serde(rename = "capabilities[transfers][requested]")]
    capabilities_transfers: Option<bool>,
    #[serde(rename = "tos_acceptance[date]")]
    tos_acceptance_date: Option<i64>,
    #[serde(rename = "tos_acceptance[ip]")]
    tos_acceptance_ip: Option<Secret<String>>,
    business_type: String,
    #[serde(rename = "business_profile[mcc]")]
    business_profile_mcc: Option<i32>,
    #[serde(rename = "business_profile[url]")]
    business_profile_url: Option<String>,
    #[serde(rename = "business_profile[name]")]
    business_profile_name: Option<Secret<String>>,
    #[serde(rename = "company[name]")]
    company_name: Option<Secret<String>>,
    #[serde(rename = "company[address][line1]")]
    company_address_line1: Option<Secret<String>>,
    #[serde(rename = "company[address][line2]")]
    company_address_line2: Option<Secret<String>>,
    #[serde(rename = "company[address][postal_code]")]
    company_address_postal_code: Option<Secret<String>>,
    #[serde(rename = "company[address][city]")]
    company_address_city: Option<Secret<String>>,
    #[serde(rename = "company[address][state]")]
    company_address_state: Option<Secret<String>>,
    #[serde(rename = "company[phone]")]
    company_phone: Option<Secret<String>>,
    #[serde(rename = "company[tax_id]")]
    company_tax_id: Option<Secret<String>>,
    #[serde(rename = "company[owners_provided]")]
    company_owners_provided: Option<bool>,
    #[serde(rename = "individual[first_name]")]
    individual_first_name: Option<Secret<String>>,
    #[serde(rename = "individual[last_name]")]
    individual_last_name: Option<Secret<String>>,
    #[serde(rename = "individual[dob][day]")]
    individual_dob_day: Option<Secret<String>>,
    #[serde(rename = "individual[dob][month]")]
    individual_dob_month: Option<Secret<String>>,
    #[serde(rename = "individual[dob][year]")]
    individual_dob_year: Option<Secret<String>>,
    #[serde(rename = "individual[address][line1]")]
    individual_address_line1: Option<Secret<String>>,
    #[serde(rename = "individual[address][line2]")]
    individual_address_line2: Option<Secret<String>>,
    #[serde(rename = "individual[address][postal_code]")]
    individual_address_postal_code: Option<Secret<String>>,
    #[serde(rename = "individual[address][city]")]
    individual_address_city: Option<String>,
    #[serde(rename = "individual[address][state]")]
    individual_address_state: Option<Secret<String>>,
    #[serde(rename = "individual[email]")]
    individual_email: Option<Email>,
    #[serde(rename = "individual[phone]")]
    individual_phone: Option<Secret<String>>,
    #[serde(rename = "individual[id_number]")]
    individual_id_number: Option<Secret<String>>,
    #[serde(rename = "individual[ssn_last_4]")]
    individual_ssn_last_4: Option<Secret<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StripeConnectRecipientCreateResponse {
    id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum StripeConnectRecipientAccountCreateRequest {
    Bank(RecipientBankAccountRequest),
    Card(RecipientCardAccountRequest),
    Token(RecipientTokenRequest),
}

#[derive(Clone, Debug, Serialize)]
pub struct RecipientTokenRequest {
    external_account: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecipientCardAccountRequest {
    #[serde(rename = "external_account[object]")]
    external_account_object: String,
    #[serde(rename = "external_account[number]")]
    external_account_number: Secret<String>,
    #[serde(rename = "external_account[exp_month]")]
    external_account_exp_month: Secret<String>,
    #[serde(rename = "external_account[exp_year]")]
    external_account_exp_year: Secret<String>,
}

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StripeConnectRecipientAccountCreateResponse {
    id: String,
}

// Payouts create/transfer request transform
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
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}

// Payouts fulfill request transform
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
                status: Some(enums::PayoutStatus::from(response.status)),
                connector_payout_id: response.id,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}

// Payouts reversal request transform
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectReversalRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
    }
}

// Payouts reversal response transform
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
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}

// Recipient creation request transform
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectRecipientCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let customer_details = request.get_customer_details()?;
        let customer_email = customer_details.get_email()?;
        let address = item.get_billing_address()?.clone();
        let payout_vendor_details = request.get_vendor_details()?;
        let (vendor_details, individual_details) = (
            payout_vendor_details.vendor_details,
            payout_vendor_details.individual_details,
        );
        Ok(Self {
            account_type: vendor_details.account_type,
            country: address.country,
            email: Some(customer_email.clone()),
            capabilities_card_payments: vendor_details.capabilities_card_payments,
            capabilities_transfers: vendor_details.capabilities_transfers,
            tos_acceptance_date: individual_details.tos_acceptance_date,
            tos_acceptance_ip: individual_details.tos_acceptance_ip,
            business_type: vendor_details.business_type,
            business_profile_mcc: vendor_details.business_profile_mcc,
            business_profile_url: vendor_details.business_profile_url,
            business_profile_name: vendor_details.business_profile_name.clone(),
            company_name: vendor_details.business_profile_name,
            company_address_line1: vendor_details.company_address_line1,
            company_address_line2: vendor_details.company_address_line2,
            company_address_postal_code: vendor_details.company_address_postal_code,
            company_address_city: vendor_details.company_address_city,
            company_address_state: vendor_details.company_address_state,
            company_phone: vendor_details.company_phone,
            company_tax_id: vendor_details.company_tax_id,
            company_owners_provided: vendor_details.company_owners_provided,
            individual_first_name: address.first_name,
            individual_last_name: address.last_name,
            individual_dob_day: individual_details.individual_dob_day,
            individual_dob_month: individual_details.individual_dob_month,
            individual_dob_year: individual_details.individual_dob_year,
            individual_address_line1: address.line1,
            individual_address_line2: address.line2,
            individual_address_postal_code: address.zip,
            individual_address_city: address.city,
            individual_address_state: address.state,
            individual_email: Some(customer_email),
            individual_phone: customer_details.phone,
            individual_id_number: individual_details.individual_id_number,
            individual_ssn_last_4: individual_details.individual_ssn_last_4,
        })
    }
}

// Recipient creation response transform
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
                status: Some(enums::PayoutStatus::RequiresVendorAccountCreation),
                connector_payout_id: response.id,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: true,
            }),
            ..item.data
        })
    }
}

// Recipient account's creation request
impl<F> TryFrom<&types::PayoutsRouterData<F>> for StripeConnectRecipientAccountCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let payout_method_data = item.get_payout_method_data()?;
        let customer_details = request.get_customer_details()?;
        let customer_name = customer_details.get_name()?;
        let payout_vendor_details = request.get_vendor_details()?;
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Card(_) => {
                Ok(Self::Token(RecipientTokenRequest {
                    external_account: "tok_visa_debit".to_string(),
                }))
            }
            api_models::payouts::PayoutMethodData::Bank(bank) => match bank {
                api_models::payouts::Bank::Ach(bank_details) => {
                    Ok(Self::Bank(RecipientBankAccountRequest {
                        external_account_object: "bank_account".to_string(),
                        external_account_country: bank_details
                            .bank_country_code
                            .get_required_value("bank_country_code")
                            .change_context(errors::ConnectorError::MissingRequiredField {
                                field_name: "bank_country_code",
                            })?,
                        external_account_currency: request.destination_currency.to_owned(),
                        external_account_account_holder_name: customer_name,
                        external_account_account_holder_type: payout_vendor_details
                            .individual_details
                            .get_external_account_account_holder_type()?,
                        external_account_account_number: bank_details.bank_account_number,
                        external_account_routing_number: bank_details.bank_routing_number,
                    }))
                }
                api_models::payouts::Bank::Bacs(_) => Err(errors::ConnectorError::NotSupported {
                    message: "BACS payouts are not supported".to_string(),
                    connector: "stripe",
                }
                .into()),
                api_models::payouts::Bank::Sepa(_) => Err(errors::ConnectorError::NotSupported {
                    message: "SEPA payouts are not supported".to_string(),
                    connector: "stripe",
                }
                .into()),
            },
            api_models::payouts::PayoutMethodData::Wallet(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Payouts via wallets are not supported".to_string(),
                    connector: "stripe",
                }
                .into())
            }
        }
    }
}

// Recipient account's creation response
impl<F> TryFrom<types::PayoutsResponseRouterData<F, StripeConnectRecipientAccountCreateResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, StripeConnectRecipientAccountCreateResponse>,
    ) -> Result<Self, Self::Error> {
        let response: StripeConnectRecipientAccountCreateResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(enums::PayoutStatus::RequiresCreation),
                connector_payout_id: response.id,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}

impl From<StripeConnectPayoutStatus> for enums::PayoutStatus {
    fn from(stripe_connect_status: StripeConnectPayoutStatus) -> Self {
        match stripe_connect_status {
            StripeConnectPayoutStatus::Paid => Self::Success,
            StripeConnectPayoutStatus::Failed => Self::Failed,
            StripeConnectPayoutStatus::Canceled => Self::Cancelled,
            StripeConnectPayoutStatus::Pending | StripeConnectPayoutStatus::InTransit => {
                Self::Pending
            }
        }
    }
}
