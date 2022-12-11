use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::storage::{PaymentIntent, PaymentIntentNew};

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
    pub up: String,
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
}
