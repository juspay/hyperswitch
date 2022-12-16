use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    errors,
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntent, PaymentIntentNew, PaymentIntentUpdate},
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
    pub fn to_field_value_pairs(&self) -> crate::StorageResult<Vec<(&str, String)>> {
        Ok(vec![(
            "typed_sql",
            serde_json::to_string(self)
                .into_report()
                .change_context(errors::DatabaseError::QueryGenerationFailed)?,
        )])
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "table", content = "data")]
pub enum Insertable {
    PaymentIntent(PaymentIntentNew),
    PaymentAttempt(PaymentAttemptNew),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "table", content = "data")]
pub enum Updateable {
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
