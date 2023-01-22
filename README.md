# HyperSwitch

[![Build Status][actions-badge]][actions-url]
[![Apache 2.0 license][license-badge]][license-url]

[actions-badge]: https://github.com/juspay/hyperswitch/workflows/CI/badge.svg
[actions-url]: https://github.com/juspay/hyperswitch/actions?query=workflow%3ACI+branch%3Amain
[license-badge]: https://img.shields.io/github/license/juspay/hyperswitch
[license-url]: https://github.com/juspay/hyperswitch/blob/main/LICENSE

HyperSwitch is an **Open Source Financial Switch** to make payments Fast,
Reliable and Affordable.
It lets you connect with **multiple payment processors with a single API
integration**.
Once integrated, you can add new payment processors and route traffic
effortlessly.
Using HyperSwitch, you can:

- **Reduce dependency** on a single processor like Stripe or Braintree
- **Reduce Dev efforts** by 90% in adding & maintaining integrations
- **Improve success rates** with auto-retries
- **Reduce processing fees** through smart routing
- **Customize your payment** flow with 100% visibility and control
- **Increase business reach** with local payment methods
- **Embrace diversity** in payments

_HyperSwitch is wire-compatible with top processors like Stripe making it easy
to integrate._

<p align="center">
<img src= "./docs/imgs/hyperswitch-product.png" alt="HyperSwitch-Product" width="60%" />
</p>

## Table of Contents

