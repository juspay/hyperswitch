#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use toml::Value;

    use crate::{get_toml_table, input_file::InputData};

    #[test]
    fn test_input_file() {
        let toml_str = r#"username = "db_user"
            password = "db_pass"
            dbname = "db_name"
            host = "localhost"
            port = 5432"#;

        let toml_value = Value::from_str(toml_str).unwrap();

        let toml_table = InputData::read(&toml_value);
        assert!(toml_table.is_ok());
        let toml_table = toml_table.unwrap();

        let db_url = toml_table.postgres_url();
        assert_eq!("postgres://db_user:db_pass@localhost:5432/db_name", db_url);
    }

    #[test]
    fn test_given_toml() {
        let toml_str_table = r#"[database]
            username = "db_user"
            password = "db_pass"
            dbname = "db_name"
            host = "localhost"
            port = 5432"#;

        let table_name = "database";
        let toml_value = Value::from_str(toml_str_table).unwrap();
        let table = get_toml_table(&table_name, &toml_value);

        assert!(table.is_table());

        let table_name = "";
        let table = get_toml_table(&table_name, &toml_value);
        assert!(table.is_table());
    }
}
