<p align="center">
  <img src="./docs/imgs/hyperswitch-logo-dark.svg#gh-dark-mode-only" alt="Hyperswitch-Logo" width="40%" />
  <img src="./docs/imgs/hyperswitch-logo-light.svg#gh-light-mode-only" alt="Hyperswitch-Logo" width="40%" />
</p>


<h1 align="center">The open-source payments switch</h1>

<div align="center" >
The single API to access payment ecosystems across 130+ countries</div>

<p align="center">
  <a href="#quick-start-guide">Quick Start Guide</a> •
  <a href="#fast-integration-for-stripe-users">Fast Integration for Stripe Users</a> •
  <a href="#supported-features">Supported Features</a> •
  <a href="#faqs">FAQs</a>
  <br>
  <a href="#whats-included">What's Included</a> •
  <a href="#join-us-in-building-hyperswitch">Join us in building HyperSwitch</a> •
  <a href="#community">Community</a> •
  <a href="#bugs-and-feature-requests">Bugs and feature requests</a> •
  <a href="#versioning">Versioning</a> •
  <a href="#copyright-and-license">Copyright and License</a>
</p>

<p align="center">
  <a href="https://github.com/juspay/hyperswitch/actions?query=workflow%3ACI+branch%3Amain">
    <img src="https://github.com/juspay/hyperswitch/workflows/CI/badge.svg" />
  </a>
  <a href="https://github.com/juspay/hyperswitch/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/juspay/hyperswitch" />
  </a>
  <a href="https://github.com/juspay/hyperswitch/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/Made_in-Rust-orange" />
  </a>
</p>

<h1 align="center">🎉 Hacktoberfest is here! 🎉</h1>

New to Rust? Hyperswitch is the perfect place to start this hacktoberfest! 😁

> ⭐️ If you're new to Hacktoberfest, you can learn more and register to participate [here](https://hacktoberfest.com/participation/). Registration is from **September 26th - October 31st**.


<hr>
<a href="https://github.com/juspay/hyperswitch/blob/main/LICENSE">
  <img src="./docs/imgs/switch.png" />
</a>

Hyperswitch is an open source payments switch to make payments fast, reliable, and, affordable.
It lets you connect with multiple payment processors and route traffic effortlessly, all with a single API integration.


Using Hyperswitch, you can:

- ⬇️ **Reduce dependency** on a single processor like Stripe or Braintree
- 🧑‍💻 **Reduce Dev effort** by 90% to add & maintain integrations
- 🚀 **Improve success rates** with seamless failover and auto-retries
- 💸 **Reduce processing fees** with smart routing
- 🎨 **Customize payment flows** with full visibility and control
- 🌐 **Increase business reach** with local/alternate payment methods

> Hyperswitch is **wire-compatible** with top processors like Stripe, making it easy to integrate.

<br>
<img src="./docs/imgs/hyperswitch-product.png" alt="Hyperswitch-Product" width="50%"/>

## ⚡️ Quick Start Guide


<a href="https://app.hyperswitch.io/register"><img src="./docs/imgs/signup-to-hs.svg" height="35"></a>

Ways to get started with Hyperswitch:

1. Try it in our Sandbox Environment: Fast and easy to
   start.
   No code or setup is required in your system, [learn more](/docs/try_sandbox.md)


<a href="https://app.hyperswitch.io/register"><img src="./docs/imgs/get-api-keys.svg" height="35"></a>

