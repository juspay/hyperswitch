# Guide to Integrating a Connector

## Table of Contents
1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Development Environment Setup & Configuration](#development-environment-setup--configuration)



## Introduction
This guide provides instructions on integrating a new connector with Router, from setting up the environment to implementing API interactions. In this document you’ll learn how to:

- Scaffold a new connector template
- Define Rust request/response types directly from your PSP’s JSON schema
- Implement transformers and the ConnectorIntegration trait for both standard auth and tokenization-first flows
- Enforce PII best practices (Secret wrappers, common_utils::pii types) and robust error-handling
- Update the Control-Center (ConnectorTypes.res, ConnectorUtils.res, icons)
- Validate your connector with end-to-end tests

By the end, you’ll have a fully functional, production-ready connector—from blank slate to live in the Control-Center.

## Prerequisites
- Before you begin, ensure you’ve completed the initial setup in our [Hyperswitch Contributor Guide](https://github.com/juspay/hyperswitch/blob/main/CONTRIBUTING.md), which covers cloning, tool installation, and access.
- You should also understanding [connectors and payment methods](https://hyperswitch.io/pm-list).
- Need help? Join the [Hyperswitch Slack Channel](). We also have weekly office hours every Thursday at 8:00 AM PT (11:00 AM ET, 4:00 PM BST, 5:00 PM CEST, and 8:30 PM IST). Link to office hours are shared in the #general channel.


## Development Environment Setup & Configuration
- **Clone the Hyperswitch monorepo** 
 
```bash
  git clone git@github.com:juspay/hyperswitch.git
```

### Rust installed and configured
Install Stable (for development) and Nightly (for formatting):

```bash
rustup toolchain install nightly
rustup default stable
```
### Node.js & wasm-pack
Use Node.js ≥16 for the Control-Center frontend. Install wasm-pack to build the UI:

``` bash
cargo install wasm-pack
```

### PSP sandbox/UAT credentials
- Obtain API keys from your payment provider
- In the `hyperswitch/crates/router/tests/connectors` directory, locate `sample_auth.toml`, copy the provider lines, and save them as a new file named `auth.toml` anywhere, like the project root. For example, if you want to build a stripe connector, your `auth.toml` will look like this: 

```bash
[stripebilling]
api_key="YOUR API KEY"
```

- Set the environment variable. 
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
> - **Never commit `auth.toml`** – It contains sensitive credentials and should never be added to version control  
> - **Use absolute paths** – This avoids issues when running tests from different directories  
> - **Populate with real credentials** – Replace the placeholder values from the sample file with actual sandbox/UAT credentials from your payment processors