use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    address::{Address, AddressNew, AddressUpdateInternal},
    errors,
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntentNew, PaymentIntentUpdate},
    refund::{Refund, RefundNew, RefundUpdate},
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    PaymentIntent, PgPooledConn,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "db_op", content = "data")]
pub enum DBOperation {
    Insert { insertable: Insertable },
    Update { updatable: Updateable },
}

impl DBOperation {
    pub fn operation<'a>(&self) -> &'a str {
        match self {
            Self::Insert { .. } => "insert",
            Self::Update { .. } => "update",
        }
    }
    pub fn table<'a>(&self) -> &'a str {
        match self {
            Self::Insert { insertable } => match insertable {
                Insertable::PaymentIntent(_) => "payment_intent",
                Insertable::PaymentAttempt(_) => "payment_attempt",
                Insertable::Refund(_) => "refund",
                Insertable::Address(_) => "address",
                Insertable::ReverseLookUp(_) => "reverse_lookup",
            },
            Self::Update { updatable } => match updatable {
                Updateable::PaymentIntentUpdate(_) => "payment_intent",
                Updateable::PaymentAttemptUpdate(_) => "payment_attempt",
                Updateable::RefundUpdate(_) => "refund",
                Updateable::AddressUpdate(_) => "address",
            },
        }
    }
}

#[derive(Debug)]
pub enum DBResult {
    PaymentIntent(Box<PaymentIntent>),
    PaymentAttempt(Box<PaymentAttempt>),
    Refund(Box<Refund>),
    Address(Box<Address>),
    ReverseLookUp(Box<ReverseLookup>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSql {
    #[serde(flatten)]
    pub op: DBOperation,
}

impl DBOperation {
    pub async fn execute(self, conn: &PgPooledConn) -> crate::StorageResult<DBResult> {
        Ok(match self {
            Self::Insert { insertable } => match insertable {
                Insertable::PaymentIntent(a) => {
                    DBResult::PaymentIntent(Box::new(a.insert(conn).await?))
                }
                Insertable::PaymentAttempt(a) => {
                    DBResult::PaymentAttempt(Box::new(a.insert(conn).await?))
                }
                Insertable::Refund(a) => DBResult::Refund(Box::new(a.insert(conn).await?)),
                Insertable::Address(addr) => DBResult::Address(Box::new(addr.insert(conn).await?)),
                Insertable::ReverseLookUp(rev) => {
                    DBResult::ReverseLookUp(Box::new(rev.insert(conn).await?))
                }
            },
            Self::Update { updatable } => match updatable {
                Updateable::PaymentIntentUpdate(a) => {
                    DBResult::PaymentIntent(Box::new(a.orig.update(conn, a.update_data).await?))
                }
                Updateable::PaymentAttemptUpdate(a) => DBResult::PaymentAttempt(Box::new(
                    a.orig.update_with_attempt_id(conn, a.update_data).await?,
                )),
                Updateable::RefundUpdate(a) => {
                    DBResult::Refund(Box::new(a.orig.update(conn, a.update_data).await?))
                }
                Updateable::AddressUpdate(a) => {
                    DBResult::Address(Box::new(a.orig.update(conn, a.update_data).await?))
                }
            },
        })
    }
}

impl TypedSql {
    pub fn to_field_value_pairs(
        &self,
        request_id: String,
        global_id: String,
    ) -> crate::StorageResult<Vec<(&str, String)>> {
        let pushed_at = common_utils::date_time::now_unix_timestamp();

        Ok(vec![
            (
                "typed_sql",
                serde_json::to_string(self)
                    .into_report()
                    .change_context(errors::DatabaseError::QueryGenerationFailed)?,
            ),
            ("global_id", global_id),
            ("request_id", request_id),
            ("pushed_at", pushed_at.to_string()),
        ])
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "table", content = "data")]
pub enum Insertable {
    PaymentIntent(PaymentIntentNew),
    PaymentAttempt(PaymentAttemptNew),
    Refund(RefundNew),
    Address(Box<AddressNew>),
    ReverseLookUp(ReverseLookupNew),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "table", content = "data")]
pub enum Updateable {
    PaymentIntentUpdate(PaymentIntentUpdateMems),
    PaymentAttemptUpdate(PaymentAttemptUpdateMems),
    RefundUpdate(RefundUpdateMems),
    AddressUpdate(Box<AddressUpdateMems>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressUpdateMems {
    pub orig: Address,
    pub update_data: AddressUpdateInternal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentIntentUpdateMems {
    pub orig: PaymentIntent,
    pub update_data: PaymentIntentUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentAttemptUpdateMems {
    pub orig: PaymentAttempt,
    pub update_data: PaymentAttemptUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefundUpdateMems {
    pub orig: Refund,
    pub update_data: RefundUpdate,
}
