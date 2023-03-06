# Try out hyperswitch on your system

**NOTE:**
This guide is aimed at users and developers who wish to set up hyperswitch on
their local systems and requires quite some time and effort.
If you'd prefer trying out hyperswitch quickly without the hassle of setting up
all dependencies, you can [try out hyperswitch sandbox environment][try-sandbox].

There are two options to set up hyperswitch on your system:

1. Use Docker Compose
2. Set up a Rust environment and other dependencies on your system

Check the Table Of Contents to jump to the relevant section.

[try-sandbox]: ./try_sandbox.md

**Table Of Contents:**

- [Set up hyperswitch using Docker Compose](#set-up-hyperswitch-using-docker-compose)
- [Set up a Rust environment and other dependencies](#set-up-a-rust-environment-and-other-dependencies)
  - [Set up dependencies on Ubuntu-based systems](#set-up-dependencies-on-ubuntu-based-systems)
  - [Set up dependencies on Windows](#set-up-dependencies-on-windows)
  - [Set up dependencies on MacOS](#set-up-dependencies-on-macos)
  - [Set up the database](#set-up-the-database)
  - [Configure the application](#configure-the-application)
  - [Run the application](#run-the-application)
- [Try out our APIs](#try-out-our-apis)
  - [Set up your merchant account](#set-up-your-merchant-account)
  - [Set up a payment connector account](#set-up-a-payment-connector-account)
  - [Create a Payment](#create-a-payment)
  - [Create a Refund](#create-a-refund)

## Set up hyperswitch using Docker Compose

1. Install [Docker Compose][docker-compose-install].
2. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hyperswitch
   cd hyperswitch
   ```

3. (Optional) Configure the application using the
   [`config/docker_compose.toml`][docker-compose-config] file.
   The provided configuration should work as is.
   If you do update the `docker_compose.toml` file, ensure to also update the
   corresponding values in the [`docker-compose.yml`][docker-compose-yml] file.
4. Start all the services using Docker Compose:

   ```shell
   docker compose up -d
   ```

5. Run database migrations:

   ```shell
   docker compose run hyperswitch-server bash -c \
      "cargo install diesel_cli && \
      diesel migration --database-url postgres://db_user:db_pass@pg:5432/hyperswitch_db run"
   ```

6. Verify that the server is up and running by hitting the health endpoint:

   ```shell
   curl --head --request GET 'http://localhost:8080/health'
   ```

   If the command returned a `200 OK` status code, proceed with
   [trying out our APIs](#try-out-our-apis).

[docker-compose-install]: https://docs.docker.com/compose/install/
[docker-compose-config]: /config/docker_compose.toml
[docker-compose-yml]: /docker-compose.yml

## Set up a Rust environment and other dependencies

If you are using `nix`, please skip the setup dependencies step and jump to 
[Set up the database](#set-up-the-database).

### Set up dependencies on Ubuntu-based systems

This section of the guide provides instructions to install dependencies on
Ubuntu-based systems.
If you're running another Linux distribution, install the corresponding packages
for your distribution and follow along.

1. Install the stable Rust toolchain using `rustup`:

   ```shell
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   When prompted, proceed with the `default` profile, which installs the stable
   toolchain.

   Optionally, verify that the Rust compiler and `cargo` are successfully
   installed:

   ```shell
   rustc --version
   ```

   _Be careful when running shell scripts downloaded from the Internet.
   We only suggest running this script as there seems to be no `rustup` package
   available in the Ubuntu package repository._

2. Install PostgreSQL and start the `postgresql` systemd service:

   ```shell
   sudo apt update
   sudo apt install postgresql postgresql-contrib libpq-dev
   systemctl start postgresql.service
   ```

   If you're running any other distribution than Ubuntu, you can follow the
   installation instructions on the
   [PostgreSQL documentation website][postgresql-install] to set up PostgreSQL
   on your system.

3. Install Redis and start the `redis` systemd service:

   ```shell
   sudo apt install redis-server
   systemctl start redis.service
   ```

   If you're running a distribution other than Ubuntu, you can follow the
   installation instructions on the [Redis website][redis-install] to set up
   Redis on your system.

4. Install `diesel_cli` using `cargo`:

   ```shell
   cargo install diesel_cli --no-default-features --features "postgres"
   ```

5. Make sure your system has OpenSSL installed:

   ```shell
   sudo apt install libssl-dev
   ```

Once you're done with setting up the dependencies, proceed with
[setting up the database](#set-up-the-database).

[postgresql-install]: https://www.postgresql.org/download/
[redis-install]: https://redis.io/docs/getting-started/installation/

### Set up dependencies on Windows

We'll be using [`winget`][winget] in this section of the guide, where possible.
You can opt to use your favorite package manager instead.

1. Install PostgreSQL database, following the
   [official installation docs][postgresql-install-windows].

2. Install Redis, following the
   [official installation docs][redis-install-windows].

3. Install rust with `winget`:

   ```shell
   winget install -e --id Rustlang.Rust.GNU
   ```

4. Install `diesel_cli` using `cargo`:

   ```shell
   cargo install diesel_cli --no-default-features --features "postgres"
   ```

5. Install OpenSSL with `winget`:

   ```shell
   winget install openssl
   ```

[winget]: https://github.com/microsoft/winget-cli

Once you're done with setting up the database, proceed with
[configuring the application](#configure-the-application).

[postgresql-install-windows]: https://www.postgresql.org/download/windows/
[redis-install-windows]: https://redis.io/docs/getting-started/installation/install-redis-on-windows

### Set up dependencies on MacOS

We'll be using [Homebrew][homebrew] in this section of the guide.
You can opt to use your favorite package manager instead.

1. Install the stable Rust toolchain using `rustup`:

   ```shell
   brew install rustup-init
   rustup-init
   ```

   When prompted, proceed with the `default` profile, which installs the stable
   toolchain.

   Optionally, verify that the Rust compiler and `cargo` are successfully
   installed:

   ```shell
   rustc --version
   ```

2. Install PostgreSQL and start the `postgresql` service:

   ```shell
   brew install postgresql@14
   brew services start postgresql@14
   ```

   If a `postgres` database user was not already created, you may have to create
   one:

   ```shell
   createuser -s postgres
   ```

3. Install Redis and start the `redis` service:

   ```shell
   brew install redis
   brew services start redis
   ```

4. Install `diesel_cli` using `cargo`:

   ```shell
   cargo install diesel_cli --no-default-features --features "postgres"
   ```

   If linking `diesel_cli` fails due to missing `libpq` (if the error message is
   along the lines of `cannot find -lpq`), you may also have to install `libpq`
   and reinstall `diesel_cli`:

   ```shell
   brew install libpq
   export PQ_LIB_DIR="$(brew --prefix libpq)/lib"

   cargo install diesel_cli --no-default-features --features "postgres"
   ```

   You may also choose to persist the value of `PQ_LIB_DIR` in your shell
   startup file like so:

   ```shell
   echo 'PQ_LIB_DIR="$(brew --prefix libpq)/lib"' >> ~/.zshrc
   ```

Once you're done with setting up the dependencies, proceed with
[setting up the database](#set-up-the-database).

[homebrew]: https://brew.sh/

### Set up the database

1. Create the database and database users, modifying the database user
   credentials and database name as required.

   ```shell
   export DB_USER="db_user"
   export DB_PASS="db_pass"
   export DB_NAME="hyperswitch_db"
   ```

   On Ubuntu-based systems:

   ```shell
   sudo -u postgres psql -e -c \
      "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;"
   sudo -u postgres psql -e -c \
      "CREATE DATABASE $DB_NAME;"
   ```

   On MacOS:

   ```shell
   psql -e -U postgres -c \
      "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;"
   psql -e -U postgres -c \
      "CREATE DATABASE $DB_NAME"
   ```

2. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hyperswitch
   cd hyperswitch
   ```

3. Run database migrations using `diesel_cli`:

   ```shell
   diesel migration --database-url postgres://$DB_USER:$DB_PASS@localhost:5432/$DB_NAME run
   ```

Once you're done with setting up the database, proceed with
[configuring the application](#configure-the-application).

### Configure the application

The application configuration files are present under the
[`config`][config-directory] directory.

The configuration file read varies with the environment:

- Development: [`config/Development.toml`][config-development]
- Sandbox: `config/Sandbox.toml`
- Production: `config/Production.toml`

Refer to [`config.example.toml`][config-example] for all the available
configuration options.
Refer to [`Development.toml`][config-development] for the recommended defaults for
local development.

Ensure to update the [`Development.toml`][config-development] file if you opted
to use different database credentials as compared to the sample ones included in
this guide.

Once you're done with configuring the application, proceed with
[running the application](#run-the-application).

[config-directory]: /config
[config-development]: /config/Development.toml
[config-example]: /config/config.example.toml
[config-docker-compose]: /config/docker_compose.toml

### Run the application

1. Compile and run the application using `cargo`:

   ```shell
   cargo run
   ```

   If you are using `nix`, you can compile and run the application using `nix`:

   ```shell
   nix run
   ```

2. Verify that the server is up and running by hitting the health endpoint:

   ```shell
   curl --head --request GET 'http://localhost:8080/health'
   ```

   If the command returned a `200 OK` status code, proceed with
   [trying out our APIs](#try-out-our-apis).

## Try out our APIs

### Set up your merchant account

1. Sign up or sign in to [Postman][postman].
2. Open our [Postman collection][postman-collection] and switch to the
   ["Variables" tab][variables].
   Update the value under the "current value" column for the `baseUrl` variable
   to have the hostname and port of the locally running server
   (`http://localhost:8080` by default).

3. While on the "Variables" tab, add the admin API key you configured in the
   application configuration under the "current value" column for the
   `admin_api_key` variable.

   1. If you're running Docker Compose, you can find the configuration file at
      [`config/docker_compose.toml`][config-docker-compose], search for
      `admin_api_key` to find the admin API key.
   2. If you set up the dependencies locally, you can find the configuration
      file at [`config/Development.toml`][config-development], search for
      `admin_api_key` to find the admin API key

4. Open the ["Quick Start" folder][quick-start] in the collection.
5. Open the ["Merchant Account - Create"][merchant-account-create] request,
   switch to the "Body" tab and update any request parameters as required.

   - If you want to use a different connector for making payments with
     than the provided default, update the `data` field present
     in the `routing_algorithm` field to your liking.

   Click on the "Send" button to create a merchant account.
   You should obtain a response containing most of the data included in the
   request, along with some additional fields.
   Store the merchant ID, API key and publishable key returned in the response
   securely.

6. Open the ["Variables" tab][variables] in the
   [Postman collection][postman-collection] and add the following variables:

   1. Add the API key you obtained in the previous step under the "current value"
      column for the `api_key` variable.
   2. Add the merchant ID you obtained in the previous step under the "current
      value" column for the `merchant_id` variable.

### Set up a payment connector account

1. Sign up on the payment connector's (say Stripe, Adyen, etc.) dashboard and
   store your connector API key (and any other necessary secrets) securely.
2. Open the ["Payment Connector - Create"][payment-connector-create] request,
   switch to the "Body" tab and update any request parameters as required.

   - Pay special attention to the `connector_name` and
     `connector_account_details` fields and update them.
     You can find connector-specific details to be included in this
     [spreadsheet][connector-specific-details].
   - Open the ["Variables" tab][variables] in the
     [Postman collection][postman-collection] and set the `connector_api_key`
     variable to your connector's API key.

   Click on the "Send" button to create a payment connector account.
   You should obtain a response containing most of the data included in the
   request, along with some additional fields.

3. Follow the above steps if you'd like to add more payment connector accounts.

### Create a Payment

Ensure that you have
[set up your merchant account](#set-up-your-merchant-account) and
[set up at least one payment connector account](#set-up-a-payment-connector-account)
before trying to create a payment.

1. Open the ["Payments - Create"][payments-create] request, switch to the "Body"
   tab and update any request parameters as required.
   Click on the "Send" button to create a payment.
   If all goes well and you had provided the correct connector credentials, the
   payment should be created successfully.
   You should see the `status` field of the response body having a value of
   `succeeded` in this case.

   - If the `status` of the payment created was `requires_confirmation`, set
     `confirm` to `true` in the request body and send the request again.

2. Open the ["Payments - Retrieve"][payments-retrieve] request and click on the
   "Send" button (without modifying anything).
   This should return the payment object for the payment created in Step 2.

### Create a Refund

1. Open the ["Refunds - Create"][refunds-create] request in the
   ["Quick Start" folder][quick-start] folder and switch to the "Body" tab.
   Update the amount to be refunded, if required, and click on the "Send" button.
   This should create a refund against the last payment made for the specified
   amount.
   Check the `status` field of the response body to verify that the refund
   hasn't failed.
2. Open the ["Refunds - Retrieve"][refunds-retrieve] request and switch to the
   "Params" tab.
   Set the `id` path variable in the "Path Variables" table to the `refund_id`
   value returned in the response during the previous step.
   This should return the refund object for the refund created in the previous
   step.

That's it!
Hope you got a hang of our APIs.
To explore more of our APIs, please check the remaining folders in the
[Postman collection][postman-collection].

[postman]: https://www.postman.com
[postman-collection]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed
[variables]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed?tab=variables
[quick-start]: https://www.postman.com/hyperswitch/workspace/hyperswitch/folder/25176183-0103918c-6611-459b-9faf-354dee8e4437
[merchant-account-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-00124712-4dff-43d8-afb2-b99cdac1511d
[payment-connector-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-f9509d03-bb1b-4d86-bb63-1658da7f1be5
[payments-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-9b4ad6a8-fbdd-4919-8505-c75c83bdf9d6
[payments-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-11995c9b-8a34-4afd-a6ce-e8645693929b
[refunds-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-5b15d068-db9e-48a5-9ee9-3a70c0aac944
[refunds-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-c50c32af-5ceb-4ab6-aca7-85f6b32df9d3
[connector-specific-details]: https://docs.google.com/spreadsheets/d/e/2PACX-1vQWHLza9m5iO4Ol-tEBx22_Nnq8Mb3ISCWI53nrinIGLK8eHYmHGnvXFXUXEut8AFyGyI9DipsYaBLG/pubhtml?gid=748960791&single=true
