use std::str::FromStr;

pub use common_enums::*;
use smithy::SmithyModel;
use utoipa::ToSchema;

pub use super::connector_enums::Connector;

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]

/// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithm {
    RoundRobin,
    MaxConversion,
    MinCost,
    Custom,
}

#[cfg(feature = "payouts")]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutConnectors {
    Adyen,
    Adyenplatform,
    Cybersource,
    Ebanx,
    Gigadat,
    Loonio,
    Nomupay,
    Nuvei,
    Payone,
    Paypal,
    Stripe,
    Wise,
    Worldpay,
    Worldpayxml,
}

#[cfg(feature = "v2")]
/// Whether active attempt is to be set/unset
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub enum UpdateActiveAttempt {
    /// Request to set the active attempt id
    #[schema(value_type = Option<String>)]
    Set(common_utils::id_type::GlobalAttemptId),
    /// To unset the active attempt id
    Unset,
}

#[cfg(feature = "payouts")]
impl From<PayoutConnectors> for RoutableConnectors {
    fn from(value: PayoutConnectors) -> Self {
        match value {
            PayoutConnectors::Adyen => Self::Adyen,
            PayoutConnectors::Adyenplatform => Self::Adyenplatform,
            PayoutConnectors::Cybersource => Self::Cybersource,
            PayoutConnectors::Ebanx => Self::Ebanx,
            PayoutConnectors::Gigadat => Self::Gigadat,
            PayoutConnectors::Loonio => Self::Loonio,
            PayoutConnectors::Nomupay => Self::Nomupay,
            PayoutConnectors::Nuvei => Self::Nuvei,
            PayoutConnectors::Payone => Self::Payone,
            PayoutConnectors::Paypal => Self::Paypal,
            PayoutConnectors::Stripe => Self::Stripe,
            PayoutConnectors::Wise => Self::Wise,
            PayoutConnectors::Worldpay => Self::Worldpay,
            PayoutConnectors::Worldpayxml => Self::Worldpayxml,
        }
    }
}

#[cfg(feature = "payouts")]
impl From<PayoutConnectors> for Connector {
    fn from(value: PayoutConnectors) -> Self {
        match value {
            PayoutConnectors::Adyen => Self::Adyen,
            PayoutConnectors::Adyenplatform => Self::Adyenplatform,
            PayoutConnectors::Cybersource => Self::Cybersource,
            PayoutConnectors::Ebanx => Self::Ebanx,
            PayoutConnectors::Gigadat => Self::Gigadat,
            PayoutConnectors::Loonio => Self::Loonio,
            PayoutConnectors::Nomupay => Self::Nomupay,
            PayoutConnectors::Nuvei => Self::Nuvei,
            PayoutConnectors::Payone => Self::Payone,
            PayoutConnectors::Paypal => Self::Paypal,
            PayoutConnectors::Stripe => Self::Stripe,
            PayoutConnectors::Wise => Self::Wise,
            PayoutConnectors::Worldpay => Self::Worldpay,
            PayoutConnectors::Worldpayxml => Self::Worldpayxml,
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<Connector> for PayoutConnectors {
    type Error = String;
    fn try_from(value: Connector) -> Result<Self, Self::Error> {
        match value {
            Connector::Adyen => Ok(Self::Adyen),
            Connector::Adyenplatform => Ok(Self::Adyenplatform),
            Connector::Cybersource => Ok(Self::Cybersource),
            Connector::Ebanx => Ok(Self::Ebanx),
            Connector::Gigadat => Ok(Self::Gigadat),
            Connector::Loonio => Ok(Self::Loonio),
            Connector::Nuvei => Ok(Self::Nuvei),
            Connector::Nomupay => Ok(Self::Nomupay),
            Connector::Payone => Ok(Self::Payone),
            Connector::Paypal => Ok(Self::Paypal),
            Connector::Stripe => Ok(Self::Stripe),
            Connector::Wise => Ok(Self::Wise),
            Connector::Worldpay => Ok(Self::Worldpay),
            Connector::Worldpayxml => Ok(Self::Worldpayxml),
            _ => Err(format!("Invalid payout connector {value}")),
        }
    }
}

#[cfg(feature = "frm")]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FrmConnectors {
    /// Signifyd Risk Manager. Official docs: https://docs.signifyd.com/
    Signifyd,
    Riskified,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaxConnectors {
    Taxjar,
}

#[derive(Clone, Debug, serde::Serialize, strum::EnumString, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingConnectors {
    Chargebee,
    Recurly,
    Stripebilling,
    Custombilling,
    #[cfg(feature = "dummy_connector")]
    DummyBillingConnector,
}

#[derive(Clone, Copy, Debug, serde::Serialize, strum::EnumString, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VaultConnectors {
    Vgs,
    HyperswitchVault,
    Tokenex,
}

impl From<VaultConnectors> for Connector {
    fn from(value: VaultConnectors) -> Self {
        match value {
            VaultConnectors::Vgs => Self::Vgs,
            VaultConnectors::HyperswitchVault => Self::HyperswitchVault,
            VaultConnectors::Tokenex => Self::Tokenex,
        }
    }
}

#[derive(
    Clone, Debug, serde::Deserialize, serde::Serialize, strum::Display, strum::EnumString, ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FrmAction {
    CancelTxn,
    AutoRefund,
    ManualReview,
}

#[derive(
    Clone, Debug, serde::Deserialize, serde::Serialize, strum::Display, strum::EnumString, ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FrmPreferredFlowTypes {
    Pre,
    Post,
}
#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedResponseReason {
    pub code: String,
    /// A message to merchant to give hint on next action he/she should do to resolve
    pub message: String,
}

