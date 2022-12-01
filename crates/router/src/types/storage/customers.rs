use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    core::errors::{self, RouterResult},
    pii::{self, PeekInterface, Secret},
    schema::customers,
    utils::{self, ValidateCall},
};

#[derive(Default, Clone, Debug, Deserialize, Serialize, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = customers))]
#[serde(deny_unknown_fields)]
pub struct CustomerNew {
    #[serde(default = "generate_customer_id")]
    pub customer_id: String,
    #[serde(default = "unknown_merchant", skip)]
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub description: Option<String>,
    pub phone_country_code: Option<String>,
    pub address: Option<Secret<serde_json::Value>>,
    pub metadata: Option<serde_json::Value>,
}

#[allow(clippy::needless_borrow)]
impl CustomerNew {
    fn insert_query(&self, table: &str) -> String {
        let sqlquery = format!("insert into {} ( {} ) values ( {} ) returning *",table,"customer_id , merchant_id , name , email , phone , description , phone_country_code , address , metadata","$1,$2,$3,$4,$5,$6,$7,$8,$9");
        sqlquery
    }

    pub async fn insert<T>(&self, pool: &sqlx::PgPool, table: &str) -> Result<T, sqlx::Error>
    where
        T: Send,
        T: for<'c> sqlx::FromRow<'c, sqlx::postgres::PgRow>,
        T: std::marker::Unpin,
    {
        let sql = self.insert_query(table);
        sqlx::query_as::<_, T>(&sql)
            .bind(&self.customer_id)
            .bind(&self.merchant_id)
            .bind(&self.name)
            .bind(&self.email)
            .bind(&self.phone)
            .bind(&self.description)
            .bind(&self.phone_country_code)
            .bind(&self.address)
            .bind(&self.metadata)
            .fetch_one(pool)
            .await
    }
}

#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for CustomerNew {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.customer_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.name);
        encoder.encode(&self.email);
        encoder.encode(&self.phone);
        encoder.encode(&self.description);
        encoder.encode(&self.phone_country_code);
        encoder.encode(&self.address);
        encoder.encode(&self.metadata);
        encoder.finish();

        sqlx::encode::IsNull::No
    }
}

#[allow(clippy::needless_borrow)]
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for CustomerNew {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let customer_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let name = decoder.try_decode()?;
        let email = decoder.try_decode()?;
        let phone = decoder.try_decode()?;
        let description = decoder.try_decode()?;
        let phone_country_code = decoder.try_decode()?;
        let address = decoder.try_decode()?;
        let metadata = decoder.try_decode()?;

        Ok(CustomerNew {
            customer_id,
            merchant_id,
            name,
            email,
            phone,
            description,
            phone_country_code,
            address,
            metadata,
        })
    }
}

impl sqlx::Type<sqlx::Postgres> for CustomerNew {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("CustomerNew")
    }
}

impl CustomerNew {
    // FIXME: Use ValidateVar trait?
    pub(crate) fn validate(self) -> RouterResult<Self> {
        self.email
            .as_ref()
            .validate_opt(|email| utils::validate_email(email.peek()))
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "email".to_string(),
                expected_format: "valid email address".to_string(),
            })?;
        self.address
            .as_ref()
            .validate_opt(|addr| utils::validate_address(addr.peek()))
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "address".to_string(),
                expected_format: "valid address".to_string(),
            })?;
        Ok(self)
    }
}

pub fn generate_customer_id() -> String {
    String::from("cus_") + &(Uuid::new_v4().to_string())
}

fn unknown_merchant() -> String {
    String::from("merchant_unknown")
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = customers))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Customer {
    #[serde(skip_serializing)]
    pub id: i32,
    pub customer_id: String,
    #[serde(skip_serializing)]
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub address: Option<Secret<serde_json::Value>>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<serde_json::Value>,
}

#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for Customer {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.id);
        encoder.encode(&self.customer_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.name);
        encoder.encode(&self.email);
        encoder.encode(&self.phone);
        encoder.encode(&self.phone_country_code);
        encoder.encode(&self.description);
        encoder.encode(&self.address);
        encoder.encode(&self.created_at);
        encoder.encode(&self.metadata);
        encoder.finish();
        sqlx::encode::IsNull::No
    }
}

#[allow(clippy::needless_borrow)]
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for Customer {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let id = decoder.try_decode()?;
        let customer_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let name = decoder.try_decode()?;
        let email = decoder.try_decode()?;
        let phone = decoder.try_decode()?;
        let phone_country_code = decoder.try_decode()?;
        let description = decoder.try_decode()?;
        let address = decoder.try_decode()?;
        let created_at = decoder.try_decode()?;
        let metadata = decoder.try_decode()?;

        Ok(Customer {
            id,
            customer_id,
            merchant_id,
            name,
            email,
            phone,
            phone_country_code,
            description,
            address,
            created_at,
            metadata,
        })
    }
}

impl sqlx::Type<sqlx::Postgres> for Customer {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("Customer")
    }
}

#[derive(Debug)]
pub enum CustomerUpdate {
    Update {
        name: Option<String>,
        email: Option<Secret<String, pii::Email>>,
        phone: Option<Secret<String>>,
        description: Option<String>,
        phone_country_code: Option<String>,
        address: Option<Secret<serde_json::Value>>,
        metadata: Option<serde_json::Value>,
    },
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = customers))]
pub(super) struct CustomerUpdateInternal {
    name: Option<String>,
    email: Option<Secret<String, pii::Email>>,
    phone: Option<Secret<String>>,
    description: Option<String>,
    phone_country_code: Option<String>,
    address: Option<Secret<serde_json::Value>>,
    metadata: Option<serde_json::Value>,
}

impl From<CustomerUpdate> for CustomerUpdateInternal {
    fn from(customer_update: CustomerUpdate) -> Self {
        match customer_update {
            CustomerUpdate::Update {
                name,
                email,
                phone,
                description,
                phone_country_code,
                address,
                metadata,
            } => Self {
                name,
                email,
                phone,
                description,
                phone_country_code,
                address,
                metadata,
            },
        }
    }
}
