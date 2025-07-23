# Guide to Integrating a Connector

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Development Environment Setup & Configuration](#development-environment-setup--configuration)

## Introduction

This guide provides instructions on integrating a new connector with Router, from setting up the environment to implementing API interactions. In this document you’ll learn how to:

* Scaffold a new connector template
* Define Rust request/response types directly from your PSP’s JSON schema
* Implement transformers and the ConnectorIntegration trait for both standard auth and tokenization-first flows
* Enforce PII best practices (Secret wrappers, common\_utils::pii types) and robust error-handling
* Update the Control-Center (ConnectorTypes.res, ConnectorUtils.res, icons)
* Validate your connector with end-to-end tests

By the end, you’ll have a fully functional, production-ready connector—from blank slate to live in the Control-Center.

## Prerequisites

* Before you begin, ensure you’ve completed the initial setup in our [Hyperswitch Contributor Guide](https://github.com/juspay/hyperswitch/blob/main/CONTRIBUTING.md), which covers cloning, tool installation, and access.
* You should also understanding [connectors and payment methods](https://hyperswitch.io/pm-list).
* Familiarity with the Connector API you’re integrating
* A locally set up and running Router repository
* API credentials for testing (sign up for sandbox/UAT credentials on the connector’s website).
* Need help? Join the [Hyperswitch Slack Channel](https://join.slack.com/t/hyperswitch-io/shared_invite/zt-39d4w0043-CgAyb75Kn0YldNyZpd8hWA). We also have weekly office hours every Thursday at 8:00 AM PT (11:00 AM ET, 4:00 PM BST, 5:00 PM CEST, and 8:30 PM IST). Link to office hours are shared in the #general channel.

## Development Environment Setup & Configuration

This guide will walk you through your environment setup and configuration.

### Clone the Hyperswitch monorepo\*\*

```bash
  git clone git@github.com:juspay/hyperswitch.git
  cd hyperswitch
```

### Rust Environment & Dependencies Setup

Before running Hyperswitch locally, make sure your Rust environment and system dependencies are properly configured.

**Follow the guide**:
[Configure Rust and install required dependencies based on your OS](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-a-rust-environment-and-other-dependencies)

**Quick links by OS**:

* [Ubuntu-based systems](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-ubuntu-based-systems)
* [Windows (WSL2)](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-windows-ubuntu-on-wsl2)
* [Windows (native)](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-windows)
* [macOS](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-macos)

**All OS Systems**:

* [Set up database](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-the-database)

* Set up the Rust nightly toolchain installed for code formatting:

```bash
rustup toolchain install nightly
```

* Install [Protobuf](https://protobuf.dev/installation/)

If you've completed the setup, you should now have:

* ✅ Rust & Cargo
* ✅ PostgreSQL (with a user and database created)
* ✅ Redis
* ✅ `diesel_cli`
* ✅ The `just` command runner
* ✅ Database migrations applied
* ✅ Set up the Rust nightly toolchain
* ✅ Installed Protobuf

You're ready to run Hyperswitch:

```bash
cargo run
```

### PSP sandbox/UAT credentials

* Obtain API keys from your payment provider
* In the `hyperswitch/crates/router/tests/connectors` directory, locate `sample_auth.toml`, copy the provider lines, and save them as a new file named `auth.toml` anywhere, like the project root. For example, if you want to build a stripe connector, your `auth.toml` will look like this:

```bash
# This is an example of auth.toml file with the Stripe provider.
# Please update the code accordingly to your provider.
[stripebilling]
api_key="YOUR API KEY"
```

* Set the environment variable.

```bash
export CONNECTOR_AUTH_FILE_PATH="/path/to/your/auth.toml"
```

It's recommended that you use a `.envrc` file with **direnv** in the `cypress-tests` directory. This approach automatically loads environment variables when you enter the directory. For example:

```bash
# In cypress-tests/.envrc  
export CONNECTOR_AUTH_FILE_PATH="/absolute/path/hyperswitch/auth.toml"  
export CYPRESS_CONNECTOR="stripe"  
export CYPRESS_BASEURL="http://localhost:8080"  # if you are deploying locally
export CYPRESS_ADMINAPIKEY="test_admin" # if you are deploying locally; see [link] for more details.
```

After that, navigate back into the `cypress-tests` directory and run the below command. This approves the `.envrc` and exports your environment variable.

```bash
direnv allow
```

> **⚠️ Important Notes**
>
> * **Never commit `auth.toml`** – It contains sensitive credentials and should never be added to version control
> * **Use absolute paths** – This avoids issues when running tests from different directories
> * **Populate with real credentials** – Replace the placeholder values from the sample file with actual sandbox/UAT credentials from your payment processors

## Create a Connector
From the root of the project, generate a new connector by running the following command. Use a single-word name for your connector:

```bash
sh scripts/add_connector.sh <connector_name> <connector_base_url>
```
When you run the script, you should see that some files were created
```bash
# Done! New project created /absolute/path/hyperswitch/crates/hyperswitch_connectors/src/connectors/connectorname
```

Don't be alarmed if you see fail tests- we haven't written any tests yet. You can ignore the failures at this point.
```bash
# test result: FAILED. 0 passed; 20 failed; 0 ignored; 0 measured; 1759 filtered out; finished in 0.10s
```

## Implement Request & Response Types



