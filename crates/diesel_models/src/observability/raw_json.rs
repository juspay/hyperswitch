use std::io::Write;

use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Clone, Debug, AsExpression, FromSqlRow, Deserialize, Serialize)]
#[diesel(sql_type = Json)]
#[serde(transparent)]
pub struct RawJson(Box<RawValue>);

impl RawJson {
    pub fn get(&self) -> &str {
        self.0.get()
    }
}

impl FromSql<Json, Pg> for RawJson {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Self(serde_json::from_slice(value.as_bytes())?))
    }
}

impl ToSql<Json, Pg> for RawJson {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.0.get().as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