2. A simple demo of integrating Hyperswitch with your React App, Try our React [Demo App](https://github.com/aashu331998/hyperswitch-react-demo-app/archive/refs/heads/main.zip).


3. Install in your local system: Configurations and
   setup required in your system.
   Suitable if you like to customise the core offering, [setup guide](/docs/try_local_system.md)

## 🔌 Fast Integration for Stripe Users

If you are already using Stripe, integrating with Hyperswitch is fun, fast &
easy.
Try the steps below to get a feel for how quick the setup is:

1. Get API keys from our [dashboard].
2. Follow the instructions detailed on our
   [documentation page][migrate-from-stripe].

[dashboard]: https://app.hyperswitch.io/register
[migrate-from-stripe]: https://hyperswitch.io/docs/migrateFromStripe

## ✅ Supported Features

### 🌟 Supported Payment Processors and Methods

As of Sept 2023, we support 50+ payment processors and multiple global payment methods.
In addition, we are continuously integrating new processors based on their reach and community requests.
Our target is to support 100+ processors by H2 2023.
You can find the latest list of payment processors, supported methods, and
features
[here][supported-connectors-and-features].

[supported-connectors-and-features]: https://docs.google.com/spreadsheets/d/e/2PACX-1vQWHLza9m5iO4Ol-tEBx22_Nnq8Mb3ISCWI53nrinIGLK8eHYmHGnvXFXUXEut8AFyGyI9DipsYaBLG/pubhtml?gid=0&single=true

### 🌟 Hosted Version

In addition to all the features of the open-source product, our hosted version
provides features and support to manage your payment infrastructure, compliance,
analytics, and operations end-to-end:

- **System Performance & Reliability**

  - Scalable to support 50000 tps
  - System uptime of up to 99.99%
  - Deployment with very low latency
  - Hosting option with AWS or GCP

- **Value Added Services**

  - Compliance Support, incl. PCI, GDPR, Card Vault etc
  - Customise the integration or payment experience
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

## 🤔 FAQs

Got more questions?
Please refer to our [FAQs page][faqs].

[faqs]: https://hyperswitch.io/docs/devSupport

<!--
## Documentation

Please refer to the following documentation pages:

- Getting Started Guide [Link]
- API Reference [Link]
- Payments Fundamentals [Link]
- Installation Support [Link]
- Router Architecture [Link]
 -->

## What's Included❓

Within the repositories, you'll find the following directories and files,
logically grouping common assets and providing both compiled and minified
variations.

### Repositories

The current setup contains a single repo, which contains the core payment router
and the various connector integrations under the `src/connector` sub-directory.

<!-- ### Sub-Crates -->

### 🌳 Files Tree Layout

<!-- FIXME: this table should either be generated by a script or smoke test
should be introduced, checking it agrees with the actual structure -->

```text
.
├── config
├── connector-template
├── crates
│   ├── api_models
│   ├── cards
│   ├── common_enums
│   ├── common_utils
│   ├── data_models
│   ├── diesel_models
│   ├── drainer
│   ├── external_services
│   ├── masking
│   ├── redis_interface
│   ├── router
│   ├── router_derive
│   ├── router_env
│   ├── scheduler
│   ├── storage_impl
│   └── test_utils
├── docs
│   ├── imgs
│   └── rfcs
├── loadtest
│   ├── config
│   ├── grafana
│   └── k6
├── migrations
├── monitoring
│   └── config
├── openapi
├── postman
│   ├── collection-dir
│   └── collection-json
└── scripts
```

## 💪 Join us in building Hyperswitch

### 🤝 Our Belief

> Payments should be open, fast, reliable and affordable to serve
> the billions of people at scale.

Globally payment diversity has been growing at a rapid pace.
There are hundreds of payment processors and new payment methods like BNPL,
RTP etc.
Businesses need to embrace this diversity to increase conversion, reduce cost
and improve control.
But integrating and maintaining multiple processors needs a lot of dev effort.
Why should devs across companies repeat the same work?
Why can't it be unified and reused? Hence, Hyperswitch was born to create that
reusable core and let companies build and customise it as per their specific requirements.

### ✨ Our Values

1. Embrace Payments Diversity: It will drive innovation in the ecosystem in
   multiple ways.
2. Make it Open Source: Increases trust; Improves the quality and reusability of
   software.
3. Be community driven: It enables participatory design and development.
4. Build it like Systems Software: This sets a high bar for Reliability,
   Security and Performance SLAs.
5. Maximise Value Creation: For developers, customers & partners.

### 🤍 Contributing

This project is being created and maintained by [Juspay](https://juspay.in),
South Asia's largest payments orchestrator/switch, processing more than 50
Million transactions per day. The solution has 1Mn+ lines of Haskell code built
over ten years.
Hyperswitch leverages our experience in building large-scale, enterprise-grade &
frictionless payment solutions.
It is built afresh for the global markets as an open-source product in Rust.
We are long-term committed to building and making it useful for the community.

The product roadmap is open for the community's feedback.
We shall evolve a prioritisation process that is open and community-driven.
We welcome contributions from the community. Please read through our
[contributing guidelines](/docs/CONTRIBUTING.md).
Included are directions for opening issues, coding standards, and notes on
development.

🦀 **Important note for Rust developers**: We aim for contributions from the community
across a broad range of tracks. Hence, we have prioritised simplicity and code
readability over purely idiomatic code. For example, some of the code in core
functions (e.g., `payments_core`) is written to be more readable than
pure-idiomatic.

## 👥 Community

Get updates on Hyperswitch development and chat with the community:

- Read and subscribe to [the official Hyperswitch blog][blog].
- Join our [Discord server][discord].
- Join our [Slack workspace][slack].
- Ask and explore our [GitHub Discussions][github-discussions].

[blog]: https://hyperswitch.io/blog
[discord]: https://discord.gg/wJZ7DVW8mm
[slack]: https://join.slack.com/t/hyperswitch-io/shared_invite/zt-1k6cz4lee-SAJzhz6bjmpp4jZCDOtOIg
[github-discussions]: https://github.com/juspay/hyperswitch/discussions

<div style="display: flex;  justify-content: center;">
    <div style="margin-right:10px">
    <a href="https://www.producthunt.com/posts/hyperswitch-2?utm_source=badge-top-post-badge&utm_medium=badge&utm_souce=badge-hyperswitch&#0045;2" target="_blank">
        <img src="https://api.producthunt.com/widgets/embed-image/v1/top-post-badge.svg?post_id=375220&theme=light&period=weekly" alt="Hyperswitch - Fast, reliable, and affordable open source payments switch | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" />
    </a>
    </div>
    <div style="margin-right:10px">
    <a href="https://www.producthunt.com/posts/hyperswitch-2?utm_source=badge-top-post-topic-badge&utm_medium=badge&utm_souce=badge-hyperswitch&#0045;2" target="_blank">
        <img src="https://api.producthunt.com/widgets/embed-image/v1/top-post-topic-badge.svg?post_id=375220&theme=light&period=weekly&topic_id=267" alt="Hyperswitch - Fast, reliable, and affordable open source payments switch | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" />
    </a>
  </div>
  <div style="margin-right:10px">
    <a href="https://www.producthunt.com/posts/hyperswitch-2?utm_source=badge-top-post-topic-badge&utm_medium=badge&utm_souce=badge-hyperswitch&#0045;2" target="_blank">
        <img src="https://api.producthunt.com/widgets/embed-image/v1/top-post-topic-badge.svg?post_id=375220&theme=light&period=weekly&topic_id=93" alt="Hyperswitch - Fast, reliable, and affordable open source payments switch | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" />
    </a>
  </div>
</div>

## 🐞 Bugs and feature requests

Please read the issue guidelines and search for [existing and closed issues].
If your problem or idea is not addressed yet, please [open a new issue].

[existing and closed issues]: https://github.com/juspay/hyperswitch/issues
[open a new issue]: https://github.com/juspay/hyperswitch/issues/new/choose

## 🔖 Versioning

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

## ©️ Copyright and License

This product is licensed under the [Apache 2.0 License](LICENSE).


## ✨ Thanks to all contributors

Thank you for your support in hyperswitch's growth. Keep up the great work! 🥂

<a href="https://github.com/juspay/hyperswitch/graphs/contributors">
  <img src="https://contributors-img.web.app/image?repo=juspay/hyperswitch" alt="Contributors"/>
</a>
