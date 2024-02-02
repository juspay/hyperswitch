use common_utils::custom_serde;
use masking::StrongSecret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

/// The request body for creating an API Key.
#[derive(Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateApiKeyRequest {
    /// A unique name for the API Key to help you identify it.
    #[schema(max_length = 64, example = "Sandbox integration key")]
    pub name: String,

    /// A description to provide more context about the API Key.
    #[schema(
        max_length = 256,
        example = "Key used by our developers to integrate with the sandbox environment"
    )]
    pub description: Option<String>,

    /// An expiration date for the API Key. Although we allow keys to never expire, we recommend
    /// rotating your keys once every 6 months.
    #[schema(example = "2022-09-10T10:11:12Z")]
    pub expiration: ApiKeyExpiration,
}

/// The response body for creating an API Key.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    /// The identifier for the API Key.
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX")]
    pub key_id: String,

    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The unique name for the API Key to help you identify it.
    #[schema(max_length = 64, example = "Sandbox integration key")]
    pub name: String,

    /// The description to provide more context about the API Key.
    #[schema(
        max_length = 256,
        example = "Key used by our developers to integrate with the sandbox environment"
    )]
    pub description: Option<String>,

    /// The plaintext API Key used for server-side API access. Ensure you store the API Key
    /// securely as you will not be able to see it again.
    #[schema(value_type = String, max_length = 128)]
    pub api_key: StrongSecret<String>,

    /// The time at which the API Key was created.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The expiration date for the API Key.
    #[schema(example = "2022-09-10T10:11:12Z")]
    pub expiration: ApiKeyExpiration,
    /*
    /// The date and time indicating when the API Key was last used.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_used: Option<PrimitiveDateTime>,
    */
}

/// The response body for retrieving an API Key.
#[derive(Debug, Serialize, ToSchema)]
pub struct RetrieveApiKeyResponse {
    /// The identifier for the API Key.
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX")]
    pub key_id: String,

    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The unique name for the API Key to help you identify it.
    #[schema(max_length = 64, example = "Sandbox integration key")]
    pub name: String,

    /// The description to provide more context about the API Key.
    #[schema(
        max_length = 256,
        example = "Key used by our developers to integrate with the sandbox environment"
    )]
    pub description: Option<String>,

    /// The first few characters of the plaintext API Key to help you identify it.
    #[schema(value_type = String, max_length = 64)]
    pub prefix: StrongSecret<String>,

    /// The time at which the API Key was created.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The expiration date for the API Key.
    #[schema(example = "2022-09-10T10:11:12Z")]
    pub expiration: ApiKeyExpiration,
    /*
    /// The date and time indicating when the API Key was last used.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_used: Option<PrimitiveDateTime>,
    */
}

/// The request body for updating an API Key.
#[derive(Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateApiKeyRequest {
    /// A unique name for the API Key to help you identify it.
    #[schema(max_length = 64, example = "Sandbox integration key")]
    pub name: Option<String>,

    /// A description to provide more context about the API Key.
    #[schema(
        max_length = 256,
        example = "Key used by our developers to integrate with the sandbox environment"
    )]
    pub description: Option<String>,

    /// An expiration date for the API Key. Although we allow keys to never expire, we recommend
    /// rotating your keys once every 6 months.
    #[schema(example = "2022-09-10T10:11:12Z")]
    pub expiration: Option<ApiKeyExpiration>,

    #[serde(skip_deserializing)]
    pub key_id: String,

    #[serde(skip_deserializing)]
    pub merchant_id: String,
}

/// The response body for revoking an API Key.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeApiKeyResponse {
    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The identifier for the API Key.
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX")]
    pub key_id: String,
    /// Indicates whether the API key was revoked or not.
    #[schema(example = "true")]
    pub revoked: bool,
}

/// The constraints that are applicable when listing API Keys associated with a merchant account.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListApiKeyConstraints {
    /// The maximum number of API Keys to include in the response.
    pub limit: Option<i64>,

    /// The number of API Keys to skip when retrieving the list of API keys.
    pub skip: Option<i64>,
}

/// The expiration date and time for an API Key.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ApiKeyExpiration {
    /// The API Key does not expire.
    #[serde(with = "never")]
    Never,

    /// The API Key expires at the specified date and time.
    #[serde(with = "custom_serde::iso8601")]
    DateTime(PrimitiveDateTime),
}

