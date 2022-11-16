# How do I get set up?

Below are steps to get setup on debian. You'll need `sudo` privileges.

Use your favorite package manager for your favorite linux flavor.

## Prerequisites

* Install Dependencies

  * Install [rust]((https://www.rust-lang.org/)) using [rustup](https://rustup.rs/). (Checkout [rustup](https://rustup.rs/) for other ways to install). We'll use stable rust.

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

    verify that rust compiler is successfully installed

    ```bash
    rustc --version
    ```

  * Setup your favorite editor to use rust-analyzer or equivalent to improve the IDE experience.
  * Install and start postgres service.

    ```bash
    sudo apt update
    sudo apt install postgresql postgresql-contrib
    # When installation is complete the postgreSQL service should start automatically
    ```
    
    You can download and install PostgreSQL for your system by following the instructions on [the official website of PostgreSQL](https://www.postgresql.org/download/).

  * Install and start redis service.

    ```bash
    sudo apt install redis-server
    ```
    
    You can download and install Redis for your system by following the instructions on [the official website of Redis](https://redis.io/docs/getting-started/installation/).

  * Install the diesel CLI

    ```bash
    # may require libpq which may be missing
    # sudo apt install libpq-dev
    cargo install diesel_cli --no-default-features --features "postgres"
    ```

## Configuration and setup

### Database setup

Setup the necessary user/database using psql commands.

```bash
export DB_USER=<your username>
export DB_PASS=<your password>
export DB_NAME=<your db name>
sudo -u postgres psql -e -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;"
sudo -u postgres psql -e -c "CREATE DATABASE $DB_NAME;"
```

Clone the repo and switch to the application directory.

run migration using below commands. This will create the required db schema and populate some sample data to get your started.

```bash
diesel migration --database-url postgres://$DB_USER:$DB_PASS@localhost:5432/$DB_NAME run
```

Use the above database credentials in the configuration file. (`Development.toml`)
Configuration has been detailed in the following section.

### Orca config

The config files are present under the `config/` folder under the main application directory.
You can use the appropriate config file for your setup.

* Dev/Local: [Development.toml](config/Development.toml)
* Sandbox/Staging: [Sandbox.toml](config/Sandbox.toml)
* Production: [Production.toml](config/Production.toml)

Refer to [config.example.toml](config/config.example.toml) for all the available configuration options.
Refer to [Development.toml](config/Development.toml) for the recommended defaults for local development.

## Testing the application

Use `cargo` to run the application

```bash
cargo run
```

Test Juspay Router APIs using the [Postman collection](postman/collection.postman.json) pointing to your test server.

### How to run tests

The application contains many unit tests and Integration test. you can run them through cargo

```bash
cargo test
```

### Errors

None reported
