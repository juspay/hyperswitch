//! This module has common utilities for links in HyperSwitch

use std::{collections::HashSet, primitive::i64};

use common_enums::{enums, UIWidgetFormLayout};
use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types::Jsonb,
    AsExpression, FromSqlRow,
};
use error_stack::{report, ResultExt};
use masking::Secret;
use regex::Regex;
#[cfg(feature = "logs")]
use router_env::logger;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{consts, errors::ParsingError, id_type, types::MinorUnit};

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[diesel(sql_type = Jsonb)]
/// Link status enum
pub enum GenericLinkStatus {
    /// Status variants for payment method collect link
    PaymentMethodCollect(PaymentMethodCollectStatus),
    /// Status variants for payout link
    PayoutLink(PayoutLinkStatus),
}

impl Default for GenericLinkStatus {
    fn default() -> Self {
        Self::PaymentMethodCollect(PaymentMethodCollectStatus::Initiated)
    }
}

crate::impl_to_sql_from_sql_json!(GenericLinkStatus);

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
/// Status variants for payment method collect links
pub enum PaymentMethodCollectStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payment method details were submitted
    Submitted,
}

impl<DB: Backend> FromSql<Jsonb, DB> for PaymentMethodCollectStatus
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        let generic_status: GenericLinkStatus = serde_json::from_value(value)?;
        match generic_status {
            GenericLinkStatus::PaymentMethodCollect(status) => Ok(status),
            GenericLinkStatus::PayoutLink(_) => Err(report!(ParsingError::EnumParseFailure(
                "PaymentMethodCollectStatus"
            )))
            .attach_printable("Invalid status for PaymentMethodCollect")?,
        }
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for PaymentMethodCollectStatus
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    // This wraps PaymentMethodCollectStatus with GenericLinkStatus
    // Required for storing the status in required format in DB (GenericLinkStatus)
    // This type is used in PaymentMethodCollectLink (a variant of GenericLink, used in the application for avoiding conversion of data and status)
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(GenericLinkStatus::PaymentMethodCollect(self.clone()))?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
/// Status variants for payout links
pub enum PayoutLinkStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payout details were submitted
    Submitted,
}

