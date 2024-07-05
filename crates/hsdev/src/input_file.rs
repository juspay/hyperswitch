use std::string::String;

use serde::Deserialize;
use toml::Value;

#[derive(Deserialize)]
pub struct InputData {
    username: String,
    password: String,
    dbname: String,
    host: String,
    port: u16,
}

impl InputData {
    pub fn read(db_table: &Value) -> InputData {
        db_table
            .clone()
            .try_into::<InputData>()
            .expect("Unable to read InputData")
    }

    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }
}
