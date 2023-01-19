# hyperswitch

[![Build Status][actions-badge]][actions-url]
[![Apache 2.0 license][license-badge]][license-url]

[actions-badge]: https://github.com/juspay/hyperswitch/workflows/CI/badge.svg
[actions-url]: https://github.com/juspay/hyperswitch/actions?query=workflow%3ACI+branch%3Amain
[license-badge]: https://img.shields.io/github/license/juspay/hyperswitch
[license-url]: https://github.com/juspay/hyperswitch/blob/main/LICENSE

hyperswitch is a **_Payment Switch_** that lets you connect with **multiple payment processors with a single API integration**.
Once integrated, you can add new payment processors and route traffic effortlessly.
Using hyperswitch, you can:

- Reduce dependency on a single processor like Stripe
- Control & customize your payment flow with 100% visibility
- Reduce processing fees through smart routing
- Improve conversion rate with dynamic routing
- Expand your business reach with new payment methods
- Reduce development & testing efforts of adding new processors

_hyperswitch is wire-compatible with top processors like Stripe making it easy to integrate._

<p align="center">
<img src= "./docs/imgs/hyperswitch-product.png" alt="hyperswitch-product" width="40%" />
</p>

## Table of Contents

- [Quick Start Guide](#quick-start-guide)
- [Supported Features](#supported-features)
- [What's Included](#whats-included)
- [Join us in building hyperswitch](#join-us-in-building-hyperswitch)
- [Community](#community)
- [Bugs and feature requests](#bugs-and-feature-requests)
- [Versioning](#versioning)
- [Copyright and License](#copyright-and-license)

## Quick Start Guide

### Try It Out

You have two options to try out hyperswitch:

1. [Try out our sandbox environment](/docs/try_sandbox.md): Requires the least
   effort and does not involve setting up anything on your system.
2. [Try out hyperswitch on your local system](/docs/try_local_system.md):
   Requires comparatively more effort as it involves setting up dependencies on
   your system.

### Fast Integration for Stripe Users

If you are already using Stripe, integrating with hyperswitch is fun, fast & easy.
Try the steps below to get a feel for how quick the setup is:

1. Get API keys from our [dashboard](https://dashboard-hyperswitch.netlify.app).
2. Follow the instructions detailed on our [documentation page](https://hyperswitch.io/docs).

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

## Join us in building hyperswitch

### Our Belief

**We believe payments should be open, fast and cheap.**

hyperswitch would allow everyone to quickly customize and set up an open payment switch, while giving a unified experience to your users, abstracting away the ever shifting payments landscape.

The hyperswitch journey starts with a payment orchestrator.
It was born from our struggle to understand and integrate various payment options/payment processors/networks and banks, with varying degrees of documentation and inconsistent API semantics.

### Contributing

This project is created and currently maintained by [Juspay](https://juspay.io/juspay-router).

We welcome contributions from the open source community.
Please read through our [contributing guidelines](/docs/CONTRIBUTING.md).
Included are directions for opening issues, coding standards, and notes on development.

Important note for Rust developers: We aim for contributions from the community across a broad range of tracks.
Hence, we have prioritized simplicity and code readability over purely idiomatic code.
For example, some of the code in core functions (e.g. `payments_core`) is written to be more readable rather than being pure-idiomatic.

## Community

Get updates on hyperswitch development and chat with the community:

- Read and subscribe to [the official hyperswitch blog](https://blog.hyperswitch.io)
- Join our [Discord server](https://discord.gg/wJZ7DVW8mm)
- Join our [Slack workspace](https://join.slack.com/t/hyperswitch-io/shared_invite/zt-1k6cz4lee-SAJzhz6bjmpp4jZCDOtOIg)
- Ask and explore our [GitHub Discussions](https://github.com/juspay/hyperswitch/discussions)

## Bugs and feature requests

Please read the issue guidelines and search for [existing and closed issues](https://github.com/juspay/hyperswitch/issues).
If your problem or idea is not addressed yet, please [open a new issue](https://github.com/juspay/hyperswitch/issues/new/choose).

## Versioning

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

## Copyright and License

This product is licensed under the [Apache 2.0 License](LICENSE).