impl<DB: Backend> FromSql<Jsonb, DB> for PayoutLinkStatus
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        let generic_status: GenericLinkStatus = serde_json::from_value(value)?;
        match generic_status {
            GenericLinkStatus::PayoutLink(status) => Ok(status),
            GenericLinkStatus::PaymentMethodCollect(_) => {
                Err(report!(ParsingError::EnumParseFailure("PayoutLinkStatus")))
                    .attach_printable("Invalid status for PayoutLink")?
            }
        }
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for PayoutLinkStatus
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    // This wraps PayoutLinkStatus with GenericLinkStatus
    // Required for storing the status in required format in DB (GenericLinkStatus)
    // This type is used in PayoutLink (a variant of GenericLink, used in the application for avoiding conversion of data and status)
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(GenericLinkStatus::PayoutLink(self.clone()))?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(Serialize, serde::Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// Payout link object
pub struct PayoutLinkData {
    /// Identifier for the payout link
    pub payout_link_id: String,
    /// Identifier for the customer
    pub customer_id: id_type::CustomerId,
    /// Identifier for the payouts resource
    pub payout_id: String,
    /// Link to render the payout link
    pub link: url::Url,
    /// Client secret generated for authenticating frontend APIs
    pub client_secret: Secret<String>,
    /// Expiry in seconds from the time it was created
    pub session_expiry: u32,
    #[serde(flatten)]
    /// Payout link's UI configurations
    pub ui_config: GenericLinkUiConfig,
    /// List of enabled payment methods
    pub enabled_payment_methods: Option<Vec<EnabledPaymentMethod>>,
    /// Payout amount
    pub amount: MinorUnit,
    /// Payout currency
    pub currency: enums::Currency,
    /// A list of allowed domains (glob patterns) where this link can be embedded / opened from
    pub allowed_domains: HashSet<String>,
    /// Form layout of the payout link
    pub form_layout: Option<UIWidgetFormLayout>,
    /// `test_mode` can be used for testing payout links without any restrictions
    pub test_mode: Option<bool>,
}

crate::impl_to_sql_from_sql_json!(PayoutLinkData);

/// Object for GenericLinkUiConfig
#[derive(Clone, Debug, serde::Deserialize, Serialize, ToSchema)]
pub struct GenericLinkUiConfig {
    /// Merchant's display logo
    #[schema(value_type = Option<String>, max_length = 255, example = "https://hyperswitch.io/favicon.ico")]
    pub logo: Option<url::Url>,

    /// Custom merchant name for the link
    #[schema(value_type = Option<String>, max_length = 255, example = "Hyperswitch")]
    pub merchant_name: Option<Secret<String>>,

    /// Primary color to be used in the form represented in hex format
    #[schema(value_type = Option<String>, max_length = 255, example = "#4285F4")]
    pub theme: Option<String>,
}

/// Object for GenericLinkUiConfigFormData
#[derive(Clone, Debug, serde::Deserialize, Serialize, ToSchema)]
pub struct GenericLinkUiConfigFormData {
    /// Merchant's display logo
    #[schema(value_type = String, max_length = 255, example = "https://hyperswitch.io/favicon.ico")]
    pub logo: url::Url,

    /// Custom merchant name for the link
    #[schema(value_type = String, max_length = 255, example = "Hyperswitch")]
    pub merchant_name: Secret<String>,

    /// Primary color to be used in the form represented in hex format
    #[schema(value_type = String, max_length = 255, example = "#4285F4")]
    pub theme: String,
}

/// Object for EnabledPaymentMethod
#[derive(Clone, Debug, Serialize, serde::Deserialize, ToSchema)]
pub struct EnabledPaymentMethod {
    /// Payment method (banks, cards, wallets) enabled for the operation
    #[schema(value_type = PaymentMethod)]
    pub payment_method: enums::PaymentMethod,

    /// An array of associated payment method types
    #[schema(value_type = HashSet<PaymentMethodType>)]
    pub payment_method_types: HashSet<enums::PaymentMethodType>,
}

/// Util function for validating a domain without any wildcard characters.
pub fn validate_strict_domain(domain: &str) -> bool {
    Regex::new(consts::STRICT_DOMAIN_REGEX)
        .map(|regex| regex.is_match(domain))
        .map_err(|err| {
            let err_msg = format!("Invalid strict domain regex: {err:?}");
            #[cfg(feature = "logs")]
            logger::error!(err_msg);
            err_msg
        })
        .unwrap_or(false)
}

/// Util function for validating a domain with "*" wildcard characters.
pub fn validate_wildcard_domain(domain: &str) -> bool {
    Regex::new(consts::WILDCARD_DOMAIN_REGEX)
        .map(|regex| regex.is_match(domain))
        .map_err(|err| {
            let err_msg = format!("Invalid strict domain regex: {err:?}");
            #[cfg(feature = "logs")]
            logger::error!(err_msg);
            err_msg
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod domain_tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn test_validate_strict_domain_regex() {
        assert!(
            Regex::new(consts::STRICT_DOMAIN_REGEX).is_ok(),
            "Strict domain regex is invalid"
        );
    }

    #[test]
    fn test_validate_wildcard_domain_regex() {
        assert!(
            Regex::new(consts::WILDCARD_DOMAIN_REGEX).is_ok(),
            "Wildcard domain regex is invalid"
        );
    }

    #[test]
    fn test_validate_strict_domain() {
        let valid_domains = vec![
            "example.com",
            "example.subdomain.com",
            "https://example.com:8080",
            "http://example.com",
            "example.com:8080",
            "example.com:443",
            "localhost:443",
            "127.0.0.1:443",
        ];

        for domain in valid_domains {
            assert!(
                validate_strict_domain(domain),
                "Could not validate strict domain: {}",
                domain
            );
        }

        let invalid_domains = vec![
            "",
            "invalid.domain.",
            "not_a_domain",
            "http://example.com/path?query=1#fragment",
            "127.0.0.1.2:443",
        ];

        for domain in invalid_domains {
            assert!(
                !validate_strict_domain(domain),
                "Could not validate invalid strict domain: {}",
                domain
            );
        }
    }

    #[test]
    fn test_validate_wildcard_domain() {
        let valid_domains = vec![
            "example.com",
            "example.subdomain.com",
            "https://example.com:8080",
            "http://example.com",
            "example.com:8080",
            "example.com:443",
            "localhost:443",
            "127.0.0.1:443",
            "*.com",
            "example.*.com",
            "example.com:*",
            "*:443",
            "localhost:*",
            "127.0.0.*:*",
            "*:*",
        ];

        for domain in valid_domains {
            assert!(
                validate_wildcard_domain(domain),
                "Could not validate wildcard domain: {}",
                domain
            );
        }

        let invalid_domains = vec![
            "",
            "invalid.domain.",
            "not_a_domain",
            "http://example.com/path?query=1#fragment",
            "*.",
            ".*",
            "example.com:*:",
            "*:443:",
            ":localhost:*",
            "127.00.*:*",
        ];

        for domain in invalid_domains {
            assert!(
                !validate_wildcard_domain(domain),
                "Could not validate invalid wildcard domain: {}",
                domain
            );
        }
    }
}
