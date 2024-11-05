<p align="center">
  <img src="./docs/imgs/hyperswitch-logo-dark.svg#gh-dark-mode-only" alt="Hyperswitch-Logo" width="40%" />
  <img src="./docs/imgs/hyperswitch-logo-light.svg#gh-light-mode-only" alt="Hyperswitch-Logo" width="40%" />
</p>

<h1 align="center">The open-source payments switch</h1>

<div align="center" >
The single API to access payment ecosystems across 130+ countries</div>

<p align="center">
  <a href="#try-a-payment">Try a Payment</a> •
  <a href="#quick-setup">Quick Setup</a> •
  <a href="/docs/try_local_system.md">Local Setup Guide (Hyperswitch App Server)</a> •
  <a href="https://api-reference.hyperswitch.io/introduction"> API Docs </a> 
   <br>
  <a href="#community-contributions">Community and Contributions</a> •
  <a href="#bugs-and-feature-requests">Bugs and feature requests</a> •
  <a href="#versioning">Versioning</a> •
  <a href="#copyright-and-license">Copyright and License</a>
</p>

<p align="center">
  <a href="https://github.com/juspay/hyperswitch/actions?query=workflow%3ACI+branch%3Amain">
    <img src="https://github.com/juspay/hyperswitch/workflows/CI-push/badge.svg" />
  </a>
  <a href="https://github.com/juspay/hyperswitch/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/juspay/hyperswitch" />
  </a>
  <a href="https://github.com/juspay/hyperswitch/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/Made_in-Rust-orange" />
  </a>
</p>
<p align="center">
  <a href="https://www.linkedin.com/company/hyperswitch/">
    <img src="https://img.shields.io/badge/follow-hyperswitch-blue?logo=linkedin&labelColor=grey"/>
  </a>
  <a href="https://x.com/hyperswitchio">
    <img src="https://img.shields.io/badge/follow-%40hyperswitchio-white?logo=x&labelColor=grey"/>
  </a>
  <a href="https://join.slack.com/t/hyperswitch-io/shared_invite/zt-2jqxmpsbm-WXUENx022HjNEy~Ark7Orw">
    <img src="https://img.shields.io/badge/chat-on_slack-blue?logo=slack&labelColor=grey&color=%233f0e40"/>
  </a>
</p>

<hr>

Hyperswitch is a community-led, open payments switch designed to empower digital businesses by providing fast, reliable, and affordable access to the best payments infrastructure.

Here are the components of Hyperswitch that deliver the whole solution:

* [Hyperswitch Backend](https://github.com/juspay/hyperswitch): Powering Payment Processing

* [SDK (Frontend)](https://github.com/juspay/hyperswitch-web): Simplifying Integration and Powering the UI

* [Control Centre](https://github.com/juspay/hyperswitch-control-center): Managing Operations with Ease

Jump in and contribute to these repositories to help improve and expand Hyperswitch!

<img src="./docs/imgs/switch.png" />


<a href="#Quick Setup">
  <h2 id="quick-setup">⚡️ Quick Setup</h2>
</a>

### Docker Compose

You can run Hyperswitch on your system using Docker Compose after cloning this repository:

```shell
git clone --depth 1 --branch latest https://github.com/juspay/hyperswitch
cd hyperswitch
docker compose up -d
```

This will start the app server, web client/SDK and control center.

Check out the [local setup guide][local-setup-guide] for a more comprehensive
setup, which includes the [scheduler and monitoring services][docker-compose-scheduler-monitoring].

### One-click deployment on AWS cloud

The fastest and easiest way to try Hyperswitch is via our CDK scripts

1. Click on the following button for a quick standalone deployment on AWS, suitable for prototyping.
   No code or setup is required in your system and the deployment is covered within the AWS free-tier setup.

   <a href="https://console.aws.amazon.com/cloudformation/home?region=us-east-1#/stacks/new?stackName=HyperswitchBootstarp&templateURL=https://hyperswitch-synth.s3.eu-central-1.amazonaws.com/hs-starter-config.yaml"><img src="./docs/imgs/aws_button.png" height="35"></a>

2. Sign-in to your AWS console.

3. Follow the instructions provided on the console to successfully deploy Hyperswitch

[docs-link-for-enterprise]: https://docs.hyperswitch.io/hyperswitch-cloud/quickstart
[docs-link-for-developers]: https://docs.hyperswitch.io/hyperswitch-open-source/overview
[contributing-guidelines]: docs/CONTRIBUTING.md
[dashboard-link]: https://app.hyperswitch.io/
[website-link]: https://hyperswitch.io/
[learning-resources]: https://docs.hyperswitch.io/learn-more/payment-flows
[local-setup-guide]: /docs/try_local_system.md
[docker-compose-scheduler-monitoring]: /docs/try_local_system.md#running-additional-services

<a href="https://app.hyperswitch.io/">
  <h2 id="try-a-payment">⚡️ Try a Payment</h2>
</a>

To quickly experience the ease of Hyperswitch, sign up on the [Hyperswitch Control Center](https://app.hyperswitch.io/) and try a payment. Once you've completed your first transaction, you’ve successfully made your first payment with Hyperswitch!

<a href="#community-contributions">
  <h2 id="community-contributions">✅ Community & Contributions</h2>
</a>

The community and core team are available in [GitHub Discussions](https://github.com/juspay/hyperswitch/discussions), where you can ask for support, discuss roadmap, and share ideas.

Our [Contribution Guide](https://github.com/juspay/hyperswitch/blob/main/docs/CONTRIBUTING.md) describes how to contribute to the codebase and Docs.

Join our Conversation in [Slack](https://join.slack.com/t/hyperswitch-io/shared_invite/zt-2jqxmpsbm-WXUENx022HjNEy~Ark7Orw), [Discord](https://discord.gg/wJZ7DVW8mm), [Twitter](https://x.com/hyperswitchio)


<a href="#Join-us-in-building-Hyperswitch">
  <h2 id="join-us-in-building-hyperswitch">💪 Join us in building Hyperswitch</h2>
</a>

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

This project is being created and maintained by [Juspay](https://juspay.io)

<a href="#Bugs and feature requests">
  <h2 id="bugs-and-feature-requests">🐞 Bugs and feature requests</h2>
</a>

Please read the issue guidelines and search for [existing and closed issues].
If your problem or idea is not addressed yet, please [open a new issue].

[existing and closed issues]: https://github.com/juspay/hyperswitch/issues
[open a new issue]: https://github.com/juspay/hyperswitch/issues/new/choose

<a href="#Versioning">
  <h2 id="versioning">🔖 Versioning</h2>
</a>

Check the [CHANGELOG.md](./CHANGELOG.md) file for details.

<a href="#©Copyright and License">
  <h2 id="copyright-and-license">©️ Copyright and License</h2>
</a>

This product is licensed under the [Apache 2.0 License](LICENSE).

<a href="#Thanks to all contributors">
  <h2 id="Thanks to all contributors">✨ Thanks to all contributors</h2>
</a>

Thank you for your support in hyperswitch's growth. Keep up the great work! 🥂

<a href="https://github.com/juspay/hyperswitch/graphs/contributors">
  <img src="https://contributors-img.web.app/image?repo=juspay/hyperswitch" alt="Contributors"/>
</a>
