use diesel::{Identifiable, Insertable, Queryable};
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::schema::temp_card;

#[derive(Clone, Debug, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Queryable, Identifiable, Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = temp_card))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type, sqlx::FromRow))]
pub struct TempCard {
    pub id: i32,
    pub date_created: PrimitiveDateTime,
    pub txn_id: Option<String>,
    pub card_info: Option<Value>,
}

#[derive(Clone, Debug, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = temp_card))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct TempCardNew {
    pub id: Option<i32>,
    pub card_info: Option<Value>,
    pub date_created: PrimitiveDateTime,
    pub txn_id: Option<String>,
}

impl TempCardNew {
    fn insert_query(&self, table: &str) -> String {
        let sqlquery = format!(
            "insert into {} ( {} ) values ( {} ) returning *",
            table, " card_info , date_created , txn_id", "$1,$2,$3"
        );
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
            .bind(&self.card_info)
            .bind(&self.date_created)
            .bind(&self.txn_id)
            .fetch_one(pool)
            .await
    }
}
