use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    address::{Address, AddressNew, AddressUpdateInternal},
    errors,
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntentNew, PaymentIntentUpdate},
    refund::{Refund, RefundNew, RefundUpdate},
    reverse_lookup::ReverseLookupNew,
    PaymentIntent,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "db_op", content = "data")]
pub enum DBOperation {
    Insert { insertable: Insertable },
    Update { updatable: Updateable },
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSql {
    #[serde(flatten)]
    pub op: DBOperation,
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