/// Possible field type of required fields in payment_method_data
#[derive(
    Clone,
    Debug,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FieldType {
    UserCardNumber,
    UserCardExpiryMonth,
    UserCardExpiryYear,
    UserCardCvc,
    UserCardNetwork,
    UserFullName,
    UserEmailAddress,
    UserPhoneNumber,
    UserPhoneNumberCountryCode,           //phone number's country code
    UserCountry { options: Vec<String> }, //for country inside payment method data ex- bank redirect
    UserCurrency { options: Vec<String> },
    UserCryptoCurrencyNetwork, //for crypto network associated with the cryptopcurrency
    UserBillingName,
    UserAddressLine1,
    UserAddressLine2,
    UserAddressCity,
    UserAddressPincode,
    UserAddressState,
    UserAddressCountry { options: Vec<String> },
    UserShippingName,
    UserShippingAddressLine1,
    UserShippingAddressLine2,
    UserShippingAddressCity,
    UserShippingAddressPincode,
    UserShippingAddressState,
    UserShippingAddressCountry { options: Vec<String> },
    UserSocialSecurityNumber,
    UserBlikCode,
    UserBank,
    UserBankOptions { options: Vec<String> },
    UserBankAccountNumber,
    UserSourceBankAccountId,
    UserDestinationBankAccountId,
    Text,
    DropDown { options: Vec<String> },
    UserDateOfBirth,
    UserVpaId,
    LanguagePreference { options: Vec<String> },
    UserPixKey,
    UserCpf,
    UserCnpj,
    UserIban,
    UserBsbNumber,
    UserBankSortCode,
    UserBankRoutingNumber,
    UserBankType { options: Vec<String> },
    UserBankAccountHolderName,
    UserMsisdn,
    UserClientIdentifier,
    OrderDetailsProductName,
}

impl FieldType {
    pub fn get_billing_variants() -> Vec<Self> {
        vec![
            Self::UserBillingName,
            Self::UserAddressLine1,
            Self::UserAddressLine2,
            Self::UserAddressCity,
            Self::UserAddressPincode,
            Self::UserAddressState,
            Self::UserAddressCountry { options: vec![] },
        ]
    }

    pub fn get_shipping_variants() -> Vec<Self> {
        vec![
            Self::UserShippingName,
            Self::UserShippingAddressLine1,
            Self::UserShippingAddressLine2,
            Self::UserShippingAddressCity,
            Self::UserShippingAddressPincode,
            Self::UserShippingAddressState,
            Self::UserShippingAddressCountry { options: vec![] },
        ]
    }
}

