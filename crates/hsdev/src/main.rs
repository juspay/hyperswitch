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

    let table = get_toml_table(&table_name, &toml_data);

    let input = input_file::InputData::read(table);

    let db_url = input.postgres_url();

    println!("Attempting to connect to {}", db_url);

    let mut conn = PgConnection::establish(&db_url).expect("Unable to connect");

    let migrations = FileBasedMigrations::find_migrations_directory()
        .expect("Could not read migrations directory");

    let mut harness = HarnessWithOutput::write_to_stdout(&mut conn);

    harness
        .run_pending_migrations(migrations)
        .expect("Couldn't run migrations");

    println!("Successfully ran migrations")
}

pub fn get_toml_table<'a>(table_name: &'a str, toml_data: &'a Value) -> &'a Value {
    if !table_name.is_empty() {
        let table = toml_data.get(&table_name);
        if table.is_none() {
            eprintln!("Unable to find toml table: \"{}\"", &table_name);
            std::process::abort()
        }

        table.unwrap()
    } else {
        &toml_data
    }
}
