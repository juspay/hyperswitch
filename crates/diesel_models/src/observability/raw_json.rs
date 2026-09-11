use std::io::Write;

use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Json,
};
use serde_json::value::RawValue;

// `product`/`values_` are `json`, not `jsonb`:
#[derive(Debug, Clone, AsExpression, FromSqlRow, serde::Serialize, serde::Deserialize)]
#[diesel(sql_type = Json)]
#[serde(transparent)]
pub struct RawJson(Box<RawValue>);

impl RawJson {
    pub fn get(&self) -> &str {
        self.0.get()
    }

    pub fn len(&self) -> usize {
        self.0.get().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.get().is_empty()
    }

    pub fn into_raw(self) -> Box<RawValue> {
        self.0
    }
}

impl From<Box<RawValue>> for RawJson {
    fn from(value: Box<RawValue>) -> Self {
        Self(value)
    }
}

impl FromSql<Json, Pg> for RawJson {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let text = std::str::from_utf8(value.as_bytes())?;
        Ok(Self(RawValue::from_string(text.to_owned())?))
    }
}

impl ToSql<Json, Pg> for RawJson {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.0.get().as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
