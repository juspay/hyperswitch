# Skill Format Specification

This document defines the authoring standard for Hyperswitch AI coding skills.

## File Naming

Skills are numbered Markdown files within category directories:

```
.agents/skills/<category>/<NN>-<skill-name>.md
```

- `<category>` — kebab-case directory: `payment-orchestration`, `connectors`, `sdk`, `vault`, `demo-store`
- `<NN>` — zero-padded sequential number within the category: `00`, `01`, `02`, …
- `<skill-name>` — kebab-case description: `quickstart`, `create-payment`, `stripe-deep-dive`

Examples:
```
payment-orchestration/00-quickstart.md
connectors/01-stripe-deep-dive.md
sdk/01-react-integration.md
```

---

## Frontmatter Schema

Every skill file begins with YAML frontmatter:

```yaml
---
name: hyperswitch-<descriptive-name>       # required — kebab-case, globally unique
description: <rich trigger description>    # required — see guidelines below
version: 1.0.0                             # required — semantic versioning
tags: [hyperswitch, <category>, <topic>]   # required — for search/filtering
---
```

### `description` Field Guidelines

The `description` field is **the most important part of a skill**. AI assistants use it to decide whether to invoke the skill. Write it to match the natural language queries developers actually ask.

**Required elements:**

1. An opening "Use this skill when..." clause
2. At least 6 quoted trigger phrases (what a developer would say)
3. Topic coverage summary

**Template:**
```
Use this skill when the user asks about "<phrase 1>", "<phrase 2>", "<phrase 3>",
"<phrase 4>", "<phrase 5>", "<phrase 6>", or needs to <accomplish goal>.
Covers: <topic 1>, <topic 2>, <topic 3>.
```

**Good example:**
```yaml
description: Use this skill when the user asks about "3D Secure", "3DS2",
"strong customer authentication", "SCA", "PSD2 compliance", "authentication_type field",
"challenge flow", or needs to handle 3DS redirects in Hyperswitch. Covers:
frictionless vs challenge flows, external 3DS providers, complete_authorize endpoint.
```

**Bad example (too vague):**
```yaml
description: Help with 3DS payments.
```

**Rules:**
- Quote specific phrases developers would type
- Include both formal terms ("3D Secure") and casual ones ("SCA", "the 3DS redirect thing")
- Include API field names people search for (`authentication_type`, `setup_future_usage`)
- Include error messages or status names people paste into chat
- Aim for 50–150 words in the description

---

## Body Structure

Recommended sections (adapt as needed; not all apply to every skill):

```markdown
# Skill Title

## Overview
2–3 sentences. What does this skill cover? Who needs it?

## Prerequisites
Bullet list of what the reader needs before following this skill.

## Core Concepts
Brief definitions of key terms (only if non-obvious).

## Step-by-Step Guide
Numbered steps for procedural flows.

## API Reference
Table of endpoints: Method | Path | Description
Key request fields table: Field | Type | Default | Notes

## Complete Examples
3–5 concrete, realistic, copy-pasteable examples.
Use real field names. Use `https://sandbox.hyperswitch.io` as the base URL.
Use `api-key: YOUR_API_KEY` as the auth header placeholder.

## Error Handling
Table: Error | Cause | Fix

## Production Tips
Bullet list of non-obvious advice. Must be specific — not generic "handle errors" advice.
```

---

## Code Example Standards

### Request Format
```bash
curl --request POST \
  --url https://sandbox.hyperswitch.io/payments \
  --header 'Content-Type: application/json' \
  --header 'api-key: YOUR_API_KEY' \
  --data '{
    "amount": 1000,
    "currency": "USD"
  }'
```

Or JSON-only for inline examples:
```json
POST /payments
{
  "amount": 1000,
  "currency": "USD"
}
```

### Rules
- `amount` values must always be in **smallest currency unit** — include a callout if non-obvious
- Use realistic but clearly fake test data: `4242424242424242` for card numbers, `cus_abc123` for IDs
- Use `https://sandbox.hyperswitch.io` as the base URL (never production)
- Omit optional fields from basic examples — show them in "with options" variants
- Always show the response body for the first example of any endpoint

---

## Quality Checklist

Before submitting a skill:

- [ ] Frontmatter has all required fields
- [ ] `name` is unique across all skills
- [ ] `description` has ≥6 quoted trigger phrases
- [ ] All endpoint paths verified against `api-reference/v1/openapi_spec_v1.json`
- [ ] All field names verified against `crates/api_models/src/payments.rs` or relevant model
- [ ] All enum values are valid (not invented)
- [ ] At least 3 complete request/response examples
- [ ] Error handling section covers the top 3 failure modes
- [ ] Production tips are specific to Hyperswitch, not generic
- [ ] Test scripts in `test-api.sh` extended to cover new endpoints (if applicable)
- [ ] `MASTER_SKILLS_LIST.md` updated to mark skill as ✅ Done

---

## Versioning

- Increment **patch** (`1.0.1`) for correcting errors or adding minor examples
- Increment **minor** (`1.1.0`) for adding new scenarios or sections
- Increment **major** (`2.0.0`) for complete rewrites or breaking API changes

---

## Anti-Patterns to Avoid

| Anti-pattern | Why | Fix |
|-------------|-----|-----|
| Vague description | Skill never triggers | Add specific quoted phrases |
| Invented field names | Developer uses wrong API | Verify against OpenAPI spec |
| Generic advice ("always validate input") | Not specific to Hyperswitch | Replace with Hyperswitch-specific tips |
| Missing error handling | Developer gets stuck on first failure | Add top 3 errors with causes and fixes |
| No return URL mentioned for redirect flows | Silent breakage | Always call out when return_url is required |
| Amounts without unit clarification | Off-by-100x errors | Always state "smallest currency unit" |
