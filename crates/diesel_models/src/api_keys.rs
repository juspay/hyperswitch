use diesel::{AsChangeset, AsExpression, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::api_keys;

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Identifiable, Queryable, Selectable,
)]
#[diesel(table_name = api_keys, primary_key(key_id), check_for_backend(diesel::pg::Pg))]
pub struct ApiKey {
    pub key_id: common_utils::id_type::ApiKeyId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: String,
    pub description: Option<String>,
    pub hashed_api_key: HashedApiKey,
    pub prefix: String,
    pub created_at: PrimitiveDateTime,
    pub expires_at: Option<PrimitiveDateTime>,
    pub last_used: Option<PrimitiveDateTime>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = api_keys)]
pub struct ApiKeyNew {
    pub key_id: common_utils::id_type::ApiKeyId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: String,
    pub description: Option<String>,
    pub hashed_api_key: HashedApiKey,
    pub prefix: String,
    pub created_at: PrimitiveDateTime,
    pub expires_at: Option<PrimitiveDateTime>,
    pub last_used: Option<PrimitiveDateTime>,
}

#[derive(Debug)]
pub enum ApiKeyUpdate {
    Update {
        name: Option<String>,
        description: Option<String>,
        expires_at: Option<Option<PrimitiveDateTime>>,
        last_used: Option<PrimitiveDateTime>,
    },
    LastUsedUpdate {
        last_used: PrimitiveDateTime,
    },
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = api_keys)]
pub(crate) struct ApiKeyUpdateInternal {
    pub name: Option<String>,
    pub description: Option<String>,
    pub expires_at: Option<Option<PrimitiveDateTime>>,
    pub last_used: Option<PrimitiveDateTime>,
}

impl From<ApiKeyUpdate> for ApiKeyUpdateInternal {
    fn from(api_key_update: ApiKeyUpdate) -> Self {
        match api_key_update {
            ApiKeyUpdate::Update {
                name,
                description,
                expires_at,
                last_used,
            } => Self {
                name,
                description,
                expires_at,
                last_used,
            },
            ApiKeyUpdate::LastUsedUpdate { last_used } => Self {
                last_used: Some(last_used),
                name: None,
                description: None,
                expires_at: None,
            },
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, AsExpression, PartialEq)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct HashedApiKey(String);

impl HashedApiKey {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for HashedApiKey {
    fn from(hashed_api_key: String) -> Self {
        Self(hashed_api_key)
    }
}

mod diesel_impl {
    use diesel::{
        backend::Backend,
        deserialize::FromSql,
        serialize::{Output, ToSql},
        sql_types::Text,
        Queryable,
    };

    impl<DB> ToSql<Text, DB> for super::HashedApiKey
    where
        DB: Backend,
        String: ToSql<Text, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
            self.0.to_sql(out)
        }
    }

    impl<DB> FromSql<Text, DB> for super::HashedApiKey
    where
        DB: Backend,
        String: FromSql<Text, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            Ok(Self(String::from_sql(bytes)?))
        }
    }

    impl<DB> Queryable<Text, DB> for super::HashedApiKey
    where
        DB: Backend,
        Self: FromSql<Text, DB>,
    {
        type Row = Self;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(row)
        }
    }
}

// Tracking data by process_tracker
#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeyExpiryTrackingData {
    pub key_id: common_utils::id_type::ApiKeyId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub api_key_name: String,
    pub prefix: String,
    pub api_key_expiry: Option<PrimitiveDateTime>,
    // Days on which email reminder about api_key expiry has to be sent, prior to it's expiry.
    pub expiry_reminder_days: Vec<u8>,
}
