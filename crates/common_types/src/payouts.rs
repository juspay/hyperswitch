//! Payout related types.

#[cfg(feature = "diesel")]
use common_utils::impl_to_sql_from_sql_json;
#[cfg(feature = "diesel")]
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Billing descriptor information for a payout.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub struct PayoutsBillingDescriptor {
    /// Reference displayed on the beneficiary's bank statement.
    pub reference: Option<String>,
    /// Statement descriptor displayed for the payout.
    pub statement_descriptor: Option<String>,
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(PayoutsBillingDescriptor);

/// Connector-specific metadata supplied with a payout request.
///
/// Each connector owns its nested contract so connector wire semantics do not leak into shared
/// payout fields.
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub struct ConnectorPayoutMetadata {
    /// Checkout.com-specific payout metadata.
    pub checkout: Option<CheckoutPayoutMetadata>,
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(ConnectorPayoutMetadata);

/// Checkout.com-specific metadata for card payouts.
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckoutPayoutMetadata {
    /// Funds Transfer Type assigned to the merchant by Checkout.com.
    /// Do not select or default this value independently. When omitted, Checkout receives `null`.
    pub funds_transfer_type: Option<CheckoutPayoutFundsTransferType>,
}

/// Checkout.com Funds Transfer Type values for Visa Direct and Mastercard Send payouts.
/// The configured value must be the one assigned to the merchant by Checkout.com.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutPayoutFundsTransferType {
    /// Visa Direct account-to-account transfer.
    AccountToAccount,
    /// Visa Direct funds transfer.
    FundsTransfer,
    /// Visa Direct cross-border money transfer.
    CrossBorderMoneyTransfer,
    /// Visa Direct peer-to-peer transfer.
    PeerToPeer,
    /// Visa Direct staged digital-wallet transfer.
    StagedDigitalWalletTransfer,
    /// Visa Direct funds disbursement.
    FundsDisbursement,
    /// Visa Direct loyalty payment.
    LoyaltyPayments,
    /// Visa Direct payroll or pension disbursement.
    PayrollAndPensionDisbursements,
    /// Visa Direct online-gambling transfer.
    OnlineGambling,
    /// Mastercard Send person-to-person transfer.
    MastercardPersonToPerson,
    /// Mastercard Send transfer to the recipient's own account.
    MastercardTransferToOwnAccount,
    /// Mastercard Send business disbursement.
    MastercardBusinessDisbursement,
    /// Mastercard Send business-to-business transfer.
    MastercardBusinessToBusiness,
    /// Mastercard Send gaming repayment.
    MastercardGamingRepay,
}

/// Normalized payout party/account data consumed by domain and connector code.
/// API sender input is converted in Hyperswitch; legacy vendor metadata is a fallback.
/// Existing field names and types are retained for connector compatibility.
#[derive(Default, Debug, ToSchema, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub struct PayoutVendorAccountDetails {
    /// Vendor or corporate party details, retaining the legacy field layout.
    pub vendor_details: PayoutVendorDetails,
    /// Individual party or account representative details.
    pub individual_details: PayoutIndividualDetails,
}

/// Vendor/account information with appended corporate sender fields.
#[derive(Default, Debug, Serialize, ToSchema, Clone, Deserialize, PartialEq, Eq)]
pub struct PayoutVendorDetails {
    /// Connector account type; empty for sender-only input without account setup.
    pub account_type: String,
    /// Existing party discriminator: individual or company.
    pub business_type: String,
    /// Merchant category code for the business.
    pub business_profile_mcc: Option<i32>,
    /// Business profile URL.
    pub business_profile_url: Option<String>,
    /// Business name; also carries the corporate sender company name.
    pub business_profile_name: Option<Secret<String>>,
    /// Corporate sender street address.
    pub company_address_line1: Option<Secret<String>>,
    /// Additional corporate sender street address.
    pub company_address_line2: Option<Secret<String>>,
    /// Corporate sender postal code.
    pub company_address_postal_code: Option<Secret<String>>,
    /// Corporate sender city.
    pub company_address_city: Option<Secret<String>>,
    /// Corporate sender state or province.
    pub company_address_state: Option<Secret<String>>,
    /// Corporate sender phone number.
    pub company_phone: Option<Secret<String>>,
    /// Company tax identifier.
    pub company_tax_id: Option<Secret<String>>,
    /// Whether account owners have been supplied.
    pub company_owners_provided: Option<bool>,
    /// Whether card payments capability is requested.
    pub capabilities_card_payments: Option<bool>,
    /// Whether transfer capability is requested.
    pub capabilities_transfers: Option<bool>,
    /// Sender fields not represented by the existing company fields above.
    pub company_address_country: Option<common_enums::CountryAlpha2>,
    /// Party contact email.
    pub email: Option<common_utils::pii::Email>,
    /// International dialing code, separate from the phone number.
    pub phone_country_code: Option<String>,
    /// Merchant-supplied stable identifier for the funding party.
    pub reference_id: Option<Secret<String>>,
    /// Merchant-supplied classification of the funding source.
    pub source_of_funds: Option<PayoutSourceOfFunds>,
}

