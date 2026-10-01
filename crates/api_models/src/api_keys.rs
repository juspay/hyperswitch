use common_utils::custom_serde;
use hyperswitch_masking::StrongSecret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

/// Maximum number of characters allowed in an API Key name, matching the `api_keys.name` column.
pub const API_KEY_NAME_MAX_LENGTH: usize = 64;

/// Maximum number of characters allowed in an API Key description, matching the
/// `api_keys.description` column.
pub const API_KEY_DESCRIPTION_MAX_LENGTH: usize = 256;

/// Checks that a field does not exceed its maximum length. Postgres `VARCHAR(n)` limits are
/// expressed in characters, so the length is counted in characters rather than bytes.
fn validate_max_length(field_name: &str, value: &str, max_length: usize) -> Result<(), String> {
    if value.chars().count() > max_length {
        return Err(format!(
            "`{field_name}` must not exceed {max_length} characters"
        ));
    }
    Ok(())
}

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

impl CreateApiKeyRequest {
    /// Validates the lengths of the `name` and `description` fields.
    pub fn validate(&self) -> Result<(), String> {
        validate_max_length("name", &self.name, API_KEY_NAME_MAX_LENGTH)?;
        self.description
            .as_deref()
            .map(|description| {
                validate_max_length("description", description, API_KEY_DESCRIPTION_MAX_LENGTH)
            })
            .transpose()?;
        Ok(())
    }
}

/// The response body for creating an API Key.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    /// The identifier for the API Key.
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX", value_type = String)]
    pub key_id: common_utils::id_type::ApiKeyId,

    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,

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
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX", value_type = String)]
    pub key_id: common_utils::id_type::ApiKeyId,

    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,

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
    #[schema(value_type = String)]
    pub key_id: common_utils::id_type::ApiKeyId,

    #[serde(skip_deserializing)]
    #[schema(value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,
}

impl UpdateApiKeyRequest {
    /// Validates the lengths of the `name` and `description` fields, when provided.
    pub fn validate(&self) -> Result<(), String> {
        self.name
            .as_deref()
            .map(|name| validate_max_length("name", name, API_KEY_NAME_MAX_LENGTH))
            .transpose()?;
        self.description
            .as_deref()
            .map(|description| {
                validate_max_length("description", description, API_KEY_DESCRIPTION_MAX_LENGTH)
            })
            .transpose()?;
        Ok(())
    }
}

/// The response body for revoking an API Key.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeApiKeyResponse {
    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,

    /// The identifier for the API Key.
    #[schema(max_length = 64, example = "5hEEqkgJUyuxgSKGArHA4mWSnX", value_type = String)]
    pub key_id: common_utils::id_type::ApiKeyId,
    /// Indicates whether the API key was revoked or not.
    #[schema(example = "true")]
    pub revoked: bool,
}

/// The constraints that are applicable when listing API Keys associated with a merchant account.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    fn from(expiration: ApiKeyExpiration) -> Self {
        match expiration {
            ApiKeyExpiration::Never => None,
            ApiKeyExpiration::DateTime(date_time) => Some(date_time),
        }
    }
}

impl From<Option<PrimitiveDateTime>> for ApiKeyExpiration {
    fn from(date_time: Option<PrimitiveDateTime>) -> Self {
        date_time.map_or(Self::Never, Self::DateTime)
    }
}

// This implementation is required as otherwise, `serde` would serialize and deserialize
// `ApiKeyExpiration::Never` as `null`, which is not preferable.
// Reference: https://github.com/serde-rs/serde/issues/1560#issuecomment-506915291
mod never {
    const NEVER: &str = "never";

