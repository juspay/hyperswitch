use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "db_op", content = "data")]
pub enum DBOperation<T> {
    Insert(InsertData<T>),
    Update(UpdateData),
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertData<T> {
    pub insertable: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateData {
    pub up: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSql<T> {
    #[serde(flatten)]
    pub op: DBOperation<T>,
}

impl<T> TypedSql<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    pub fn to_field_value_pairs(&self) -> Vec<(&str, String)> {
        vec![("typedsql", serde_json::to_string(self).unwrap())]
    }
}
