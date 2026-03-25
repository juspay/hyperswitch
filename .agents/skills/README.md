# Hyperswitch AI Coding Skills

> Context7-compatible skills for [Hyperswitch](https://hyperswitch.io) — the open-source payment orchestration platform trusted by hundreds of businesses globally.

## Quick Install

```bash
npx ctx7 skills install /juspay/hyperswitch
```

Or install specific skills:

```bash
npx ctx7 skills install /juspay/hyperswitch payment-orchestration/00-quickstart
```

## Skills Index

### Payment Orchestration

| Skill | Description |
|-------|-------------|
| [`payment-orchestration/00-quickstart`](./payment-orchestration/00-quickstart.md) | Zero-to-working-payment in 15 minutes |
| [`payment-orchestration/01-create-payment`](./payment-orchestration/01-create-payment.md) | All payment creation patterns — auth, capture, 3DS, wallets |
| [`payment-orchestration/02-refunds`](./payment-orchestration/02-refunds.md) | Refunds, partial refunds, disputes & chargeback evidence |
| [`payment-orchestration/03-smart-routing`](./payment-orchestration/03-smart-routing.md) | Connector routing — priority, volume-split, rule-based |
| [`payment-orchestration/04-webhook-handling`](./payment-orchestration/04-webhook-handling.md) | Webhook setup, HMAC verification, event processing |
| [`payment-orchestration/05-mandates-recurring`](./payment-orchestration/05-mandates-recurring.md) | Mandates, subscriptions, off-session MIT charges |
| [`payment-orchestration/06-payment-links`](./payment-orchestration/06-payment-links.md) | Hosted checkout links — create, customize, handle |

### Connectors

| Skill | Description |
|-------|-------------|
| [`connectors/00-connector-setup`](./connectors/00-connector-setup.md) | Onboard any connector — credentials, MCA, payment methods |
| [`connectors/01-stripe-deep-dive`](./connectors/01-stripe-deep-dive.md) | Stripe-specific: test cards, 3DS, webhooks, quirks |
| [`connectors/02-adyen-deep-dive`](./connectors/02-adyen-deep-dive.md) | Adyen-specific: HMAC setup, local methods, capture quirks |

### SDK

| Skill | Description |
|-------|-------------|
| [`sdk/01-react-integration`](./sdk/01-react-integration.md) | HyperElements + PaymentElement in React |

### Vault

| Skill | Description |
|-------|-------------|
| [`vault/00-vault-overview`](./vault/00-vault-overview.md) | Card tokenization, PCI scope, network tokens, self-hosted locker |

### Demo & Examples

| Skill | Description |
|-------|-------------|
| [`demo-store/00-demo-store-overview`](./demo-store/00-demo-store-overview.md) | Demo store, Postman collection, Cypress tests |

## How These Skills Work

These skills follow the [Context7](https://context7.com) universal skill format. Each skill is a Markdown file with YAML frontmatter containing:

- **`name`** — skill identifier
- **`description`** — trigger phrases that tell AI assistants when to invoke the skill
- **`version`** — semantic version
- **`tags`** — searchable categories

When you ask your AI assistant a question like *"how do I process a refund in Hyperswitch?"*, the assistant matches your query against skill descriptions and loads the relevant skill into context — giving you accurate, Hyperswitch-specific guidance instead of generic payment API advice.

## Compatible AI Assistants

- [Claude Code](https://claude.ai/code) (`npx ctx7 skills install /juspay/hyperswitch --claude`)
- [Cursor](https://cursor.sh) (`npx ctx7 skills install /juspay/hyperswitch --cursor`)
- [Windsurf](https://codeium.com/windsurf) (`npx ctx7 skills install /juspay/hyperswitch --cursor`)
- Any assistant supporting the `.agents/skills/` universal convention

## Contributing

See [`SKILL_FORMAT.md`](./SKILL_FORMAT.md) for the skill authoring guide.

See [`MASTER_SKILLS_LIST.md`](./MASTER_SKILLS_LIST.md) for the full skills roadmap.

To contribute a new skill:
1. Copy the template from `SKILL_FORMAT.md`
2. Place your file in the appropriate category directory
3. Follow the naming convention: `NN-skill-name.md` (e.g., `07-network-tokenization.md`)
4. Open a PR — include test results from `test-api.sh` if your skill covers API flows
