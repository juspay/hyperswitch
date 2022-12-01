use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{schema::payment_intent, types::enums};

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "diesel",
    derive(Identifiable, Queryable, Serialize, Deserialize)
)]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_intent))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PaymentIntent {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: enums::IntentStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    pub amount_captured: Option<i32>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>, // FIXME: this is optional
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
}

#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for PaymentIntent {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.id);
        encoder.encode(&self.payment_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.status);
        encoder.encode(&self.amount);
        encoder.encode(&self.currency);
        encoder.encode(&self.amount_captured);
        encoder.encode(&self.customer_id);
        encoder.encode(&self.description);
        encoder.encode(&self.return_url);
        encoder.encode(&self.metadata);
        encoder.encode(&self.connector_id);
        encoder.encode(&self.shipping_address_id);
        encoder.encode(&self.billing_address_id);
        encoder.encode(&self.statement_descriptor_name);
        encoder.encode(&self.statement_descriptor_suffix);
        encoder.encode(&self.created_at);
        encoder.encode(&self.modified_at);
        encoder.encode(&self.last_synced);
        encoder.encode(&self.setup_future_usage);
        encoder.encode(&self.off_session);
        encoder.encode(&self.client_secret);
        encoder.finish();
        sqlx::encode::IsNull::No
    }
}

impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for PaymentIntent {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let id = decoder.try_decode()?;
        let payment_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let status = decoder.try_decode()?;
        let amount = decoder.try_decode()?;
        let currency = decoder.try_decode()?;
        let amount_captured = decoder.try_decode()?;
        let customer_id = decoder.try_decode()?;
        let description = decoder.try_decode()?;
        let return_url = decoder.try_decode()?;
        let metadata = decoder.try_decode()?;
        let connector_id = decoder.try_decode()?;
        let shipping_address_id = decoder.try_decode()?;
        let billing_address_id = decoder.try_decode()?;
        let statement_descriptor_name = decoder.try_decode()?;
        let statement_descriptor_suffix = decoder.try_decode()?;
        let created_at = decoder.try_decode()?;
        let modified_at = decoder.try_decode()?;
        let last_synced = decoder.try_decode()?;
        let setup_future_usage = decoder.try_decode()?;
        let off_session = decoder.try_decode()?;
        let client_secret = decoder.try_decode()?;

        Ok(PaymentIntent {
            id,
            payment_id,
            merchant_id,
            status,
            amount,
            currency,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            connector_id,
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name,
            statement_descriptor_suffix,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage,
            off_session,
            client_secret,
        })
    }
}

impl sqlx::Type<sqlx::Postgres> for PaymentIntent {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("PaymentIntent")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_intent))]
pub struct PaymentIntentNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub status: enums::IntentStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    pub amount_captured: Option<i32>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub last_synced: Option<PrimitiveDateTime>,
    pub client_secret: Option<String>,
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
}

#[allow(clippy::needless_borrow)]
impl PaymentIntentNew {
    fn insert_query(&self, table: &str) -> String {
        let sqlquery = format!("insert into {} ( {} ) values ( {} ) returning *",table,"payment_id , merchant_id , status , amount , currency , amount_captured , customer_id , description , return_url , metadata , connector_id , shipping_address_id , billing_address_id , statement_descriptor_name , statement_descriptor_suffix , created_at , modified_at , last_synced , client_secret , setup_future_usage , off_session","$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21");
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
            .bind(&self.payment_id)
            .bind(&self.merchant_id)
            .bind(&self.status)
            .bind(&self.amount)
            .bind(&self.currency)
            .bind(&self.amount_captured)
            .bind(&self.customer_id)
            .bind(&self.description)
            .bind(&self.return_url)
            .bind(&self.metadata)
            .bind(&self.connector_id)
            .bind(&self.shipping_address_id)
            .bind(&self.billing_address_id)
            .bind(&self.statement_descriptor_name)
            .bind(&self.statement_descriptor_suffix)
            .bind(&self.created_at)
            .bind(&self.modified_at)
            .bind(&self.last_synced)
            .bind(&self.client_secret)
            .bind(&self.setup_future_usage)
            .bind(&self.off_session)
            .fetch_one(pool)
            .await
    }
}

#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for PaymentIntentNew {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.payment_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.status);
        encoder.encode(&self.amount);
        encoder.encode(&self.currency);
        encoder.encode(&self.amount_captured);
        encoder.encode(&self.customer_id);
        encoder.encode(&self.description);
        encoder.encode(&self.return_url);
        encoder.encode(&self.metadata);
        encoder.encode(&self.connector_id);
        encoder.encode(&self.shipping_address_id);
        encoder.encode(&self.billing_address_id);
        encoder.encode(&self.statement_descriptor_name);
        encoder.encode(&self.statement_descriptor_suffix);
        encoder.encode(&self.created_at);
        encoder.encode(&self.modified_at);
        encoder.encode(&self.last_synced);
        encoder.encode(&self.client_secret);
        encoder.encode(&self.setup_future_usage);
        encoder.encode(&self.off_session);
        encoder.finish();

        sqlx::encode::IsNull::No
    }
}

impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for PaymentIntentNew {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let payment_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let status = decoder.try_decode()?;
        let amount = decoder.try_decode()?;
        let currency = decoder.try_decode()?;
        let amount_captured = decoder.try_decode()?;
        let customer_id = decoder.try_decode()?;
        let description = decoder.try_decode()?;
        let return_url = decoder.try_decode()?;
        let metadata = decoder.try_decode()?;
        let connector_id = decoder.try_decode()?;
        let shipping_address_id = decoder.try_decode()?;
        let billing_address_id = decoder.try_decode()?;
        let statement_descriptor_name = decoder.try_decode()?;
        let statement_descriptor_suffix = decoder.try_decode()?;
        let created_at = decoder.try_decode()?;
        let modified_at = decoder.try_decode()?;
        let last_synced = decoder.try_decode()?;
        let client_secret = decoder.try_decode()?;
        let setup_future_usage = decoder.try_decode()?;
        let off_session = decoder.try_decode()?;

        Ok(PaymentIntentNew {
            payment_id,
            merchant_id,
            status,
            amount,
            currency,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            connector_id,
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name,
            statement_descriptor_suffix,
            created_at,
            modified_at,
            last_synced,
            client_secret,
            setup_future_usage,
            off_session,
        })
    }
}

impl sqlx::Type<sqlx::Postgres> for PaymentIntentNew {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("PaymentIntentNew")
    }
}

#[derive(Debug, Clone)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: enums::IntentStatus,
        amount_captured: Option<i32>,
        return_url: Option<String>,
    },
    MetadataUpdate {
        metadata: serde_json::Value,
    },
    ReturnUrlUpdate {
        return_url: Option<String>,
        status: Option<enums::IntentStatus>,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    MerchantStatusUpdate {
        status: enums::IntentStatus,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    PGStatusUpdate {
        status: enums::IntentStatus,
    },
    Update {
        amount: i32,
        currency: enums::Currency,
        status: enums::IntentStatus,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_intent))]

pub struct PaymentIntentUpdateInternal {
    pub amount: Option<i32>,
    pub currency: Option<enums::Currency>,
    pub status: Option<enums::IntentStatus>,
    pub amount_captured: Option<i32>,
    pub customer_id: Option<String>,
    pub return_url: Option<String>,
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub client_secret: Option<Option<String>>,
    pub billing_address_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub modified_at: Option<PrimitiveDateTime>,
}

impl PaymentIntentUpdate {
    pub fn apply_changeset(self, source: PaymentIntent) -> PaymentIntent {
        let internal_update: PaymentIntentUpdateInternal = self.into();
        PaymentIntent {
            amount: internal_update.amount.unwrap_or(source.amount),
            currency: internal_update.currency.or(source.currency),
            status: internal_update.status.unwrap_or(source.status),
            amount_captured: internal_update.amount_captured.or(source.amount_captured),
            customer_id: internal_update.customer_id.or(source.customer_id),
            return_url: internal_update.return_url.or(source.return_url),
            setup_future_usage: internal_update
                .setup_future_usage
                .or(source.setup_future_usage),
            off_session: internal_update.off_session.or(source.off_session),
            metadata: internal_update.metadata.or(source.metadata),
            client_secret: internal_update
                .client_secret
                .unwrap_or(source.client_secret),
            billing_address_id: internal_update
                .billing_address_id
                .or(source.billing_address_id),
            shipping_address_id: internal_update
                .shipping_address_id
                .or(source.shipping_address_id),
            modified_at: common_utils::date_time::now(),
            ..source
        }
    }
}

impl From<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn from(payment_intent_update: PaymentIntentUpdate) -> Self {
        match payment_intent_update {
            PaymentIntentUpdate::Update {
                amount,
                currency,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                status: Some(status),
                customer_id,
                client_secret: make_client_secret_null_if_success(Some(status)),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::MetadataUpdate { metadata } => Self {
                metadata: Some(metadata),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self {
                return_url,
                status,
                client_secret: make_client_secret_null_if_success(status),
                customer_id,
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::PGStatusUpdate { status } => Self {
                status: Some(status),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            } => Self {
                status: Some(status),
                client_secret: make_client_secret_null_if_success(Some(status)),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ResponseUpdate {
                // amount,
                // currency,
                status,
                amount_captured,
                // customer_id,
                return_url,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                // customer_id,
                return_url,
                client_secret: make_client_secret_null_if_success(Some(status)),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
        }
    }
}

fn make_client_secret_null_if_success(
    status: Option<enums::IntentStatus>,
) -> Option<Option<String>> {
    if status == Some(enums::IntentStatus::Succeeded) {
        Option::<Option<String>>::Some(None)
    } else {
        None
    }
}
