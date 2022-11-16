# How do I get set up?

Below are steps to get setup on MacOS using `Brew`. But use your favorite package manager.

## Prequisites

We'll use [Brew](https://brew.sh/) on MacOS for simplicity.

* Install Dependencies
  * Install [rust]((https://www.rust-lang.org/)) using [rustup](https://rustup.rs/). (Checkout [rustup](https://rustup.rs/) for other ways to install). We'll use stable rust.

    ```bash
    brew install rustup
    rustup-init
    ```

    verify that rust compiler is successfully installed

    ```bash
    rustc --version
    ```

  * Setup your favorite editor to use rust-analyzer or equivalent to improve the IDE experience.
  * Install and start postgres service.

    ```bash
    brew install postgresql@14
    brew services start postgresql@14
    # You may need to create the `postgres` user if not already added.
    createuser -s postgres
    ```

  * Install and start redis service.

    ```bash
    brew install redis
    brew services start redis
    ```

  * Install the diesel CLI

    ```bash
    cargo install diesel_cli --no-default-features --features "postgres"
    ```

## Configuration and setup

### Database setup

Setup the necessary user/database using psql commands.

```bash
export DB_USER=<your username>
export DB_PASS=<your password>
export DB_NAME=<your db name>
psql -e -U postgres -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;"
psql -e -U postgres -c "CREATE DATABASE $DB_NAME"
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
You can use the appropriate config file for your environment.

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

* Compiling fails due to `libpq` missing (`-lpq` error during linking stage).
  * This implies the postgres isn't properly installed or not exposed in PATH correctly. You can workaround by installing libpq and exporting `PQ_LIB_DIR` as below. But this will lead to unnecessary re-compile of pq/diesel.

    ```bash
    # If library itself isn't present, then
    brew install libpq
    export PQ_LIB_DIR="$(brew --prefix libpq)/lib"
    # persist in your startup file
    echo "PQ_LIB_DIR=$(brew --prefix libpq)/lib" >> ~/.zshrc
    ```