    pub fn serialize<S>(serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(NEVER)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NeverVisitor;

        impl serde::de::Visitor<'_> for NeverVisitor {
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
    use super::*;

    #[test]
    fn test_serialization() {
        assert_eq!(
            serde_json::to_string(&ApiKeyExpiration::Never).unwrap(),
            r#""never""#
        );

        let date = time::Date::from_calendar_date(2022, time::Month::September, 10).unwrap();
        let time = time::Time::from_hms(11, 12, 13).unwrap();
        assert_eq!(
            serde_json::to_string(&ApiKeyExpiration::DateTime(PrimitiveDateTime::new(
                date, time
            )))
            .unwrap(),
            r#""2022-09-10T11:12:13.000Z""#
        );
    }

    #[test]
    fn test_deserialization() {
        assert_eq!(
            serde_json::from_str::<ApiKeyExpiration>(r#""never""#).unwrap(),
            ApiKeyExpiration::Never
        );

        let date = time::Date::from_calendar_date(2022, time::Month::September, 10).unwrap();
        let time = time::Time::from_hms(11, 12, 13).unwrap();
        assert_eq!(
            serde_json::from_str::<ApiKeyExpiration>(r#""2022-09-10T11:12:13.000Z""#).unwrap(),
            ApiKeyExpiration::DateTime(PrimitiveDateTime::new(date, time))
        );
    }

    #[test]
    fn test_null() {
        let result = serde_json::from_str::<ApiKeyExpiration>("null");
        assert!(result.is_err());

        let result = serde_json::from_str::<Option<ApiKeyExpiration>>("null").unwrap();
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod api_key_request_validation_tests {
    use super::*;

    fn create_request(name: &str, description: Option<&str>) -> CreateApiKeyRequest {
        CreateApiKeyRequest {
            name: name.to_string(),
            description: description.map(str::to_string),
            expiration: ApiKeyExpiration::Never,
        }
    }

    fn update_request(name: Option<&str>, description: Option<&str>) -> UpdateApiKeyRequest {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "description": description,
        }))
        .expect("update request should deserialize")
    }

    #[test]
    fn create_request_accepts_fields_at_max_length() {
        let name = "n".repeat(API_KEY_NAME_MAX_LENGTH);
        let description = "d".repeat(API_KEY_DESCRIPTION_MAX_LENGTH);

        assert!(create_request(&name, Some(&description)).validate().is_ok());
        assert!(create_request(&name, None).validate().is_ok());
    }

    #[test]
    fn create_request_rejects_description_over_max_length() {
        let description = "d".repeat(API_KEY_DESCRIPTION_MAX_LENGTH + 1);

        let error = create_request("Sandbox key", Some(&description))
            .validate()
            .expect_err("description over the limit should be rejected");

        assert_eq!(error, "`description` must not exceed 256 characters");
    }

    #[test]
    fn create_request_rejects_name_over_max_length() {
        let name = "n".repeat(API_KEY_NAME_MAX_LENGTH + 1);

        let error = create_request(&name, None)
            .validate()
            .expect_err("name over the limit should be rejected");

        assert_eq!(error, "`name` must not exceed 64 characters");
    }

    #[test]
    fn length_is_counted_in_characters_not_bytes() {
        // Each "é" is two bytes in UTF-8, so this is 256 characters but 512 bytes.
        let description = "é".repeat(API_KEY_DESCRIPTION_MAX_LENGTH);

        assert!(create_request("Sandbox key", Some(&description))
            .validate()
            .is_ok());
    }

    #[test]
    fn update_request_accepts_missing_fields() {
        assert!(update_request(None, None).validate().is_ok());
    }

    #[test]
    fn update_request_rejects_fields_over_max_length() {
        let name = "n".repeat(API_KEY_NAME_MAX_LENGTH + 1);
        let description = "d".repeat(API_KEY_DESCRIPTION_MAX_LENGTH + 1);

        assert_eq!(
            update_request(Some(&name), None).validate(),
            Err("`name` must not exceed 64 characters".to_string())
        );
        assert_eq!(
            update_request(None, Some(&description)).validate(),
            Err("`description` must not exceed 256 characters".to_string())
        );
    }
}
