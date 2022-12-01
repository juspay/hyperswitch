use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::{pii::Secret, schema::merchant_connector_account, types::enums};

#[derive(Clone, Debug, Eq, PartialEq, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_connector_account))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type, sqlx::FromRow))]
pub struct MerchantConnectorAccount {
    pub id: i32,
    pub merchant_id: String,
    pub connector_name: String,
    pub connector_account_details: serde_json::Value,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: i32,
    #[diesel(deserialize_as = super::OptionalDieselArray<serde_json::Value>)]
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
    pub connector_type: enums::ConnectorType,
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_connector_account))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<String>,
    pub connector_type: Option<enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Secret<serde_json::Value>>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: Option<i32>,
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
}

#[cfg(feature = "sqlx")]
#[allow(clippy::needless_borrow)]
impl MerchantConnectorAccountNew {
    fn insert_query(&self, table: &str) -> String {
        let sqlquery = format!("insert into {} ( {} ) values ( {} ) returning *",table,"merchant_id , connector_type , connector_name , connector_account_details , test_mode , disabled , merchant_connector_id , payment_methods_enabled","$1,$2,$3,$4,$5,$6,$7,$8");
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
            .bind(&self.merchant_id)
            .bind(&self.connector_type)
            .bind(&self.connector_name)
            .bind(&self.connector_account_details)
            .bind(&self.test_mode)
            .bind(&self.disabled)
            .bind(&self.merchant_connector_id.unwrap_or_default())
            .bind(&self.payment_methods_enabled)
            .fetch_one(pool)
            .await
    }
}

#[derive(Debug)]
pub enum MerchantConnectorAccountUpdate {
    Update {
        merchant_id: Option<String>,
        connector_type: Option<enums::ConnectorType>,
        connector_name: Option<String>,
        connector_account_details: Option<Secret<serde_json::Value>>,
        test_mode: Option<bool>,
        disabled: Option<bool>,
        merchant_connector_id: Option<i32>,
        payment_methods_enabled: Option<Vec<serde_json::Value>>,
    },
}
#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = merchant_connector_account))]
pub(super) struct MerchantConnectorAccountUpdateInternal {
    merchant_id: Option<String>,
    connector_type: Option<enums::ConnectorType>,
    connector_name: Option<String>,
    connector_account_details: Option<Secret<serde_json::Value>>,
    test_mode: Option<bool>,
    disabled: Option<bool>,
    merchant_connector_id: Option<i32>,
    payment_methods_enabled: Option<Vec<serde_json::Value>>,
}

impl From<MerchantConnectorAccountUpdate> for MerchantConnectorAccountUpdateInternal {
    fn from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                merchant_id,
                connector_type,
                connector_name,
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
            } => Self {
                merchant_id,
                connector_type,
                connector_name,
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
            },
        }
    }
}
