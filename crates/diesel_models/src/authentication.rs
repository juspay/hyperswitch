use std::{fmt, str::FromStr};

use common_utils::errors;
use diesel::{
    backend::Backend,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types::Jsonb,
    AsChangeset, AsExpression, FromSqlRow, Identifiable, Insertable, Queryable,
};
use error_stack::report;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use serde_json;

use crate::schema::authentication;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication,  primary_key(authentication_id))]
pub struct Authentication {
    pub authentication_id: String,
    pub merchant_id: String,
    pub authentication_connector: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_ds_server_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

impl Authentication {
    pub fn is_separate_authn_required(&self) -> bool {
        self.maximum_supported_version
            .as_ref()
            .is_some_and(|version| version.major == 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct SemanticVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl FromStr for SemanticVersion {
    type Err = error_stack::Report<errors::ValidationError>;

    fn from_str(version_string: &str) -> Result<Self, Self::Err> {
        let mut parts = version_string.split('.');
        let error = errors::ValidationError::InvalidValue {
            message: format!("Version string is invalid: {}", version_string),
        };
        dbg!(parts.size_hint());
        // if parts.size_hint().0 != 3 {
        //     return Err(report!(error.clone()));
        // }
        let (major, minor, patch) = (
            parts
                .next()
                .ok_or(report!(error.clone()))?
                .parse::<u8>()
                .map_err(|_e| report!(error.clone()))?,
            parts
                .next()
                .ok_or(report!(error.clone()))?
                .parse::<u8>()
                .map_err(|_e| report!(error.clone()))?,
            parts
                .next()
                .ok_or(report!(error.clone()))?
                .parse::<u8>()
                .map_err(|_e| report!(error.clone()))?,
        );

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl<DB: Backend> FromSql<Jsonb, DB> for SemanticVersion
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for SemanticVersion
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

impl Serialize for SemanticVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let version_string = format!("{}.{}.{}", self.major, self.minor, self.patch);
        serializer.serialize_str(&version_string)
    }
}

impl<'de> Deserialize<'de> for SemanticVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version_string = String::deserialize(deserializer)?;
        let mut parts = version_string.split('.');
        // if parts.size_hint().0 != 3 {
        //     return Err(serde::de::Error::custom("Invalid version format"));
        // }
        let (major, minor, patch) = (
            parts
                .next()
                .ok_or(serde::de::Error::custom("Invalid version format"))?
                .parse::<u8>()
                .map_err(serde::de::Error::custom)?,
            parts
                .next()
                .ok_or(serde::de::Error::custom("Invalid version format"))?
                .parse::<u8>()
                .map_err(serde::de::Error::custom)?,
            parts
                .next()
                .ok_or(serde::de::Error::custom("Invalid version format"))?
                .parse::<u8>()
                .map_err(serde::de::Error::custom)?,
        );

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = authentication)]
pub struct AuthenticationNew {
    pub authentication_id: String,
    pub merchant_id: String,
    pub authentication_connector: String,
    pub connector_authentication_id: Option<String>,
    // pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

#[derive(Debug)]
pub enum AuthenticationUpdate {
    PreAuthenticationUpdate {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: SemanticVersion,
        connector_authentication_id: String,
        three_ds_method_data: String,
        three_ds_method_url: Option<String>,
        message_version: SemanticVersion,
        connector_metadata: Option<serde_json::Value>,
        authentication_status: common_enums::AuthenticationStatus,
        payment_method_id: Option<String>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
    },
    AuthenticationUpdate {
        authentication_value: Option<String>,
        trans_status: common_enums::TransactionStatus,
        authentication_type: common_enums::DecoupledAuthenticationType,
        acs_url: Option<String>,
        challenge_request: Option<String>,
        acs_reference_number: Option<String>,
        acs_trans_id: Option<String>,
        acs_signed_content: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
    },
    PostAuthenticationUpdate {
        trans_status: common_enums::TransactionStatus,
        authentication_value: Option<String>,
        eci: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
    },
    ErrorUpdate {
        error_message: Option<String>,
        error_code: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
        connector_authentication_id: Option<String>,
    },
    PostAuthorizationUpdate {
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = authentication)]
pub struct AuthenticationUpdateInternal {
    pub connector_authentication_id: Option<String>,
    // pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: Option<String>,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: Option<common_enums::AuthenticationStatus>,
    pub authentication_lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

impl Default for AuthenticationUpdateInternal {
    fn default() -> Self {
        Self {
            connector_authentication_id: Default::default(),
            payment_method_id: Default::default(),
            authentication_type: Default::default(),
            authentication_status: Default::default(),
            authentication_lifecycle_status: Default::default(),
            modified_at: common_utils::date_time::now(),
            error_message: Default::default(),
            error_code: Default::default(),
            connector_metadata: Default::default(),
            maximum_supported_version: Default::default(),
            threeds_server_transaction_id: Default::default(),
            cavv: Default::default(),
            authentication_flow_type: Default::default(),
            message_version: Default::default(),
            eci: Default::default(),
            trans_status: Default::default(),
            acquirer_bin: Default::default(),
            acquirer_merchant_id: Default::default(),
            three_ds_method_data: Default::default(),
            three_ds_method_url: Default::default(),
            acs_url: Default::default(),
            challenge_request: Default::default(),
            acs_reference_number: Default::default(),
            acs_trans_id: Default::default(),
            three_dsserver_trans_id: Default::default(),
            acs_signed_content: Default::default(),
        }
    }
}

impl AuthenticationUpdateInternal {
    pub fn apply_changeset(self, source: Authentication) -> Authentication {
        let Self {
            connector_authentication_id,
            payment_method_id,
            authentication_type,
            authentication_status,
            authentication_lifecycle_status,
            modified_at: _,
            error_code,
            error_message,
            connector_metadata,
            maximum_supported_version,
            threeds_server_transaction_id,
            cavv,
            authentication_flow_type,
            message_version,
            eci,
            trans_status,
            acquirer_bin,
            acquirer_merchant_id,
            three_ds_method_data,
            three_ds_method_url,
            acs_url,
            challenge_request,
            acs_reference_number,
            acs_trans_id,
            three_dsserver_trans_id,
            acs_signed_content,
        } = self;
        Authentication {
            connector_authentication_id: connector_authentication_id
                .or(source.connector_authentication_id),
            payment_method_id: payment_method_id.unwrap_or(source.payment_method_id),
            authentication_type: authentication_type.or(source.authentication_type),
            authentication_status: authentication_status.unwrap_or(source.authentication_status),
            authentication_lifecycle_status: authentication_lifecycle_status
                .unwrap_or(source.authentication_lifecycle_status),
            modified_at: common_utils::date_time::now(),
            error_code: error_code.or(source.error_code),
            error_message: error_message.or(source.error_message),
            connector_metadata: connector_metadata.or(source.connector_metadata),
            maximum_supported_version: maximum_supported_version
                .or(source.maximum_supported_version),
            threeds_server_transaction_id: threeds_server_transaction_id
                .or(source.threeds_server_transaction_id),
            cavv: cavv.or(source.cavv),
            authentication_flow_type: authentication_flow_type.or(source.authentication_flow_type),
            message_version: message_version.or(source.message_version),
            eci: eci.or(source.eci),
            trans_status: trans_status.or(source.trans_status),
            acquirer_bin: acquirer_bin.or(source.acquirer_bin),
            acquirer_merchant_id: acquirer_merchant_id.or(source.acquirer_merchant_id),
            three_ds_method_data: three_ds_method_data.or(source.three_ds_method_data),
            three_ds_method_url: three_ds_method_url.or(source.three_ds_method_url),
            acs_url: acs_url.or(source.acs_url),
            challenge_request: challenge_request.or(source.challenge_request),
            acs_reference_number: acs_reference_number.or(source.acs_reference_number),
            acs_trans_id: acs_trans_id.or(source.acs_trans_id),
            three_ds_server_trans_id: three_dsserver_trans_id.or(source.three_ds_server_trans_id),
            acs_signed_content: acs_signed_content.or(source.acs_signed_content),
            ..source
        }
    }
}

impl From<AuthenticationUpdate> for AuthenticationUpdateInternal {
    fn from(auth_update: AuthenticationUpdate) -> Self {
        match auth_update {
            AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => Self {
                error_code,
                error_message,
                authentication_status: Some(authentication_status),

                connector_authentication_id,
                authentication_type: None,
                authentication_lifecycle_status: None,
                modified_at: common_utils::date_time::now(),
                payment_method_id: None,
                connector_metadata: None,
                ..Default::default()
            },
            AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => Self {
                connector_authentication_id: None,

                payment_method_id: None,
                authentication_type: None,
                authentication_status: None,
                authentication_lifecycle_status: Some(authentication_lifecycle_status),
                modified_at: common_utils::date_time::now(),
                error_message: None,
                error_code: None,
                connector_metadata: None,
                ..Default::default()
            },
            AuthenticationUpdate::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                payment_method_id,
                acquirer_bin,
                acquirer_merchant_id,
            } => Self {
                threeds_server_transaction_id: Some(threeds_server_transaction_id),
                maximum_supported_version: Some(maximum_supported_3ds_version),
                connector_authentication_id: Some(connector_authentication_id),
                three_ds_method_data: Some(three_ds_method_data),
                three_ds_method_url,
                message_version: Some(message_version),
                connector_metadata,
                authentication_status: Some(authentication_status),
                payment_method_id,
                acquirer_bin,
                acquirer_merchant_id,
                ..Default::default()
            },
            AuthenticationUpdate::AuthenticationUpdate {
                authentication_value,
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                authentication_status,
            } => Self {
                cavv: authentication_value,
                trans_status: Some(trans_status),
                authentication_type: Some(authentication_type),
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                authentication_status: Some(authentication_status),
                ..Default::default()
            },
            AuthenticationUpdate::PostAuthenticationUpdate {
                trans_status,
                authentication_value,
                eci,
                authentication_status,
            } => Self {
                trans_status: Some(trans_status),
                cavv: authentication_value,
                eci,
                authentication_status: Some(authentication_status),
                ..Default::default()
            },
        }
    }
}
