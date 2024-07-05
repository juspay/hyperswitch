use clap::{Parser, ValueHint};
use diesel::{pg::PgConnection, Connection};
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationHarness};
use toml::Value;

mod input_file;
mod tests;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    toml_file: std::path::PathBuf,

    #[arg(long, default_value_t = String::from(""))]
    toml_table: String,
}

fn main() {
    let args = Args::parse();

    let toml_file = &args.toml_file;
    let table_name = &args.toml_table;
    let toml_contents = match std::fs::read_to_string(toml_file) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Error reading TOML file: {}", e);
            return;
        }
    };
    let toml_data: Value = match toml_contents.parse() {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing TOML file: {}", e);
            return;
        }
    };

    let table = get_toml_table(table_name, &toml_data);

    let input = match input_file::InputData::read(table) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error loading TOML file: {}", e);
            return;
        }
    };

    let db_url = input.postgres_url();

    println!("Attempting to connect to {}", db_url);

    let mut conn = match PgConnection::establish(&db_url) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Unable to establish database connection");
            return;
        }
    };

    let migrations = match FileBasedMigrations::find_migrations_directory() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Could not find migrations directory");
            return;
        }
    };

    let mut harness = HarnessWithOutput::write_to_stdout(&mut conn);

    match harness.run_pending_migrations(migrations) {
        Ok(_) => println!("Successfully ran migrations"),
        Err(_) => eprintln!("Couldn't run migrations"),
    };
}

pub fn get_toml_table<'a>(table_name: &'a str, toml_data: &'a Value) -> &'a Value {
    if !table_name.is_empty() {
        match toml_data.get(table_name) {
            Some(value) => value,
            None => {
                eprintln!("Unable to find toml table: \"{}\"", &table_name);
                std::process::abort()
            }
        }
    } else {
        toml_data
    }
}
