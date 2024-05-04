# Try out hyperswitch on your system

The simplest way to run hyperswitch locally is
[with Docker Compose](#run-hyperswitch-using-docker-compose) by pulling the
latest images from Docker Hub.
However, if you're willing to modify the code and run it, or are a developer
contributing to hyperswitch, then you can either
[set up a development environment using Docker Compose](#set-up-a-development-environment-using-docker-compose),
or [set up a Rust environment on your system](#set-up-a-rust-environment-and-other-dependencies).

Check the Table Of Contents to jump to the relevant section.

**Table Of Contents:**

- [Run hyperswitch using Docker Compose](#run-hyperswitch-using-docker-compose)
  - [Run the scheduler and monitoring services](#run-the-scheduler-and-monitoring-services)
- [Set up a development environment using Docker Compose](#set-up-a-development-environment-using-docker-compose)
- [Set up a Rust environment and other dependencies](#set-up-a-rust-environment-and-other-dependencies)
  - [Set up dependencies on Ubuntu-based systems](#set-up-dependencies-on-ubuntu-based-systems)
  - [Set up dependencies on Windows (Ubuntu on WSL2)](#set-up-dependencies-on-windows-ubuntu-on-wsl2)
  - [Set up dependencies on Windows](#set-up-dependencies-on-windows)
  - [Set up dependencies on MacOS](#set-up-dependencies-on-macos)
  - [Set up the database](#set-up-the-database)
  - [Configure the application](#configure-the-application)
  - [Run the application](#run-the-application)
- [Try out our APIs](#try-out-our-apis)
  - [Set up your merchant account](#set-up-your-merchant-account)
  - [Create an API key](#create-an-api-key)
  - [Set up a payment connector account](#set-up-a-payment-connector-account)
  - [Create a Payment](#create-a-payment)
  - [Create a Refund](#create-a-refund)

## Run hyperswitch using Docker Compose

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

   This should run the hyperswitch payments router, the primary component within
   hyperswitch.
   Wait for the `migration_runner` container to finish installing `diesel_cli`
   and running migrations (approximately 2 minutes) before proceeding further.
   You can also choose to
   [run the scheduler and monitoring services](#run-the-scheduler-and-monitoring-services)
   in addition to the payments router.

5. Verify that the server is up and running by hitting the health endpoint:

   ```shell
   curl --head --request GET 'http://localhost:8080/health'
   ```

   If the command returned a `200 OK` status code, proceed with
   [trying out our APIs](#try-out-our-apis).

### Run the scheduler and monitoring services

You can run the scheduler and monitoring services by specifying suitable profile
names to the above Docker Compose command.
To understand more about the hyperswitch architecture and the components
involved, check out the [architecture document][architecture].

- To run the scheduler components (consumer and producer), you can specify
  `--profile scheduler`:

  ```shell
  docker compose --profile scheduler up -d
  ```

- To run the monitoring services (Grafana, Promtail, Loki, Prometheus and Tempo),
  you can specify `--profile monitoring`:

  ```shell
  docker compose --profile monitoring up -d
  ```

  You can then access Grafana at `http://localhost:3000` and view application
  logs using the "Explore" tab, select Loki as the data source, and select the
  container to query logs from.

- You can also specify multiple profile names by specifying the `--profile` flag
  multiple times.
  To run both the scheduler components and monitoring services, the Docker
  Compose command would be:

  ```shell
  docker compose --profile scheduler --profile monitoring up -d
  ```

Once the services have been confirmed to be up and running, you can proceed with
[trying out our APIs](#try-out-our-apis)

[docker-compose-install]: https://docs.docker.com/compose/install/
[docker-compose-config]: /config/docker_compose.toml
[docker-compose-yml]: /docker-compose.yml
[architecture]: /docs/architecture.md

## Set up a development environment using Docker Compose

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
   docker compose --file docker-compose-development.yml up -d
   ```

   This will compile the payments router, the primary component within
   hyperswitch and then start it.
   Depending on the specifications of your machine, compilation can take
   around 15 minutes.

5. (Optional) You can also choose to
   [start the scheduler and/or monitoring services](#run-the-scheduler-and-monitoring-services)
   in addition to the payments router.

6. Verify that the server is up and running by hitting the health endpoint:

   ```shell
   curl --head --request GET 'http://localhost:8080/health'
   ```

   If the command returned a `200 OK` status code, proceed with
   [trying out our APIs](#try-out-our-apis).

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
   cargo install diesel_cli --no-default-features --features postgres
   ```

5. Make sure your system has the `pkg-config` package and OpenSSL installed:

   ```shell
   sudo apt install pkg-config libssl-dev
   ```

Once you're done with setting up the dependencies, proceed with
[setting up the database](#set-up-the-database).

[postgresql-install]: https://www.postgresql.org/download/
[redis-install]: https://redis.io/docs/getting-started/installation/

### Set up dependencies on Windows (Ubuntu on WSL2)

This section of the guide provides instructions to install dependencies on
Ubuntu on WSL2.
If you prefer running another Linux distribution, install the corresponding
packages for your distribution and follow along.

1. Install Ubuntu on WSL:

   ```shell
   wsl --install -d Ubuntu
   ```

   Refer to the [official installation docs][wsl-install] for more information.
   Launch the WSL instance and set up your username and password.
   The following steps assume that you are running the commands within the WSL
   shell environment.

2. Install the stable Rust toolchain using `rustup`:

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

3. Install PostgreSQL and start the `postgresql` service:

   ```shell
   sudo apt update
   sudo apt install postgresql postgresql-contrib libpq-dev
   sudo service postgresql start
   ```

   For more information, refer to the docs for
   [installing PostgreSQL on WSL][postgresql-install-wsl].
   If you're running any other distribution than Ubuntu, you can follow the
   installation instructions on the
   [PostgreSQL documentation website][postgresql-install] to set up PostgreSQL
   on your system.

4. Install Redis and start the `redis-server` service:

   ```shell
   sudo apt install redis-server
   sudo service redis-server start
   ```

   For more information, refer to the docs for
   [installing Redis on WSL][redis-install-wsl].
   If you're running a distribution other than Ubuntu, you can follow the
   installation instructions on the [Redis website][redis-install] to set up
   Redis on your system.

5. Make sure your system has the packages necessary for compiling Rust code:

   ```shell
   sudo apt install build-essential
   ```

6. Install `diesel_cli` using `cargo`:

   ```shell
   cargo install diesel_cli --no-default-features --features postgres
   ```

7. Make sure your system has the `pkg-config` package and OpenSSL installed:

   ```shell
   sudo apt install pkg-config libssl-dev
   ```

Once you're done with setting up the dependencies, proceed with
[setting up the database](#set-up-the-database).

[wsl-install]: https://learn.microsoft.com/en-us/windows/wsl/install
[postgresql-install-wsl]: https://learn.microsoft.com/en-us/windows/wsl/tutorials/wsl-database#install-postgresql
[redis-install-wsl]: https://learn.microsoft.com/en-us/windows/wsl/tutorials/wsl-database#install-redis

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
   cargo install diesel_cli --no-default-features --features postgres
   ```

5. Install OpenSSL with `winget`:

   ```shell
   winget install openssl
   ```

Once you're done with setting up the dependencies, proceed with
[setting up the database](#set-up-the-database).

[winget]: https://github.com/microsoft/winget-cli
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
   cargo install diesel_cli --no-default-features --features postgres
   ```

   If linking `diesel_cli` fails due to missing `libpq` (if the error message is
   along the lines of `cannot find -lpq`), you may also have to install `libpq`
   and reinstall `diesel_cli`:

   ```shell
   brew install libpq
   export PQ_LIB_DIR="$(brew --prefix libpq)/lib"

   cargo install diesel_cli --no-default-features --features postgres
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

   On Ubuntu-based systems (also applicable for Ubuntu on WSL2):

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

- Development: [`config/development.toml`][config-development]
- Sandbox: `config/sandbox.toml`
- Production: `config/production.toml`

Refer to [`config.example.toml`][config-example] for all the available
configuration options.
Refer to [`development.toml`][config-development] for the recommended defaults for
local development.

Ensure to update the [`development.toml`][config-development] file if you opted
to use different database credentials as compared to the sample ones included in
this guide.

Once you're done with configuring the application, proceed with
[running the application](#run-the-application).

[config-directory]: /config
[config-development]: /config/development.toml
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
      file at [`config/development.toml`][config-development], search for
      `admin_api_key` to find the admin API key

4. Open the ["Quick Start" folder][quick-start] in the collection.
5. Open the ["Merchant Account - Create"][merchant-account-create] request,
   switch to the "Body" tab and update any request parameters as required.

   - If you want to use a different connector for making payments with
     than the provided default, update the `data` field present
     in the `routing_algorithm` field to your liking.

   Click on the "Send" button to create a merchant account
   (You may need to "create a fork" to fork this collection to your own
   workspace to send a request).
   You should obtain a response containing most of the data included in the
   request, along with some additional fields.
   Store the merchant ID and publishable key returned in the response.

### Create an API key

1. Open the ["API Key - Create"][api-key-create] request, switch to the "Body"
   tab and update any request parameters as required.
   Click on the "Send" button to create an API key.
   You should obtain a response containing the data included in the request,
   along with the plaintext API key.
   Store the API key returned in the response securely.

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
[postman-collection]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/collection/25176162-630b5353-7002-44d1-8ba1-ead6c230f2e3
[variables]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/collection/25176162-630b5353-7002-44d1-8ba1-ead6c230f2e3?tab=variables
[quick-start]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/folder/25176162-0f61a2bb-f9d5-4c60-8b73-9b677bf8ebbc
[merchant-account-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-3c5d5282-931b-4adc-a651-f88c8697ebcb
[api-key-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-98ce39af-0dbc-4583-8c22-dcaa801851e0
[payment-connector-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-295d83c8-957a-4524-95c8-589a26d751cf
[payments-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-ee0549bf-dd38-41fd-9a8a-de74879f3cda
[payments-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-8baf2590-d2af-44d0-ba37-e9cab7ef891a
[refunds-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-4d1315c6-ac61-4411-8f7d-15d4e4e736a1
[refunds-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch-development/request/25176162-137d6260-24f7-4752-9e69-26b61b83df0d
[connector-specific-details]: https://docs.google.com/spreadsheets/d/e/2PACX-1vQWHLza9m5iO4Ol-tEBx22_Nnq8Mb3ISCWI53nrinIGLK8eHYmHGnvXFXUXEut8AFyGyI9DipsYaBLG/pubhtml?gid=748960791&single=true
