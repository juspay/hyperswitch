<p align="left">
  <img src="./docs/imgs/hyperswitch-logo-dark.svg#gh-dark-mode-only" alt="Hyperswitch-Logo" width="40%" />
  <img src="./docs/imgs/hyperswitch-logo-light.svg#gh-light-mode-only" alt="Hyperswitch-Logo" width="40%" />
</p>


<h1 align="left">The open-source payments switch</h1>

<p align="left">
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


<hr>
<img src="./docs/imgs/switch.png" />

<h2>What is Hyperswitch?</h2>

Hyperswitch is a community-led, open payments and high-performance switch to enable access to the best payments infrastructure for every digital business. It is best suited for digital businesses that want to take control of their payments. Using Hyperswitch, you can:

- ⬇️ **Reduce dependency** on a single processor like Stripe or Braintree
- 🧑‍💻 **Reduce Dev effort** by 90% to add & maintain integrations
- 🚀 **Improve success rates** with seamless failover and auto-retries
- 💸 **Reduce processing fees** with smart routing
- 🎨 **Customize payment flows** with full visibility and control
- 🌐 **Increase business reach** with local/alternate payment methods

This project is being created and maintained by [Juspay](https://juspay.in), South Asia's largest payments orchestrator/switch, processing more than 70 Million transactions per day. The solution has 1Mn+ lines of Haskell code built over ten years. Hyperswitch leverages our experience in building large-scale, enterprise-grade & frictionless payment solutions. It is built afresh for the global markets as an open-source product in Rust. We are committed for a long-term to building and making it useful for the community.

<br>
<img src="./docs/imgs/hyperswitch-product.png" alt="Hyperswitch-Product" width="50%"/>

<h2>Table of contents</h2>

* Quickstart
* Core Features
* Roadmap
* Architecture
* About the repo
* Contribute
* Need Help?
* Versioning
* License
* Read More 


<a href="#Quick Start Guide">
  <h2 id="Quick Start Guide">Quick Start Guide</h2>
</a>

**One-click deployment on AWS cloud** - The fastest and easiest way to try hyperswitch is via our CDK scripts

1. Click on the following button for a quick standalone deployment on AWS, suitable for prototyping. No code or setup is required in your system and the deployment is covered within the AWS free-tier setup.

&emsp;&emsp; <a title="Bootstrap" href="https://console.aws.amazon.com/cloudformation/home?region=us-east-1#/stacks/new?stackName=cdk-hs&templateURL=https://hyperswitch-synth.s3.eu-central-1.amazonaws.com/bootstrap-template.yml"> Click here if you have not bootstrapped your region before deploying</a>

&emsp;&emsp; <a href="https://console.aws.amazon.com/cloudformation/home?region=us-east-1#/stacks/new?stackName=Hyperswitch&templateURL=https://hyperswitch-synth.s3.eu-central-1.amazonaws.com/deployment.yaml"><img src="./docs/imgs/aws_button.png" height="35"></a>

2. Sign-in to your AWS console.
3. Follow the instructions provided on the console to successfully deploy Hyperswitch

For an early access to the production-ready setup fill this <a href="https://forms.gle/v6ru55XDZFufVPnu9">Early Access Form</a>

**Fast Integration for Stripe Users** - If you are already using Stripe, integrating with Hyperswitch is fun, fast & easy. Try the steps below to get a feel for how quick the setup is:

1. Get API keys from our [dashboard].
2. Follow the instructions detailed on our
   [documentation page][migrate-from-stripe].

[dashboard]: https://app.hyperswitch.io/register
[migrate-from-stripe]: https://hyperswitch.io/docs/migrateFromStripe

<a href="#Core Features">
  <h2 id="Core Features">Core Features</h2>
</a>

* **Web Client** - It is an inclusive, consistent and blended payment experience optimized for the best payment conversions
  * **Unified payment experience** - _details to be added_
  * **Elaborate customizations** - _details to be added_
  * **Unified payment ops & analytics** - _details to be added_
  
* **App Server** - It is the core payments engine responsible for managing payment flows, payment unification and smart routing
  * **Super lightweight** - It is is optimized for sub 30 ms application overhead (and getting better) and falls within 5% of the payment processor's latency.
  * **Continuos availability** - _details to be added_
  * **Horizontal scalability** - _details to be added_
  * **Supports any payment processor and payment method** - We support 50+ payment processors and multiple global payment methods. In addition, we are continuously integrating new processors based on their reach and community requests. You can find the latest list of payment processors, supported methods, and features [here][supported-connectors-and-features].
    
* **Control Centre** - A dashboard for payment analytics and operations, adding and managing payment processors or payment methods and configuring payment routing rules
    * **One-click processor addition** - _details to be added_
    * **No-code smart retry** - _details to be added_
    * **Unified payment ops & analytics** - _details to be added_


[supported-connectors-and-features]: https://docs.google.com/spreadsheets/d/e/2PACX-1vQWHLza9m5iO4Ol-tEBx22_Nnq8Mb3ISCWI53nrinIGLK8eHYmHGnvXFXUXEut8AFyGyI9DipsYaBLG/pubhtml?gid=0&single=true

Got more questions? Please refer to our [FAQs page][faqs].

[faqs]: https://hyperswitch.io/docs/devSupport

<a href="#Roadmap">
  <h2 id="Roadmap">Roadmap</h2>
</a>

[Here's][OND Roadmap] a list of key features that are being worked on for the upcoming release or have been recently released. You can also request for a feature or submit a new idea on the same link. 

[OND Roadmap]: https://github.com/juspay/hyperswitch/wiki/Roadmap-(Oct-%E2%80%90-Dec-2023)

<a href="#Architecture">
  <h2 id="Architecture">Architecture</h2>
</a>

Below is the Hyperswitch architecture diagram. Review detailed architecture in our [Docs][Hyperswitch Architecture]

[Hyperswitch Architecture]: https://opensource.hyperswitch.io/learn-how-hyperswitch-works/hyperswitch-architecture 

<img src="https://github.com/juspay/hyperswitch/assets/118448330/d4f3a512-7683-4561-a42c-8641cccfbd3e" alt="Hyperswitch-Logo" width="60%" />


<a href="#About the repo">
  <h2 id="About the repo">About the repo</h2>
</a>

Within the repositories, you'll find the following directories and files, logically grouping common assets and providing both compiled and minified variations.

### Repositories

The current setup contains a single repo, which contains the core payment router
and the various connector integrations under the `src/connector` sub-directory.

<!-- ### Sub-Crates -->

### Files Tree Layout

<!-- FIXME: this table should either be generated by a script or smoke test
should be introduced, checking it agrees with the actual structure -->

```text
.
├── config                             : Initial startup config files for the router
├── connector-template                 : boilerplate code for connectors
├── crates                             : sub-crates
│   ├── api_models                     : Request/response models for the `router` crate
│   ├── cards                          : Types to handle card masking and validation
│   ├── common_enums                   : Enums shared across the request/response types and database types
│   ├── common_utils                   : Utilities shared across `router` and other crates
│   ├── data_models                    : Represents the data/domain models used by the business/domain layer
│   ├── diesel_models                  : Database models shared across `router` and other crates
│   ├── drainer                        : Application that reads Redis streams and executes queries in database
│   ├── external_services              : Interactions with external systems like emails, KMS, etc.
│   ├── masking                        : Personal Identifiable Information protection
│   ├── redis_interface                : A user-friendly interface to Redis
│   ├── router                         : Main crate of the project
│   ├── router_derive                  : Utility macros for the `router` crate
│   ├── router_env                     : Environment of payment router: logger, basic config, its environment awareness
│   ├── scheduler                      : Scheduling and executing deferred tasks like mail scheduling
│   ├── storage_impl                   : Storage backend implementations for data structures & objects
│   └── test_utils                     : Utilities to run Postman and connector UI tests
├── docs                               : hand-written documentation
├── loadtest                           : performance benchmarking setup
├── migrations                         : diesel DB setup
├── monitoring                         : Grafana & Loki monitoring related configuration files
├── openapi                            : automatically generated OpenAPI spec
├── postman                            : postman scenarios API
└── scripts                            : automation, testing, and other utility scripts
```


<a href="#Contribute">
  <h2 id="Contribute">Contribute</h2>
</a>

As an open-source project with a strong focus on the user community, we welcome contributions as GitHub pull requests. Please read through our [contributing guidelines](/docs/CONTRIBUTING.md). Included are directions for opening issues, coding standards, and notes on development. We aim for contributions from the community across a broad range of tracks. Hence, we have prioritised simplicity and code readability over purely idiomatic code. For example, some of the code in core functions (e.g., `payments_core`) is written to be more readable than pure-idiomatic. 

<a href="#Need Help?">
  <h2 id="Need Help">Need Help?</h2>
</a>

Get updates on Hyperswitch development and chat with the community:

- [Slack workspace][slack] - Real-time chat with the Hyperswitch crew. Meet fellow users, contributors, and our developer advocates. Perfect for quick questions.
- [GitHub Discussions][github-discussions] - Drop feature requests or suggest anything payments-related you need for your stack. Got questions? Ask away!

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


<a href="#Versioning">
  <h2 id="Versioning">Versioning</h2>
</a>

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

<a href="#License">
  <h2 id="License">License</h2>
</a>

This product is licensed under the [Apache 2.0 License](LICENSE).

<a href="#Read more">
  <h2 id="Read more">Read more</h2>
</a>

- For in-depth details into every component of Hyperswitch, see our [developer docs][docs]
- Hyperswitch [tech blogs][blog]

[blog]: https://hyperswitch.io/blog 
[docs]:  https://hyperswitch.io/docs 

<a href="#Thanks to all contributors">
  <h2 id="Thanks to all contributors">Thanks to all contributors</h2>
</a>

Thank you for your support in hyperswitch's growth. Keep up the great work! 🥂

<a href="https://github.com/juspay/hyperswitch/graphs/contributors">
  <img src="https://contributors-img.web.app/image?repo=juspay/hyperswitch" alt="Contributors"/>
</a>
