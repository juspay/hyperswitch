# Validation Scenarios

Scenarios for validating that each skill triggers correctly and produces accurate AI assistant responses.

## How to Use

For each scenario:
1. Ask your AI assistant the **User Query** exactly as written
2. Verify the assistant invokes the correct skill (check which skill is referenced)
3. Verify the **Expected Response Elements** appear in the answer

---

## payment-orchestration/00-quickstart

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I get started with Hyperswitch?" | Steps to get API key, first curl command, test card `4242424242424242` |
| 2 | "What are the steps to make my first payment?" | sandbox.hyperswitch.io URL, `api-key` header, `amount` in cents |
| 3 | "How do I set up a Hyperswitch sandbox?" | app.hyperswitch.io signup, connector setup, test card matrix |

---

## payment-orchestration/01-create-payment

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I create a payment in Hyperswitch?" | POST /payments, `amount`, `currency`, `confirm`, `capture_method` |
| 2 | "What's the difference between capture_method automatic and manual?" | `automatic` = immediate charge, `manual` = auth-only + requires_capture state |
| 3 | "How do I accept PayPal with Hyperswitch?" | `payment_method: wallet`, `paypal_redirect`, `return_url` required |
| 4 | "I want to auth now and capture later" | `capture_method: manual`, `requires_capture` status, POST capture endpoint |
| 5 | "Show me a 3DS payment" | `authentication_type: three_ds`, `next_action`, redirect URL, `complete_authorize` |

---

## payment-orchestration/02-refunds

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I refund a payment in Hyperswitch?" | POST /refunds, `payment_id`, `reason` enum values |
| 2 | "Can I do a partial refund?" | `amount` field, multiple refunds up to captured amount |
| 3 | "What happens when a chargeback is filed?" | `dispute.opened` webhook, evidence submission API |
| 4 | "Why is my refund still pending?" | Async refund, `refund.succeeded` webhook, 5-10 business days |

---

## payment-orchestration/03-smart-routing

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I route payments to different connectors?" | `routing` field, type options, `merchant_connector_id` |
| 2 | "I want to use Stripe for 70% and Adyen for 30%" | `volume_split`, array with `split` values summing to 100 |
| 3 | "How do I set up a fallback connector?" | `priority` type, ordered connector array |
| 4 | "Which connector does Hyperswitch use by default?" | Dashboard routing config, smart routing ML |

---

## payment-orchestration/04-webhook-handling

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I verify Hyperswitch webhook signatures?" | `x-webhook-signature-512` header, HMAC-SHA512, constant-time compare |
| 2 | "What events does Hyperswitch send?" | Event table: payment.succeeded, refund.succeeded, dispute.opened, etc. |
| 3 | "My webhook handler is getting duplicate events" | Idempotency with `event_id`, deduplication pattern |
| 4 | "How do I test webhooks locally?" | smee.io, ngrok forwarding instructions |

---

## payment-orchestration/05-mandates-recurring

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I save a card for future charges?" | `setup_future_usage: off_session`, `customer_acceptance`, `mandate_id` returned |
| 2 | "How do I charge a customer without them being present?" | `off_session: true`, `mandate_id`, no payment_method_data needed |
| 3 | "What is setup_future_usage?" | Two values: `on_session` / `off_session`, MIT vs CIT distinction |
| 4 | "My recurring charge failed with do_not_honor" | Soft decline handling, bring customer back on-session |

---

## payment-orchestration/06-payment-links

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I create a payment link?" | POST /payment_links, `link` field in response |
| 2 | "Can I customize the payment link page?" | `payment_link_config`, `theme`, `logo`, `seller_name` |
| 3 | "How do I know when a payment link is paid?" | `payment.succeeded` webhook, `return_url` redirect |

---

## connectors/00-connector-setup

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I add a connector to Hyperswitch?" | Dashboard → Connectors, MCA concept, credentials |
| 2 | "What is a merchant_connector_id?" | MCA ID format `mca_...`, per-configuration ID |
| 3 | "How do I enable Apple Pay on my connector?" | Payment Methods tab, toggle, domain verification |

---

## connectors/01-stripe-deep-dive

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I configure Stripe in Hyperswitch?" | `sk_test_...` key location, dashboard config fields |
| 2 | "What test cards work with Stripe in Hyperswitch?" | Test card table with scenarios |
| 3 | "Stripe webhooks aren't firing in Hyperswitch" | Stripe → Hyperswitch webhook forwarding setup |

---

## connectors/02-adyen-deep-dive

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I set up Adyen with Hyperswitch?" | API key, Merchant Account (exact string), HMAC key |
| 2 | "Adyen webhooks are failing signature verification" | HMAC key rotation, Adyen CA webhook config |
| 3 | "How do I accept iDEAL via Adyen?" | bank_redirect.ideal, redirect flow, mock bank page |

---

## sdk/01-react-integration

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How do I integrate Hyperswitch in React?" | `@juspay-tech/react-hyper-js`, `HyperElements`, `PaymentElement` |
| 2 | "How do I use loadHyper?" | publishable key, `customBackendUrl`, sandbox URL |
| 3 | "The PaymentElement isn't rendering" | `client_secret` required, `confirm: false` server-side |

---

## vault/00-vault-overview

| # | User Query | Expected Response Elements |
|---|-----------|---------------------------|
| 1 | "How does Hyperswitch handle PCI compliance?" | Vault tokenization, SAQ A scope, no PAN on your server |
| 2 | "How do I store a card for later use?" | `setup_future_usage`, `payment_method_id` token |
| 3 | "What is network tokenization in Hyperswitch?" | DPAN, Visa VTS / Mastercard MDES, higher auth rates |
