# ORCA

[![Build Status][actions-badge]][actions-url]
[![Apache 2.0 license][license-badge]][license-url]

[actions-badge]: https://github.com/juspay/orca/workflows/CI/badge.svg
[actions-url]: https://github.com/juspay/orca/actions?query=workflow%3ACI+branch%3Amain
[license-badge]: https://img.shields.io/github/license/juspay/orca
[license-url]: https://github.com/juspay/orca/blob/main/LICENSE

Orca is a **_Payment Switch_** that lets you connect with **multiple payment processors with a single API integration**.
Once integrated, you can add new payment processors and route traffic effortlessly.
Using Orca, you can:

- Reduce dependency on a single processor like Stripe
- Control & customize your payment flow with 100% visibility
- Reduce processing fees through smart routing
- Improve conversion rate with dynamic routing
- Expand your business reach with new payment methods
- Reduce development & testing efforts of adding new processors

_Orca is wire-compatible with top processors like Stripe making it easy to integrate._

<p align="center">
<img src= "./images/orca-product.png" alt="orca-product" width="40%" />
</p>

## Table of Contents

- [Quick Start Guide](#quick-start-guide)
- [Supported Features](#supported-features)
- [What's Included](#whats-included)
- [Join us in building ORCA](#join-us-in-building-orca)
- [Bugs and feature requests](#bugs-and-feature-requests)
- [Versioning](#versioning)
- [Copyright and License](#copyright-and-license)

## Quick Start Guide

### Try It Out

**Step 1:** Use [**Orca Sandbox**](https://orca-test-app.netlify.app/) to create your account and test payments.
Please save the API key for the next step.

**Step 2:** Import our [**Postman Collection**](https://www.getpostman.com/collections/63d0d419ce1d1140fc9f) using the link below.
(Please use the API key auth type.)

```text
https://www.getpostman.com/collections/63d0d419ce1d1140fc9f
```

### Installation Options

**Option 1:** Self-hosting with **Docker Image**.
_(This option is coming soon!!)_

**Option 2:** Setup dev environment using **Docker Compose**:

1. [Install Docker Compose](https://docs.docker.com/compose/install/).

2. Clone the repository:

   ```bash
   git clone https://github.com/juspay/orca.git
   ```

   You might need to create a [Personal Access Token (PAT)](https://docs.github.com/en/enterprise-server@3.4/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token) if you are prompted to authenticate.
   Use the generated PAT as the password.

3. [Optional] Configure your settings in [docker_compose.toml](./config/docker_compose.toml)

4. Run the application and create an account:

   1. Build and run Orca using Docker Compose:

      ```bash
      docker compose up -d
      ```

   2. Run database migrations:

      ```bash
      docker compose run orca-server bash -c "cargo install diesel_cli && diesel migration --database-url postgres://db_user:db_pass@pg:5432/orca_db run"
      ```

   3. Verify that the Orca server is up by checking your local server health:

      ```bash
      curl --location --request GET 'http://localhost:8080/health'
      ```

   4. Create your Merchant Account and get your account information:

      ```bash
      bash ./scripts/create_merchant_account.sh
      ```

#### Configure & Test

5. Configure & test using your API key:

   1. Add your Connector API keys in [`keys.conf`](./keys.conf) file.
      You can fetch API keys from the Connector's Dashboard (say Stripe/Adyen/Checkout dashboard).

   2. Configure the connector in your dev environment:

      ```bash
      bash ./scripts/create_connector_account.sh <connector_name> <your Orca merchant_id>
      ```

      Use the Orca merchant ID generated from the previous step.

   3. Run a health check for your local server:

      ```bash
      curl --location --request GET 'http://localhost:8080/health'
      ```

   4. Update the below command with your Orca API key and perform a test transaction.
      Refer our [Postman collection](https://www.getpostman.com/collections/63d0d419ce1d1140fc9f) to test more features (refunds, customers, payment methods etc.,)

      ```bash
      export API_KEY="<your api-key>"
      curl --location --request POST "http://localhost:8080/payments" \
      --header "Content-Type: application/json" \
      --header "Accept: application/json" \
      --header "api-key: ${API_KEY}" \
      --data-raw '{
      "amount": 6540,
      "currency": "USD",
      "confirm" :true,
      "return_url": "https://juspay.io",
      "payment_method": "card",
      "payment_method_data": {
         "card": {
            "card_number": "4000056655665556",
            "card_exp_month": "10",
            "card_exp_year": "25",
            "card_holder_name": "John Doe",
            "card_cvc": "123"
         }
      }
      }'
      ```

**Option 3:** Local setup:

a. [For MacOS](/INSTALL_macos.md)

b. [For Linux](/INSTALL_linux.md)

<!-- 4. Install with **Setup Script**

    a. Clone the repository
    ```
    git clone https://github.com/juspay/orca.git
    ```

    b. Execute script
    ```
    install orca.sh
    ```

    b. Create your Merchant Account
    ```
    create_merchant_account.sh
    ```

    c. [Configure & Test](#configure--test-the-setup) the setup using the api-key generated in Step 2 -->

### Fast Integration for Stripe Users

If you are already using Stripe, integrating with Orca is fun, fast & easy.
Try the steps below to get a feel for how quick the setup is:

1. Download Stripe's [demo app](https://stripe.com/docs/payments/quickstart)
2. Change server and client SDK dependencies in your app
3. Change API keys in your App
4. [Configure & Test](#configure--test)

## Supported Features

| Features                 | Stripe             | Adyen              | Checkout           | Authorize.net      | ACI                |
| ------------------------ | ------------------ | ------------------ | ------------------ | ------------------ | ------------------ |
| Payments - CRUD, Confirm | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| Customers - CRUD         | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| Refunds                  | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | WIP                |
| Mandates                 | :white_check_mark: | WIP                | WIP                | WIP                | WIP                |
| PCI Compliance           | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: |

The **hosted version** provides the following additional features:

- **System Performance & Reliability**

  - Scalable to support 50000 tps
  - System uptime of upto 99.99%
  - Low latency service
  - Hosting option with AWS, GCP

- **Value Added Services**

  - Compliance Support incl. PCI, GDPR etc
  - Support for processors / gateways not currently available as part of OSS (E.g. Chase Payments)
  - Integration with Risk Management Solutions
  - Support for Subscription

- **Payment Operations Support**

  - 24x7 Support
  - Dashboards with deep analytics
  - Experts team to consult and improve business metrics

<!--
## Documentation

Please refer to the following documentation pages:

- Getting Started Guide [Link]
- API Reference [Link]
- Payments Fundamentals [Link]
- Installation Support [Link]
- Router Architecture [Link]
 -->

## What's Included

Within the repositories you'll find the following directories and files, logically grouping common assets and providing both compiled and minified variations.

### Repositories

The current setup contains a single repo, which contains the core payment router and the various connector integrations under the `src/connector` sub-directory.

<!-- ### Sub-Crates -->

<!--
| Crate | Stability | Master | Docs | Example |
|--------|-----------|-------|:----:|:------:|
| [masking](./crates/masking) | [![experimental](https://raster.shields.io/static/v1?label=&message=experimental&color=orange)](https://github.com/emersion/stability-badges#experimental) | [![Health](https://raster.shields.io/static/v1?label=&message=unknown&color=333)]() | [![docs.rs](https://raster.shields.io/static/v1?label=&message=docs&color=eee)](https://docs.rs/masking) | [![Open in Gitpod](https://raster.shields.io/static/v1?label=&message=try%20online&color=eee)]() |
| [router](./crates/router) | [![experimental](https://raster.shields.io/static/v1?label=&message=experimental&color=orange)](https://github.com/emersion/stability-badges#experimental) | [![Health](https://raster.shields.io/static/v1?label=&message=unknown&color=333)]() | [![docs.rs](https://raster.shields.io/static/v1?label=&message=docs&color=eee)](https://docs.rs/router) | [![Open in Gitpod](https://raster.shields.io/static/v1?label=&message=try%20online&color=eee)]() |
-->

### Files Tree Layout

<!-- FIXME: this table should either be generated by a script or smoke test should be introduced checking it agrees with actual structure -->

```text
├── config                       : config files for router. This stores the initial startup config and separate configs can be provided for debug/release builds.
├── crates                       : sub-crates
│   ├── masking                  : making pii information for pci and gdpr compliance
│   ├── router                   : the main crate
│   └── router_derive            : utility macros for the router crate
├── docs                         : hand written documentation
├── examples                     : examples
├── logs                         : logs generated at runtime
├── migrations                   : diesel db setup
├── openapi                      : API definition
├── postman                      : postman scenarios for API
└── target                       : generated files
```

## Join us in building ORCA

### Our Belief

**We believe payments should be open, fast and cheap.**

Orca would allow everyone to quickly customize and set up an open payment switch, while giving a unified experience to your users, abstracting away the ever shifting payments landscape.

The Orca journey starts with a payment orchestrator.
It was born from our struggle to understand and integrate various payment options/payment processors/networks and banks, with varying degrees of documentation and inconsistent API semantics.

### Contributing

This project is created and currently maintained by [Juspay](https://juspay.io/juspay-router).

We welcome contributions from the open source community.
Please read through our [contributing guidelines](contrib/CONTRIBUTING.md).
Included are directions for opening issues, coding standards, and notes on development.

Important note for Rust developers: We aim for contributions from the community across a broad range of tracks.
Hence, we have prioritized simplicity and code readability over purely idiomatic code.
For example, some of the code in core functions (e.g. `payments_core`) is written to be more readable rather than being pure-idiomatic.

<!--
## Community

Get updates on ORCA development and chat with the community:

- Join our Slack channel [Link]
- Join our Discord channel [Link]
- Follow @orca_juspay on Twitter [Link]
- Read and subscribe to The Official Orca Blog [Link]
- Ask and explore our GitHub Discussion [Link]
-->

## Bugs and feature requests

Please read the issue guidelines and search for [existing and closed issues](https://github.com/juspay/orca/issues).
If your problem or idea is not addressed yet, please [open a new issue](https://github.com/juspay/orca/issues/new/choose).

## Versioning

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

## Copyright and License

This product is licensed under the [Apache 2.0 License](LICENSE).
