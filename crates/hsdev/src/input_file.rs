use std::string::String;

use toml::Value;

pub struct InputData {
    username: String,
    password: String,
    dbname: String,
    host: String,
    port: u16,
}

fn get_str_or_default(toml: &Value, key: &str) -> String {
    let value = toml.get(key);

    let str = if value.is_none() {
        eprintln!("Could not read toml field: \"{}\"", key);
        ""
    } else {
        value.unwrap().as_str().unwrap_or_default()
    };

    String::from(str)
}
fn get_int_or_default(toml: &Value, key: &str) -> i64 {
    let value = toml.get(key);

    if value.is_none() {
        eprintln!("Could not read toml field: \"{}\"", key);
        0
    } else {
        value.unwrap().as_integer().unwrap_or_default()
    }
}

impl InputData {
    pub fn read(toml: &Value) -> InputData {
        InputData {
            username: get_str_or_default(toml, "username"),
            password: get_str_or_default(toml, "password"),
            dbname: get_str_or_default(toml, "dbname"),
            host: get_str_or_default(toml, "host"),
            port: get_int_or_default(toml, "port") as u16,
        }
    }

    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }
}
