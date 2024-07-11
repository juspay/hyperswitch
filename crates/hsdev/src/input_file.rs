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
    pub fn read(db_table: &Value) -> Result<Self, toml::de::Error> {
        db_table.clone().try_into()
    }

    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }
}
