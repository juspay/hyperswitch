#[cfg(feature = "payouts")]
use common_enums::enums::PayoutEntityType;
use common_enums::{enums, Currency, PayoutStatus};
use common_utils::{pii::Email, types::FloatMajorUnit};
use hyperswitch_domain_models::router_data::ConnectorAuthType;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_response_types::PayoutsResponseData, types::PayoutsRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::utils::PayoutFulfillRequestData;
#[cfg(feature = "payouts")]
use crate::{types::PayoutsResponseRouterData, utils::RouterData as UtilsRouterData};

pub const PURPOSE_OF_PAYMENT_IS_OTHER: &str = "OTHER";

pub struct NomupayRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for NomupayRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub country: enums::CountryAlpha2,
    pub state_province: Secret<String>,
    pub street: Secret<String>,
    pub city: String,
    pub postal_code: Secret<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProfileType {
    #[default]
    Individual,
    Businness,
}

#[cfg(feature = "payouts")]
impl From<PayoutEntityType> for ProfileType {
    fn from(entity: PayoutEntityType) -> Self {
        match entity {
            PayoutEntityType::Personal
            | PayoutEntityType::NaturalPerson
            | PayoutEntityType::Individual => Self::Individual,
            _ => Self::Businness,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum NomupayGender {
    Male,
    Female,
    #[default]
    Other,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub profile_type: ProfileType,
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub date_of_birth: Secret<String>,
    pub gender: NomupayGender,
    pub email_address: Email,
    pub phone_number_country_code: Option<String>,
    pub phone_number: Option<Secret<String>>,
    pub address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccountRequest {
    pub account_id: Secret<String>,
    pub client_sub_account_id: Secret<String>,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    pub bank_id: Option<Secret<String>>,
    pub account_id: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAccountsType {
    pub country_code: String,
    pub currency_code: String,
    pub bank_id: Secret<String>,
    pub bank_account_id: Secret<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferMethodType {
    #[default]
    BankAccount,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethodRequest {
    pub country_code: enums::CountryAlpha2,
    pub currency_code: Currency,
    #[serde(rename = "type")]
    pub transfer_method_type: TransferMethodType,
    pub display_name: Secret<String>,
    pub bank_account: BankAccount,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NomupayPaymentRequest {
    pub source_id: Secret<String>,
    pub destination_id: Secret<String>,
    pub payment_reference: String,
    pub amount: FloatMajorUnit,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: Option<String>,
    pub internal_memo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub source_id: Secret<String>,
    pub source_currency_code: Currency,
    pub destination_currency_code: Currency,
    pub amount: FloatMajorUnit,
    pub include_fee: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    pub source_id: Secret<String>,
    pub id: String,
    pub destination_id: Secret<String>,
    pub payment_reference: String,
    pub amount: FloatMajorUnit,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccountResponse {
    pub account_id: Secret<String>,
    pub id: String,
    pub client_sub_account_id: Secret<String>,
    pub profile: Profile,
    pub virtual_accounts: Vec<VirtualAccountsType>,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethodResponse {
    pub parent_id: Secret<String>,
    pub account_id: Secret<String>,
    pub sub_account_id: Secret<String>,
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub country_code: String,
    pub currency_code: Currency,
    pub display_name: String,
    #[serde(rename = "type")]
    pub transfer_method_type: String,
    pub profile: Profile,
    pub bank_account: BankAccount,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NomupayPaymentResponse {
    pub id: String,
    pub status: NomupayPaymentStatus,
    pub created_on: String,
    pub last_updated: String,
    pub source_id: Secret<String>,
    pub destination_id: Secret<String>,
    pub payment_reference: String,
    pub amount: FloatMajorUnit,
    pub currency_code: String,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeesType {
    #[serde(rename = "type")]
    pub fees_type: String,
    pub fees: FloatMajorUnit,
    pub currency_code: Currency,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PayoutQuoteResponse {
    pub source_id: Secret<String>,
    pub destination_currency_code: Currency,
    pub amount: FloatMajorUnit,
    pub source_currency_code: Currency,
    pub include_fee: bool,
    pub fees: Vec<FeesType>,
    pub payment_reference: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitResponse {
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub source_id: Secret<String>,
    pub destination_id: Secret<String>,
    pub payment_reference: String,
    pub amount: FloatMajorUnit,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NomupayMetadata {
    pub private_key: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetailsType {
    pub loc: Vec<String>,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NomupayInnerError {
    pub error_code: String,
    pub error_description: Option<String>,
    pub validation_errors: Option<Vec<ValidationError>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NomupayErrorResponse {
    pub status: Option<String>,
    pub code: Option<u64>,
    pub error: Option<NomupayInnerError>,
    pub status_code: Option<u16>,
    pub detail: Option<Vec<DetailsType>>,
}

pub struct NomupayAuthType {
    pub(super) kid: Secret<String>,
    #[cfg(feature = "payouts")]
    pub(super) eid: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NomupayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            #[cfg(feature = "payouts")]
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                kid: api_key.to_owned(),
                eid: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NomupayPaymentStatus {
    Pending,
    Processed,
    Failed,
    #[default]
    Processing,
    Scheduled,
    PendingAccountActivation,
    PendingTransferMethodCreation,
    PendingAccountKyc,
}

impl From<NomupayPaymentStatus> for PayoutStatus {
    fn from(item: NomupayPaymentStatus) -> Self {
        match item {
            NomupayPaymentStatus::Processed => Self::Success,
            NomupayPaymentStatus::Failed => Self::Failed,
            NomupayPaymentStatus::Processing
            | NomupayPaymentStatus::Pending
            | NomupayPaymentStatus::Scheduled
            | NomupayPaymentStatus::PendingAccountActivation
            | NomupayPaymentStatus::PendingTransferMethodCreation
            | NomupayPaymentStatus::PendingAccountKyc => Self::Pending,
        }
    }
}

#[cfg(feature = "payouts")]
fn get_profile<F>(
    item: &PayoutsRouterData<F>,
    entity_type: PayoutEntityType,
) -> Result<Profile, error_stack::Report<errors::ConnectorError>> {
    let my_address = Address {
        country: item.get_billing_country()?,
        state_province: item.get_billing_state()?,
        street: item.get_billing_line1()?,
        city: item.get_billing_city()?,
        postal_code: item.get_billing_zip()?,
    };

    Ok(Profile {
        profile_type: ProfileType::from(entity_type),
        first_name: item.get_billing_first_name()?,
        last_name: item.get_billing_last_name()?,
        date_of_birth: Secret::new("1991-01-01".to_string()), // Query raised with Nomupay regarding why this field is required
        gender: NomupayGender::Other, // Query raised with Nomupay regarding why this field is required
        email_address: item.get_billing_email()?,
        phone_number_country_code: item
            .get_billing_phone()
            .map(|phone| phone.country_code.clone())?,
        phone_number: Some(item.get_billing_phone_number()?),
        address: my_address,
    })
}

// PoRecipient Request
#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for OnboardSubAccountRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let payout_type = request.payout_type;

        let profile = get_profile(item, request.entity_type)?;

        let nomupay_auth_type = NomupayAuthType::try_from(&item.connector_auth_type)?;

        match payout_type {
            Some(common_enums::PayoutType::Bank) => Ok(Self {
                account_id: nomupay_auth_type.eid,
                client_sub_account_id: Secret::new(request.payout_id),
                profile,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "This payment method is not implemented for Nomupay".to_string(),
            )
            .into()),
        }
    }
}

// PoRecipient Response
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, OnboardSubAccountResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, OnboardSubAccountResponse>,
    ) -> Result<Self, Self::Error> {
        let response: OnboardSubAccountResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresVendorAccountCreation),
                connector_payout_id: Some(response.id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

// PoRecipientAccount Request
#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for OnboardTransferMethodRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let payout_method_data = item.get_payout_method_data()?;
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Bank(bank) => match bank {
                api_models::payouts::Bank::Sepa(bank_details) => {
                    let bank_account = BankAccount {
                        bank_id: bank_details.bic,
                        account_id: bank_details.iban,
                    };

                    let country_iso2_code = item
                        .get_billing_country()
                        .unwrap_or(enums::CountryAlpha2::CA);

                    let profile = get_profile(item, item.request.entity_type)?;

                    Ok(Self {
                        country_code: country_iso2_code,
                        currency_code: item.request.destination_currency,
                        transfer_method_type: TransferMethodType::BankAccount,
                        display_name: item.get_billing_full_name()?,
                        bank_account,
                        profile,
                    })
                }
                other_bank => Err(errors::ConnectorError::NotSupported {
                    message: format!("{:?} is not supported", other_bank),
                    connector: "nomupay",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "This payment method is not implemented for Nomupay".to_string(),
            )
            .into()),
        }
    }
}

// PoRecipientAccount response
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, OnboardTransferMethodResponse>>
    for PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, OnboardTransferMethodResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresCreation),
                connector_payout_id: Some(item.response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

// PoFulfill Request
#[cfg(feature = "payouts")]
impl<F> TryFrom<(&PayoutsRouterData<F>, FloatMajorUnit)> for NomupayPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, amount): (&PayoutsRouterData<F>, FloatMajorUnit),
    ) -> Result<Self, Self::Error> {
        let nomupay_auth_type = NomupayAuthType::try_from(&item.connector_auth_type)?;
        let destination = item.request.clone().get_connector_transfer_method_id()?;

        Ok(Self {
            source_id: nomupay_auth_type.eid,
            destination_id: Secret::new(destination),
            payment_reference: item.request.clone().payout_id,
            amount,
            currency_code: item.request.destination_currency,
            purpose: PURPOSE_OF_PAYMENT_IS_OTHER.to_string(),
            description: item.description.clone(),
            internal_memo: item.description.clone(),
        })
    }
}

// PoFulfill response
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, NomupayPaymentResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, NomupayPaymentResponse>,
    ) -> Result<Self, Self::Error> {
        let response: NomupayPaymentResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::from(response.status)),
                connector_payout_id: Some(response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}