- [Quick Start Guide](#quick-start-guide)
- [Fast Integration for Stripe Users](#fast-integration-for-stripe-users)
- [Supported Features](#supported-features)
- [What's Included](#whats-included)
- [Join us in building HyperSwitch](#join-us-in-building-hyperswitch)
- [Community](#community)
- [Bugs and feature requests](#bugs-and-feature-requests)
- [Versioning](#versioning)
- [Copyright and License](#copyright-and-license)

## Quick Start Guide

You have two options to try out HyperSwitch:

1. [Try it in our Sandbox Environment](/docs/try_sandbox.md): Fast and easy to
   start.
   No code or setup required in your system.
2. Try our React Demo App: A simple demo of integrating Hyperswitch with your React app.

   <a href="https://github.com/aashu331998/hyperswitch-react-demo-app/archive/refs/heads/main.zip">
   <img src= "./docs/imgs/download-button.png" alt="Download Now" width="190rem" />
   </a>
3. [Install in your local system](/docs/try_local_system.md): Configurations and
   setup required in your system.
   Suitable if you like to customize the core offering.

## Fast Integration for Stripe Users

If you are already using Stripe, integrating with HyperSwitch is fun, fast &
easy.
Try the steps below to get a feel for how quick the setup is:

1. Get API keys from our [dashboard].
2. Follow the instructions detailed on our
   [documentation page][migrate-from-stripe].

[dashboard]: https://dashboard-hyperswitch.netlify.app
[migrate-from-stripe]: https://hyperswitch.io/docs/migrateFromStripe

## Supported Features

### Supported Payment Processors and Methods

As of Jan 2023, we support 14 payment processors and multiple payment methods.
In addition, we are continuously integrating new processors based on their reach
and community requests.
Our target is to support 100+ processors by H2 2023.
You can find the latest list of payment processors, supported methods, and
features
[here][supported-connectors-and-features].

[supported-connectors-and-features]: https://docs.google.com/spreadsheets/d/e/2PACX-1vQWHLza9m5iO4Ol-tEBx22_Nnq8Mb3ISCWI53nrinIGLK8eHYmHGnvXFXUXEut8AFyGyI9DipsYaBLG/pubhtml?gid=0&single=true

### Hosted Version

In addition to all the features of the open-source product, our hosted version
provides features and support to manage your payment infrastructure, compliance,
analytics, and operations end-to-end:

- **System Performance & Reliability**

  - Scalable to support 50000 tps
  - System uptime of up to 99.99%
  - Deployment with very low latency
  - Hosting option with AWS or GCP

- **Value Added Services**

  - Compliance Support incl. PCI, GDPR, Card Vault etc
  - Customize the integration or payment experience
  - Control Center with elaborate analytics and reporting
  - Integration with Risk Management Solutions
  - Integration with other platforms like Subscription, E-commerce, Accounting,
    etc.

- **Enterprise Support**

  - 24x7 Email / On-call Support
  - Dedicated Relationship Manager
  - Custom dashboards with deep analytics, alerts, and reporting
  - Expert team to consult and improve business metrics

You can [try the hosted version in our sandbox][dashboard].

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

Within the repositories you'll find the following directories and files,
logically grouping common assets and providing both compiled and minified
variations.

### Repositories

The current setup contains a single repo, which contains the core payment router
and the various connector integrations under the `src/connector` sub-directory.

<!-- ### Sub-Crates -->

<!--
| Crate | Stability | Master | Docs | Example |
|--------|-----------|-------|:----:|:------:|
| [masking](./crates/masking) | [![experimental](https://raster.shields.io/static/v1?label=&message=experimental&color=orange)](https://github.com/emersion/stability-badges#experimental) | [![Health](https://raster.shields.io/static/v1?label=&message=unknown&color=333)]() | [![docs.rs](https://raster.shields.io/static/v1?label=&message=docs&color=eee)](https://docs.rs/masking) | [![Open in Gitpod](https://raster.shields.io/static/v1?label=&message=try%20online&color=eee)]() |
| [router](./crates/router) | [![experimental](https://raster.shields.io/static/v1?label=&message=experimental&color=orange)](https://github.com/emersion/stability-badges#experimental) | [![Health](https://raster.shields.io/static/v1?label=&message=unknown&color=333)]() | [![docs.rs](https://raster.shields.io/static/v1?label=&message=docs&color=eee)](https://docs.rs/router) | [![Open in Gitpod](https://raster.shields.io/static/v1?label=&message=try%20online&color=eee)]() |
-->

### Files Tree Layout

<!-- FIXME: this table should either be generated by a script or smoke test
should be introduced checking it agrees with actual structure -->

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

## Join us in building HyperSwitch

### Our Belief

_We believe payments should be open, fast, reliable and affordable to serve
billions of people at scale._

<!--
HyperSwitch would allow everyone to quickly customize and set up an open payment
switch, while giving a unified experience to your users, abstracting away the
ever shifting payments landscape.

The HyperSwitch journey starts with a payment orchestrator.
It was born from our struggle to understand and integrate various payment
options/payment processors/networks and banks, with varying degrees of
documentation and inconsistent API semantics. -->

Globally payment diversity has been growing exponentially.
There are hundreds of payment processors and new payment methods.
So, businesses embrace diversity by onboarding multiple payment processors to
increase conversion, reduce cost and improve control.
But integrating and maintaining multiple processors needs a lot of dev efforts.
So, why should devs across companies repeat this same work?
Why can't it be unified and reused? Hence, HyperSwitch was born to create that
reusable core and let companies build or customize on top of it.

### Our Values

1. Embrace Diversity of Payments: It leads to a better experience, efficiency &
   resilience.
2. Future is Open Source: It enables Innovation, code reuse & affordability.
3. Be part of the Community: It helps in collaboration, learning & contribution.
4. Build it like a System Software: It makes the product reliable, secure &
   performant.
5. Maximize Value Creation: For developers, customers & partners.

### Contributing

This project is being created and maintained by [Juspay](https://juspay.in),
South Asia's largest payments orchestrator/switch, processing more than 50
Million transactions per day. The solution has 1Mn+ lines of Haskell code built
over 10 years.
HyperSwitch leverages our experience in building large-scale, enterprise-grade &
frictionless payment solutions.
It is built afresh for the global markets as an open-source product in Rust.
We are long-term committed to building and making it useful for the community.

The product roadmap is open for the community's feedback.
We shall evolve a prioritization process that is open and community-driven.
We welcome contributions from the community. Please read through our
[contributing guidelines](/docs/CONTRIBUTING.md).
Included are directions for opening issues, coding standards, and notes on
development.

Important note for Rust developers: We aim for contributions from the community
across a broad range of tracks. Hence, we have prioritized simplicity and code
readability over purely idiomatic code. For example, some of the code in core
functions (e.g., `payments_core`) is written to be more readable than
pure-idiomatic.

## Community

Get updates on HyperSwitch development and chat with the community:

- Read and subscribe to [the official HyperSwitch blog][blog].
- Join our [Discord server][discord].
- Join our [Slack workspace][slack].
- Ask and explore our [GitHub Discussions][github-discussions].

[blog]: https://blog.hyperswitch.io
[discord]: https://discord.gg/wJZ7DVW8mm
[slack]: https://join.slack.com/t/hyperswitch-io/shared_invite/zt-1k6cz4lee-SAJzhz6bjmpp4jZCDOtOIg
[github-discussions]: https://github.com/juspay/hyperswitch/discussions

## Bugs and feature requests

Please read the issue guidelines and search for [existing and closed issues].
If your problem or idea is not addressed yet, please [open a new issue].

[existing and closed issues]: https://github.com/juspay/hyperswitch/issues
[open a new issue]: https://github.com/juspay/hyperswitch/issues/new/choose

## Versioning

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

## Copyright and License

This product is licensed under the [Apache 2.0 License](LICENSE).