/// Individual/account information with appended individual sender fields.
#[derive(Default, Debug, Serialize, ToSchema, Clone, Deserialize, PartialEq, Eq)]
pub struct PayoutIndividualDetails {
    /// Unix timestamp of terms-of-service acceptance.
    pub tos_acceptance_date: Option<i64>,
    /// IP address used to accept terms of service.
    pub tos_acceptance_ip: Option<Secret<String>>,
    /// Deprecated. Use `date_of_birth`.
    #[schema(deprecated, value_type = Option<String>)]
    pub individual_dob_day: Option<Secret<String>>,
    /// Deprecated. Use `date_of_birth`.
    #[schema(deprecated, value_type = Option<String>)]
    pub individual_dob_month: Option<Secret<String>>,
    /// Deprecated. Use `date_of_birth`.
    #[schema(deprecated, value_type = Option<String>)]
    pub individual_dob_year: Option<Secret<String>>,
    /// The individual's date of birth.
    #[schema(value_type = Option<time::Date>, example = "1990-01-31")]
    pub date_of_birth: Option<Secret<time::Date>>,
    /// Individual government identification number.
    pub individual_id_number: Option<Secret<String>>,
    /// Last four digits of the individual social security number.
    pub individual_ssn_last_4: Option<Secret<String>>,
    /// Existing individual/company account-holder classification.
    pub external_account_account_holder_type: Option<String>,
    /// Sender fields not represented by the existing individual fields above.
    pub first_name: Option<Secret<String>>,
    /// Individual sender family name; existing billing fallback applies when absent.
    pub last_name: Option<Secret<String>>,
    /// Individual sender address, separate from beneficiary billing.
    pub address: Option<PayoutVendorAddress>,
    /// Party contact email.
    pub email: Option<common_utils::pii::Email>,
    /// Individual sender phone number.
    pub phone_number: Option<Secret<String>>,
    /// International dialing code, separate from the phone number.
    pub phone_country_code: Option<String>,
    /// Merchant-supplied stable identifier for the funding party.
    pub reference_id: Option<Secret<String>>,
    /// Merchant-supplied classification of the funding source.
    pub source_of_funds: Option<PayoutSourceOfFunds>,
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(PayoutVendorAccountDetails);

/// Postal address for an individual funding a payout.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PayoutVendorAddress {
    /// Street address.
    pub line1: Option<Secret<String>>,
    /// Additional street address.
    pub line2: Option<Secret<String>>,
    /// City.
    pub city: Option<Secret<String>>,
    /// State or province.
    pub state: Option<Secret<String>>,
    /// Postal code.
    pub zip: Option<Secret<String>>,
    /// ISO country code.
    pub country: Option<common_enums::CountryAlpha2>,
}

/// Merchant-supplied classification of the payout funding source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayoutSourceOfFunds {
    /// Credit funding.
    Credit,
    /// Debit funding.
    Debit,
    /// Prepaid funding.
    Prepaid,
    /// Deposit account funding.
    DepositAccount,
    /// Mobile money account funding.
    MobileMoneyAccount,
    /// Cash funding.
    Cash,
}

/// Party providing the funds, independently of the destination account holder.
///
/// This is the API and persistence representation of the sender identity. Domain and
/// connector code consumes only the normalized [`PayoutVendorAccountDetails`] converted
/// from this type at the router-data boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub enum SenderDetails {
    /// A person providing the funds.
    Individual(IndividualSenderDetails),
    /// A legal entity providing the funds.
    Corporate(CorporateSenderDetails),
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(SenderDetails);

impl SenderDetails {
    /// Merchant-supplied sender reference, independent of the sender kind.
    pub fn reference_id(&self) -> Option<&Secret<String>> {
        match self {
            Self::Individual(sender) => sender.reference_id.as_ref(),
            Self::Corporate(sender) => sender.reference_id.as_ref(),
        }
    }

    /// Whether any explicitly supplied sender name is blank.
    pub fn has_blank_name(&self) -> bool {
        use hyperswitch_masking::PeekInterface;
        match self {
            Self::Individual(sender) => [&sender.first_name, &sender.last_name]
                .into_iter()
                .flatten()
                .any(|name| name.peek().trim().is_empty()),
            Self::Corporate(sender) => sender.company_name.peek().trim().is_empty(),
        }
    }
}

/// Individual sender identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IndividualSenderDetails {
    /// Given name; billing may be used when omitted.
    #[schema(value_type = Option<String>)]
    pub first_name: Option<Secret<String>>,
    /// Family name; billing may be used when omitted.
    #[schema(value_type = Option<String>)]
    pub last_name: Option<Secret<String>>,
    /// Sender birth date.
    #[schema(value_type = Option<String>)]
    pub date_of_birth: Option<Secret<time::Date>>,
    /// Sender address, independent of beneficiary billing.
    pub address: Option<SenderAddress>,
    /// Sender email.
    #[schema(value_type = Option<String>)]
    pub email: Option<common_utils::pii::Email>,
    /// Sender phone.
    pub phone: Option<SenderPhone>,
    /// Merchant's stable identifier for the sender, at most 15 characters.
    #[schema(value_type = Option<String>, max_length = 15)]
    pub reference_id: Option<Secret<String>>,
    /// Origin of the funds; supplied by the merchant.
    pub source_of_funds: Option<SourceOfFunds>,
}