/// This implementatiobn is to ignore the inner value of UserAddressCountry enum while comparing
impl PartialEq for FieldType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::UserCardNumber, Self::UserCardNumber) => true,
            (Self::UserCardExpiryMonth, Self::UserCardExpiryMonth) => true,
            (Self::UserCardExpiryYear, Self::UserCardExpiryYear) => true,
            (Self::UserCardCvc, Self::UserCardCvc) => true,
            (Self::UserFullName, Self::UserFullName) => true,
            (Self::UserEmailAddress, Self::UserEmailAddress) => true,
            (Self::UserPhoneNumber, Self::UserPhoneNumber) => true,
            (Self::UserPhoneNumberCountryCode, Self::UserPhoneNumberCountryCode) => true,
            (
                Self::UserCountry {
                    options: options_self,
                },
                Self::UserCountry {
                    options: options_other,
                },
            ) => options_self.eq(options_other),
            (
                Self::UserCurrency {
                    options: options_self,
                },
                Self::UserCurrency {
                    options: options_other,
                },
            ) => options_self.eq(options_other),
            (Self::UserCryptoCurrencyNetwork, Self::UserCryptoCurrencyNetwork) => true,
            (Self::UserBillingName, Self::UserBillingName) => true,
            (Self::UserAddressLine1, Self::UserAddressLine1) => true,
            (Self::UserAddressLine2, Self::UserAddressLine2) => true,
            (Self::UserAddressCity, Self::UserAddressCity) => true,
            (Self::UserAddressPincode, Self::UserAddressPincode) => true,
            (Self::UserAddressState, Self::UserAddressState) => true,
            (Self::UserAddressCountry { .. }, Self::UserAddressCountry { .. }) => true,
            (Self::UserShippingName, Self::UserShippingName) => true,
            (Self::UserShippingAddressLine1, Self::UserShippingAddressLine1) => true,
            (Self::UserShippingAddressLine2, Self::UserShippingAddressLine2) => true,
            (Self::UserShippingAddressCity, Self::UserShippingAddressCity) => true,
            (Self::UserShippingAddressPincode, Self::UserShippingAddressPincode) => true,
            (Self::UserShippingAddressState, Self::UserShippingAddressState) => true,
            (Self::UserShippingAddressCountry { .. }, Self::UserShippingAddressCountry { .. }) => {
                true
            }
            (Self::UserBlikCode, Self::UserBlikCode) => true,
            (Self::UserBank, Self::UserBank) => true,
            (Self::Text, Self::Text) => true,
            (
                Self::DropDown {
                    options: options_self,
                },
                Self::DropDown {
                    options: options_other,
                },
            ) => options_self.eq(options_other),
            (Self::UserDateOfBirth, Self::UserDateOfBirth) => true,
            (Self::UserVpaId, Self::UserVpaId) => true,
            (Self::UserPixKey, Self::UserPixKey) => true,
            (Self::UserCpf, Self::UserCpf) => true,
            (Self::UserCnpj, Self::UserCnpj) => true,
            (Self::LanguagePreference { .. }, Self::LanguagePreference { .. }) => true,
            (Self::UserMsisdn, Self::UserMsisdn) => true,
            (Self::UserClientIdentifier, Self::UserClientIdentifier) => true,
            (Self::OrderDetailsProductName, Self::OrderDetailsProductName) => true,
            _unused => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_partialeq_for_field_type() {
        let user_address_country_is_us = FieldType::UserAddressCountry {
            options: vec!["US".to_string()],
        };

        let user_address_country_is_all = FieldType::UserAddressCountry {
            options: vec!["ALL".to_string()],
        };

        assert!(user_address_country_is_us.eq(&user_address_country_is_all))
    }
}

/// Denotes the retry action
#[derive(
    Debug,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    Clone,
    PartialEq,
    Eq,
    ToSchema,
    SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum RetryAction {
    /// Manual retry through request is being deprecated, now it is available through profile
    ManualRetry,
    /// Denotes that the payment is requeued
    Requeue,
}

#[derive(Clone, Copy)]
pub enum LockerChoice {
    HyperswitchCardVault,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PmAuthConnectors {
    Plaid,
}

pub fn convert_pm_auth_connector(connector_name: &str) -> Option<PmAuthConnectors> {
    PmAuthConnectors::from_str(connector_name).ok()
}

pub fn convert_authentication_connector(connector_name: &str) -> Option<AuthenticationConnectors> {
    AuthenticationConnectors::from_str(connector_name).ok()
}

pub fn convert_tax_connector(connector_name: &str) -> Option<TaxConnectors> {
    TaxConnectors::from_str(connector_name).ok()
}

pub fn convert_billing_connector(connector_name: &str) -> Option<BillingConnectors> {
    BillingConnectors::from_str(connector_name).ok()
}
#[cfg(feature = "frm")]
pub fn convert_frm_connector(connector_name: &str) -> Option<FrmConnectors> {
    FrmConnectors::from_str(connector_name).ok()
}

pub fn convert_vault_connector(connector_name: &str) -> Option<VaultConnectors> {
    VaultConnectors::from_str(connector_name).ok()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, Hash)]
pub enum ReconPermissionScope {
    #[serde(rename = "R")]
    Read = 0,
    #[serde(rename = "RW")]
    Write = 1,
}

impl From<PermissionScope> for ReconPermissionScope {
    fn from(scope: PermissionScope) -> Self {
        match scope {
            PermissionScope::Read => Self::Read,
            PermissionScope::Write => Self::Write,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum TokenStatus {
    /// Indicates that the token is active and can be used for payments
    Active,
    /// Indicates that the token is inactive and can't be used for payments
    Inactive,
    /// Indicates that the token is suspended from network's end for some reason and can't be used for payments until it is re-activated
    Suspended,
    /// Indicates that the token is expired and can't be used for payments
    Expired,
    /// Indicates that the token is deleted and further can't be used for payments
    Deleted,
}
