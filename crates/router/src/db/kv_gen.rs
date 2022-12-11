use serde::{Deserialize, Serialize};

use crate::types::storage::{
    PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentIntent, PaymentIntentNew,
    PaymentIntentUpdate,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "db_op", content = "data")]
pub enum DBOperation {
    Insert(InsertData),
    Update(UpdateData),
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertData {
    pub insertable: Insertables,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateData {
    pub updateable: Updateables,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSql {
    #[serde(flatten)]
    pub op: DBOperation,
}

impl TypedSql {
    pub fn to_field_value_pairs(&self) -> Vec<(&str, String)> {
        vec![("typedsql", serde_json::to_string(self).unwrap())]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Insertables {
    PaymentIntent(PaymentIntentNew),
    PaymentAttempt(PaymentAttemptNew),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Updateables {
    PaymentIntentUpdate(PaymentIntentUpdateMems),
    PaymentAttemptUpdate(PaymentAttemptUpdateMems),
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