impl From<ApiKeyExpiration> for Option<PrimitiveDateTime> {
        /// Converts an ApiKeyExpiration enum into an Option<Self> where Self is the type of the expiration.
    /// If the ApiKeyExpiration is ApiKeyExpiration::Never, it returns None.
    /// If the ApiKeyExpiration is ApiKeyExpiration::DateTime, it returns Some with the date_time value.
    fn from(expiration: ApiKeyExpiration) -> Self {
        match expiration {
            ApiKeyExpiration::Never => None,
            ApiKeyExpiration::DateTime(date_time) => Some(date_time),
        }
    }
}

impl From<Option<PrimitiveDateTime>> for ApiKeyExpiration {
        /// Converts an Option<PrimitiveDateTime> into Self. If the Option is Some, it returns the corresponding Self::DateTime variant,
    /// otherwise it returns Self::Never.
    fn from(date_time: Option<PrimitiveDateTime>) -> Self {
        date_time.map_or(Self::Never, Self::DateTime)
    }
}

// This implementation is required as otherwise, `serde` would serialize and deserialize
// `ApiKeyExpiration::Never` as `null`, which is not preferable.
// Reference: https://github.com/serde-rs/serde/issues/1560#issuecomment-506915291
mod never {
    const NEVER: &str = "never";

        /// Serializes the value "NEVER" using the provided serializer.
    pub fn serialize<S>(serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(NEVER)
    }

        /// Deserialize method for deserializing a string into a Result<(), D::Error>
    pub fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NeverVisitor;

        impl<'de> serde::de::Visitor<'de> for NeverVisitor {
            type Value = ();

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, r#""{NEVER}""#)
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value == NEVER {
                    Ok(())
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                }
            }
        }

        deserializer.deserialize_str(NeverVisitor)
    }
}

impl<'a> ToSchema<'a> for ApiKeyExpiration {
        /// Returns a tuple containing a string identifying the schema as "ApiKeyExpiration" and a reference to a openapi schema. The schema is defined using OneOfBuilder to represent two possible object structures: one with a string schema type and enum values of "never", and the other with a string schema type and a known date-time format.
    fn schema() -> (
        &'a str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        use utoipa::openapi::{KnownFormat, ObjectBuilder, OneOfBuilder, SchemaFormat, SchemaType};

        (
            "ApiKeyExpiration",
            OneOfBuilder::new()
                .item(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                        .enum_values(Some(["never"])),
                )
                .item(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime))),
                )
                .into(),
        )
    }
}

#[cfg(test)]
mod api_key_expiration_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
        /// Tests the serialization of the ApiKeyExpiration enum using serde_json. 
    fn test_serialization() {
        assert_eq!(
            serde_json::to_string(&ApiKeyExpiration::Never).unwrap(),
            r#""never""#
        );

        let date = time::Date::from_calendar_date(2022, time::Month::September, 10).unwrap();
        let time = time::Time::from_hms(11, 12, 13).unwrap();
        assert_eq!(
            serde_json::to_string(&ApiKeyExpiration::DateTime(time::PrimitiveDateTime::new(
                date, time
            )))
            .unwrap(),
            r#""2022-09-10T11:12:13.000Z""#
        );
    }

    #[test]
        /// This method is used for testing deserialization of the ApiKeyExpiration enum from JSON strings.
    fn test_deserialization() {
        assert_eq!(
            serde_json::from_str::<ApiKeyExpiration>(r#""never""#).unwrap(),
            ApiKeyExpiration::Never
        );

        let date = time::Date::from_calendar_date(2022, time::Month::September, 10).unwrap();
        let time = time::Time::from_hms(11, 12, 13).unwrap();
        assert_eq!(
            serde_json::from_str::<ApiKeyExpiration>(r#""2022-09-10T11:12:13.000Z""#).unwrap(),
            ApiKeyExpiration::DateTime(time::PrimitiveDateTime::new(date, time))
        );
    }

    #[test]
        /// This method tests the deserialization of a null value into a custom struct type `ApiKeyExpiration`
    /// and an `Option` of the same struct type. It asserts that deserializing "null" into `ApiKeyExpiration`
    /// results in an error, and that deserializing "null" into `Option<ApiKeyExpiration>` results in a `None` value.
    fn test_null() {
        let result = serde_json::from_str::<ApiKeyExpiration>("null");
        assert!(result.is_err());

        let result = serde_json::from_str::<Option<ApiKeyExpiration>>("null").unwrap();
        assert_eq!(result, None);
    }
}
