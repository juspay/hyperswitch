use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::{pii::StrongSecret, schema::merchant_account, types::enums};

#[derive(
    sqlx::FromRow,
    Clone,
    Debug,
    Eq,
    PartialEq,
    router_derive::DebugAsDisplay,
    Identifiable,
    Queryable,
)]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_account))]
pub struct MerchantAccount {
    pub id: i32,
    pub merchant_id: String,
    pub api_key: Option<StrongSecret<String>>,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<String>,
    pub merchant_details: Option<serde_json::Value>,
    pub webhook_details: Option<serde_json::Value>,
    pub routing_algorithm: Option<enums::RoutingAlgorithm>,
    pub custom_routing_rules: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub publishable_key: Option<String>,
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_account))]
pub struct MerchantAccountNew {
    pub merchant_id: String,
    pub merchant_name: Option<String>,
    pub api_key: Option<StrongSecret<String>>,
    pub merchant_details: Option<serde_json::Value>,
    pub return_url: Option<String>,
    pub webhook_details: Option<serde_json::Value>,
    pub routing_algorithm: Option<enums::RoutingAlgorithm>,
    pub custom_routing_rules: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub(crate) publishable_key: Option<String>,
}

#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for MerchantAccountNew {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.merchant_name);
        encoder.encode(&self.api_key);
        encoder.encode(&self.merchant_details);
        encoder.encode(&self.return_url);
        encoder.encode(&self.webhook_details);
        encoder.encode(&self.routing_algorithm);
        encoder.encode(&self.custom_routing_rules);
        encoder.encode(&self.sub_merchants_enabled);
        encoder.encode(&self.parent_merchant_id);
        encoder.encode(&self.enable_payment_response_hash);
        encoder.encode(&self.payment_response_hash_key);
        encoder.encode(&self.redirect_to_merchant_with_http_post);
        encoder.encode(&self.publishable_key);
        encoder.finish();
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "sqlx")]
#[allow(clippy::needless_borrow)]
impl MerchantAccountNew {
    fn insert_query(&self, table: &str) -> String {
        format!("insert into {} ({}) values ({}) returning *", table, "merchant_id, merchant_name, api_key, merchant_details, return_url, webhook_details, routing_algorithm, custom_routing_rules, sub_merchants_enabled, parent_merchant_id, enable_payment_response_hash, payment_response_hash_key, redirect_to_merchant_with_http_post, publishable_key","$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14")
    }

    pub async fn insert<T>(&self, pool: &sqlx::PgPool, table: &str) -> Result<T, sqlx::Error>
    where
        T: Send,
        T: for<'c> sqlx::FromRow<'c, sqlx::postgres::PgRow>,
        T: std::marker::Unpin,
    {
        let sql = self.insert_query(table);
        sqlx::query_as::<_, T>(&sql)
            .bind(&self.merchant_id)
            .bind(&self.merchant_name)
            .bind(&self.api_key)
            .bind(&self.merchant_details)
            .bind(&self.return_url)
            .bind(&self.webhook_details)
            .bind(&self.routing_algorithm)
            .bind(&self.custom_routing_rules)
            .bind(&self.sub_merchants_enabled)
            .bind(&self.parent_merchant_id)
            .bind(&self.enable_payment_response_hash.unwrap_or_default())
            .bind(&self.payment_response_hash_key)
            .bind(&self.redirect_to_merchant_with_http_post.unwrap_or_default())
            .bind(&self.publishable_key)
            .fetch_one(pool)
            .await
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for MerchantAccountNew {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let merchant_id = decoder.try_decode()?;
        let merchant_name = decoder.try_decode()?;
        let api_key = decoder.try_decode()?;
        let merchant_details = decoder.try_decode()?;
        let return_url = decoder.try_decode()?;
        let webhook_details = decoder.try_decode()?;
        let routing_algorithm = decoder.try_decode()?;
        let custom_routing_rules = decoder.try_decode()?;
        let sub_merchants_enabled = decoder.try_decode()?;
        let parent_merchant_id = decoder.try_decode()?;
        let enable_payment_response_hash = decoder.try_decode()?;
        let payment_response_hash_key = decoder.try_decode()?;
        let redirect_to_merchant_with_http_post = decoder.try_decode()?;
        let publishable_key = decoder.try_decode()?;

        Ok(MerchantAccountNew {
            merchant_id,
            merchant_name,
            api_key,
            merchant_details,
            return_url,
            webhook_details,
            routing_algorithm,
            custom_routing_rules,
            sub_merchants_enabled,
            parent_merchant_id,
            enable_payment_response_hash,
            payment_response_hash_key,
            redirect_to_merchant_with_http_post,
            publishable_key,
        })
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Postgres> for MerchantAccountNew {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("MerchantAccountNew")
    }
}

#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_id: String,
        merchant_name: Option<String>,
        api_key: Option<StrongSecret<String>>,
        merchant_details: Option<serde_json::Value>,
        return_url: Option<String>,
        webhook_details: Option<serde_json::Value>,
        routing_algorithm: Option<enums::RoutingAlgorithm>,
        custom_routing_rules: Option<serde_json::Value>,
        sub_merchants_enabled: Option<bool>,
        parent_merchant_id: Option<String>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        publishable_key: Option<String>,
    },
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_account))]
pub(super) struct MerchantAccountUpdateInternal {
    merchant_id: Option<String>,
    merchant_name: Option<String>,
    api_key: Option<StrongSecret<String>>,
    merchant_details: Option<serde_json::Value>,
    return_url: Option<String>,
    webhook_details: Option<serde_json::Value>,
    routing_algorithm: Option<enums::RoutingAlgorithm>,
    custom_routing_rules: Option<serde_json::Value>,
    sub_merchants_enabled: Option<bool>,
    parent_merchant_id: Option<String>,
    enable_payment_response_hash: Option<bool>,
    payment_response_hash_key: Option<String>,
    redirect_to_merchant_with_http_post: Option<bool>,
    publishable_key: Option<String>,
}

impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_id,
                merchant_name,
                api_key,
                merchant_details,
                return_url,
                webhook_details,
                routing_algorithm,
                custom_routing_rules,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
            } => Self {
                merchant_id: Some(merchant_id),
                merchant_name,
                api_key,
                merchant_details,
                return_url,
                webhook_details,
                routing_algorithm,
                custom_routing_rules,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
            },
        }
    }
}
