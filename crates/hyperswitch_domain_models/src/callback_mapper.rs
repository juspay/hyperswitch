use common_utils::pii;
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallBackMapper {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: pii::SecretSerdeValue,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallBackMapperNew {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: pii::SecretSerdeValue,
}
