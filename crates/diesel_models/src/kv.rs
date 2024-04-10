use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    address::{Address, AddressNew, AddressUpdateInternal},
    errors,
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntentNew, PaymentIntentUpdate},
    payout_attempt::{PayoutAttempt, PayoutAttemptNew, PayoutAttemptUpdate},
    payouts::{Payouts, PayoutsNew, PayoutsUpdate},
    refund::{Refund, RefundNew, RefundUpdate},
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    Mandate, MandateNew, MandateUpdateInternal, PaymentIntent, PaymentMethod, PaymentMethodNew,
    PaymentMethodUpdateInternal, PgPooledConn,
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
                Insertable::Payouts(_) => "payouts",
                Insertable::PayoutAttempt(_) => "payout_attempt",
                Insertable::ReverseLookUp(_) => "reverse_lookup",
                Insertable::PaymentMethod(_) => "payment_method",
                Insertable::Mandate(_) => "mandate",
            },
            Self::Update { updatable } => match updatable {
                Updateable::PaymentIntentUpdate(_) => "payment_intent",
                Updateable::PaymentAttemptUpdate(_) => "payment_attempt",
                Updateable::RefundUpdate(_) => "refund",
                Updateable::AddressUpdate(_) => "address",
                Updateable::PayoutsUpdate(_) => "payouts",
                Updateable::PayoutAttemptUpdate(_) => "payout_attempt",
                Updateable::PaymentMethodUpdate(_) => "payment_method",
                Updateable::MandateUpdate(_) => " mandate",
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
    Payouts(Box<Payouts>),
    PayoutAttempt(Box<PayoutAttempt>),
    PaymentMethod(Box<PaymentMethod>),
    Mandate(Box<Mandate>),
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
                Insertable::Payouts(rev) => DBResult::Payouts(Box::new(rev.insert(conn).await?)),
                Insertable::PayoutAttempt(rev) => {
                    DBResult::PayoutAttempt(Box::new(rev.insert(conn).await?))
                }
                Insertable::PaymentMethod(rev) => {
                    DBResult::PaymentMethod(Box::new(rev.insert(conn).await?))
                }
                Insertable::Mandate(m) => DBResult::Mandate(Box::new(m.insert(conn).await?)),
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
                Updateable::PayoutsUpdate(a) => {
                    DBResult::Payouts(Box::new(a.orig.update(conn, a.update_data).await?))
                }
                Updateable::PayoutAttemptUpdate(a) => DBResult::PayoutAttempt(Box::new(
                    a.orig.update_with_attempt_id(conn, a.update_data).await?,
                )),
                Updateable::PaymentMethodUpdate(v) => DBResult::PaymentMethod(Box::new(
                    v.orig
                        .update_with_payment_method_id(conn, v.update_data)
                        .await?,
                )),
                Updateable::MandateUpdate(m) => DBResult::Mandate(Box::new(
                    Mandate::update_by_merchant_id_mandate_id(
                        conn,
                        &m.orig.merchant_id,
                        &m.orig.mandate_id,
                        m.update_data,
                    )
                    .await?,
                )),
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
    Payouts(PayoutsNew),
    PayoutAttempt(PayoutAttemptNew),
    PaymentMethod(PaymentMethodNew),
    Mandate(MandateNew),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "table", content = "data")]
pub enum Updateable {
    PaymentIntentUpdate(PaymentIntentUpdateMems),
    PaymentAttemptUpdate(PaymentAttemptUpdateMems),
    RefundUpdate(RefundUpdateMems),
    AddressUpdate(Box<AddressUpdateMems>),
    PayoutsUpdate(PayoutsUpdateMems),
    PayoutAttemptUpdate(PayoutAttemptUpdateMems),
    PaymentMethodUpdate(PaymentMethodUpdateMems),
    MandateUpdate(MandateUpdateMems),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PayoutsUpdateMems {
    pub orig: Payouts,
    pub update_data: PayoutsUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayoutAttemptUpdateMems {
    pub orig: PayoutAttempt,
    pub update_data: PayoutAttemptUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethodUpdateMems {
    pub orig: PaymentMethod,
    pub update_data: PaymentMethodUpdateInternal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MandateUpdateMems {
    pub orig: Mandate,
    pub update_data: MandateUpdateInternal,
}
