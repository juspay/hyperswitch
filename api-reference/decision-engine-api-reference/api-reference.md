---
title: "API Reference"
description: "Schema-backed reference for every Decision Engine endpoint, with request/response models and an interactive playground."
---

# API Reference

This is the schema-backed reference for every Decision Engine endpoint. Each page below shows the full request and response model and includes an interactive playground, generated from the OpenAPI contract.

Looking for copy-paste examples and end-to-end flows instead? Start with the [API Guide](/decision-engine-api-reference/api-reference/guides/api-ref).

## Two ways to read the API

| Surface | Best for |
| --- | --- |
| [API Guide](/decision-engine-api-reference/api-reference/guides/api-ref) | Task-oriented `curl` examples, complete flows, and request variants. |
| API Reference (this section) | Exact request/response schemas and an interactive playground, one page per endpoint. |

For advanced rule examples — AND, OR, nested AND+OR, `volume_split_priority`, enum arrays, and number-array matching — see the [Advanced Routing Example](/decision-engine-api-reference/api-reference/guides/configure-routing/routing-advanced-example). For the exact `POST /routing/create` schema, use [Create Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/createRoutingRule).

## Access classes

| Class | Routes | Authentication |
| --- | --- | --- |
| Public | `GET /health`, `GET /health/ready`, `GET /health/diagnostics`, `POST /auth/signup`, `POST /auth/login` | None |
| Admin bootstrap | `POST /merchant-account/create` | Admin secret |
| Protected | All routing, decision, score update, rule config, API key, merchant read/delete, analytics, audit, config, and authenticated auth routes | `Authorization: Bearer <jwt_token>` or `x-api-key: <api_key>` |
| Sandbox | Any Decision Engine route served through `https://sandbox.hyperswitch.io` | Same auth rules plus `x-feature: decision-engine` |

## Endpoint Families

### Health

- [Health Check](/decision-engine-api-reference/api-reference/endpoint/health/healthCheck)
- [Health Ready](/decision-engine-api-reference/api-reference/endpoint/health/healthReady)
- [Health Diagnostics](/decision-engine-api-reference/api-reference/endpoint/health/healthDiagnostics)

### Auth And Onboarding

- [Signup](/decision-engine-api-reference/api-reference/endpoint/auth/signup)
- [Login](/decision-engine-api-reference/api-reference/endpoint/auth/login)
- [Logout](/decision-engine-api-reference/api-reference/endpoint/auth/logout)
- [Current User](/decision-engine-api-reference/api-reference/endpoint/auth/me)
- [List User Merchants](/decision-engine-api-reference/api-reference/endpoint/auth/listUserMerchants)
- [Switch Merchant](/decision-engine-api-reference/api-reference/endpoint/auth/switchMerchant)
- [Onboard Merchant](/decision-engine-api-reference/api-reference/endpoint/auth/onboardMerchant)

### API Keys

- [Create API Key](/decision-engine-api-reference/api-reference/endpoint/auth/createApiKey)
- [List API Keys](/decision-engine-api-reference/api-reference/endpoint/auth/listApiKeys)
- [Revoke API Key](/decision-engine-api-reference/api-reference/endpoint/auth/revokeApiKey)

### Merchant Account

- [Create Merchant](/decision-engine-api-reference/api-reference/endpoint/merchant-account/createMerchant)
- [Get Merchant](/decision-engine-api-reference/api-reference/endpoint/merchant-account/getMerchant)
- [Delete Merchant](/decision-engine-api-reference/api-reference/endpoint/merchant-account/deleteMerchant)
- [Get Merchant Debit Routing](/decision-engine-api-reference/api-reference/endpoint/merchant-account/getMerchantDebitRouting)
- [Update Merchant Debit Routing](/decision-engine-api-reference/api-reference/endpoint/merchant-account/updateMerchantDebitRouting)

### Gateway Decision

- [Decide Gateway](/decision-engine-api-reference/api-reference/endpoint/gateway-decision/decideGateway)
- [Legacy Decision Gateway](/decision-engine-api-reference/api-reference/endpoint/compatibility/legacyDecisionGateway)
- [Update Gateway Score](/decision-engine-api-reference/api-reference/endpoint/score-feedback/updateGatewayScore)
- [Legacy Update Score](/decision-engine-api-reference/api-reference/endpoint/compatibility/legacyUpdateScore)

### Routing Rules

- [Create Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/createRoutingRule)
- [Activate Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/activateRoutingRule)
- [Deactivate Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/deactivateRoutingRule)
- [List Routing Rules](/decision-engine-api-reference/api-reference/endpoint/routing-rules/listRoutingRules)
- [Get Active Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/getActiveRoutingRule)
- [Evaluate Routing Rule](/decision-engine-api-reference/api-reference/endpoint/routing-rules/evaluateRoutingRule)
- [Hybrid Routing](/decision-engine-api-reference/api-reference/endpoint/routing-rules/hybridRouting)

### Rule Configuration

- [Create Rule Config](/decision-engine-api-reference/api-reference/endpoint/rule-configuration/createRuleConfig)
- [Get Rule Config](/decision-engine-api-reference/api-reference/endpoint/rule-configuration/getRuleConfig)
- [Update Rule Config](/decision-engine-api-reference/api-reference/endpoint/rule-configuration/updateRuleConfig)
- [Delete Rule Config](/decision-engine-api-reference/api-reference/endpoint/rule-configuration/deleteRuleConfig)

### Config

- [Get Routing Config](/decision-engine-api-reference/api-reference/endpoint/compatibility/getRoutingConfig)
- [Configure SR Dimensions](/decision-engine-api-reference/api-reference/endpoint/compatibility/configSrDimension)

### Analytics

- [Overview](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsOverview)
- [Gateway Scores](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsGatewayScores)
- [Decisions](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsDecisions)
- [Routing Stats](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsRoutingStats)
- [Log Summaries](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsLogSummaries)
- [Payment Audit](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsPaymentAudit)
- [Preview Trace](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsPreviewTrace)
- [Cost Savings](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsCostSavings)
- [Routing Events](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsRoutingEvents)
- [A/B Test Experiment Results](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsExperimentResults)
- [A/B Test Experiment Transactions](/decision-engine-api-reference/api-reference/endpoint/analytics/analyticsExperimentTransactions)

## Curl Examples

For local and sandbox smoke-test examples, use [API Examples](/decision-engine-api-reference/api-reference/guides/api-ref).