/// Corporate sender identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CorporateSenderDetails {
    /// Registered legal company name.
    #[schema(value_type = String)]
    pub company_name: Secret<String>,
    /// Sender address, independent of beneficiary billing.
    pub address: Option<SenderAddress>,
    /// Sender email.
    #[schema(value_type = Option<String>)]
    pub email: Option<common_utils::pii::Email>,
    /// Sender phone.
    pub phone: Option<SenderPhone>,
    /// Merchant's stable identifier for the sender, at most 15 characters.
    #[schema(value_type = Option<String>, max_length = 15)]
    pub reference_id: Option<Secret<String>>,
    /// Origin of the funds; supplied by the merchant.
    pub source_of_funds: Option<SourceOfFunds>,
}

/// Sender postal address.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SenderAddress {
    /// Street address.
    #[schema(value_type = Option<String>)]
    pub line1: Option<Secret<String>>,
    /// Additional street address.
    #[schema(value_type = Option<String>)]
    pub line2: Option<Secret<String>>,
    /// City.
    pub city: Option<String>,
    /// State or province.
    #[schema(value_type = Option<String>)]
    pub state: Option<Secret<String>>,
    /// Postal code.
    #[schema(value_type = Option<String>)]
    pub zip: Option<Secret<String>>,
    /// ISO country.
    pub country: Option<common_enums::CountryAlpha2>,
}

/// Sender phone contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SenderPhone {
    /// Phone number.
    #[schema(value_type = Option<String>)]
    pub number: Option<Secret<String>>,
    /// International dialling code.
    pub country_code: Option<String>,
}

/// Origin of funds supplied by a sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceOfFunds {
    /// Credit funding.
    Credit,
    /// Debit funding.
    Debit,
    /// Prepaid funding.
    Prepaid,
    /// Deposit account funding.
    DepositAccount,
    /// Mobile money funding.
    MobileMoneyAccount,
    /// Cash funding.
    Cash,
}

impl From<SourceOfFunds> for PayoutSourceOfFunds {
    fn from(value: SourceOfFunds) -> Self {
        match value {
            SourceOfFunds::Credit => Self::Credit,
            SourceOfFunds::Debit => Self::Debit,
            SourceOfFunds::Prepaid => Self::Prepaid,
            SourceOfFunds::DepositAccount => Self::DepositAccount,
            SourceOfFunds::MobileMoneyAccount => Self::MobileMoneyAccount,
            SourceOfFunds::Cash => Self::Cash,
        }
    }
}

impl From<SenderAddress> for PayoutVendorAddress {
    fn from(value: SenderAddress) -> Self {
        Self {
            line1: value.line1,
            line2: value.line2,
            city: value.city.map(Secret::new),
            state: value.state,
            zip: value.zip,
            country: value.country,
        }
    }
}

impl From<SenderDetails> for PayoutVendorAccountDetails {
    fn from(value: SenderDetails) -> Self {
        match value {
            SenderDetails::Individual(sender) => Self {
                vendor_details: PayoutVendorDetails {
                    business_type: "individual".to_owned(),
                    ..Default::default()
                },
                individual_details: PayoutIndividualDetails {
                    first_name: sender.first_name,
                    last_name: sender.last_name,
                    date_of_birth: sender.date_of_birth,
                    address: sender.address.map(Into::into),
                    email: sender.email,
                    phone_number: sender.phone.as_ref().and_then(|p| p.number.clone()),
                    phone_country_code: sender.phone.and_then(|p| p.country_code),
                    reference_id: sender.reference_id,
                    source_of_funds: sender.source_of_funds.map(Into::into),
                    ..Default::default()
                },
            },
            SenderDetails::Corporate(sender) => {
                let address = sender.address.unwrap_or_default();
                Self {
                    vendor_details: PayoutVendorDetails {
                        business_type: "company".to_owned(),
                        business_profile_name: Some(sender.company_name),
                        company_address_line1: address.line1,
                        company_address_line2: address.line2,
                        company_address_postal_code: address.zip,
                        company_address_city: address.city.map(Secret::new),
                        company_address_state: address.state,
                        company_address_country: address.country,
                        company_phone: sender.phone.as_ref().and_then(|p| p.number.clone()),
                        phone_country_code: sender.phone.and_then(|p| p.country_code),
                        email: sender.email,
                        reference_id: sender.reference_id,
                        source_of_funds: sender.source_of_funds.map(Into::into),
                        ..Default::default()
                    },
                    individual_details: PayoutIndividualDetails::default(),
                }
            }
        }
    }
}
