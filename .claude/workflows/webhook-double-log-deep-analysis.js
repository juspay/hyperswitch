export const meta = {
  name: 'webhook-double-log-deep-analysis',
  description: 'Exhaustive analysis of incoming-webhook api_events double logging with adversarially verified fix designs',
  phases: [
    { title: 'Facts', detail: 'parallel readers: emitters, webhook paths, consumers, history, existing mechanisms' },
    { title: 'Design', detail: '4 independent fix designs from distinct stances' },
    { title: 'Verify', detail: '3 adversarial lenses per design' },
    { title: 'Judge', detail: 'rank designs + completeness critic' },
  ],
}

const ROOT = '/Users/anurag.thakur/Work/switchhyper/hyperswitch'

const ISSUE = `
ISSUE: For every incoming webhook API call, 2 events are logged in the api_events table.
- Outer (generic): crates/router/src/routes/webhooks.rs wraps the handler in api::server_wrap. server_wrap_util builds an ApiEvent and calls state.event_handler().log_event(&api_event) at crates/router/src/services/api.rs:350/:368 — fires for every API call.
- Inner (webhook-specific): the wrapped closure incoming_webhooks_wrapper extracts the same request_id (crates/router/src/core/webhooks/incoming.rs:104), builds a second ApiEvent of type ApiEventsType::Webhooks{...} (:121) and calls log_event again at :138.
Two log_event calls, one request, identical request_id. Rows differ in api_event_type (Webhooks{connector,payment_id,refund_id} vs Miscellaneous) and status_code (inner hardcodes 200 at :111).
The inner emit exists to attach connector/payment/refund metadata that server_wrap cannot infer (webhook response type is serde_json::Value -> Miscellaneous).
Subtle problem: for webhooks the 'response' field of the inner row does not hold the actual API response; it holds the WebhookResponseTracker (status of the payment/refund/dispute entity after webhook execution).
GOAL: decide how to fix this (single correct row, right semantics, no downstream breakage).
`

const PRIOR = `
PRIOR FINDINGS from a first-pass analysis (treat as hypotheses — VERIFY against code, correct anything wrong):
- Outer row: request field is "null" because the route passes () as payload (routes/webhooks.rs:28); response is None because webhooks return ApplicationResponse::StatusOk or TextPlain which server_wrap_util does not serialize (services/api.rs:295-306); event_type falls back to Miscellaneous because impl ApiEventMetric for serde_json::Value is empty (common_utils/src/events.rs:186); status_code is real; error path IS logged.
- Inner row: request = masked webhook body; response = WebhookResponseTracker JSON; status hardcoded 200 (incoming.rs:111); NOT logged on error path (incoming_webhooks_core errors propagate via ? before the emit); latency covers core only.
- Same double-emit bug in network_token_incoming_webhooks_wrapper (incoming.rs:143-200).
- ApiEventsType::Webhooks{connector,payment_id,refund_id} fields are #[serde(flatten)]-ed and become real ClickHouse columns (crates/analytics/docs/clickhouse/scripts/api_events.sql); analytics payment-scoped queries filter on payment_id (crates/analytics/src/api_event/events.rs) — so the inner row is the only one visible to payment-level API-log queries.
- ReqState { event_context: events::EventContext<EventType, EventsHandler> } at crates/router/src/routes/app.rs:116; server_wrap_util calls request_state.event_context.record_info(...) for request_id/flow/tenant_id — capabilities of EventContext not yet examined.
- impl ApiEventMetric for ApplicationResponse<T> (hyperswitch_domain_models/src/api.rs:45) returns None for StatusOk/TextPlain, so metadata cannot ride on the response value for webhooks; the HTTP ack body to the connector must not change.
Candidate directions identified so far (non-exhaustive, challenge and improve):
 (A) carry metadata on typed response — believed non-viable for StatusOk/TextPlain;
 (B) suppress the outer emit for webhook flows and fix the inner one (real status, error rows);
 (C) single emit in server_wrap_util enriched via a request-scoped side channel (e.g. Arc<Mutex<Option<ApiEventOverrides>>> on ReqState) — first-pass recommendation;
 (D) keep both rows, dedupe downstream.
`

const FACTS = {
  type: 'object',
  properties: {
    topic: { type: 'string' },
    report: { type: 'string', description: 'Detailed findings with exact file:line evidence for every claim' },
    corrections_to_prior: { type: 'array', items: { type: 'string' }, description: 'Anything in the PRIOR FINDINGS block that is wrong or incomplete' },
    key_facts: { type: 'array', items: { type: 'string' }, description: 'The 5-15 most load-bearing facts, each with file:line' },
  },
  required: ['topic', 'report', 'key_facts'],
}

const DESIGN = {
  type: 'object',
  properties: {
    approach_key: { type: 'string', description: 'short kebab-case identifier for the mechanism, e.g. reqstate-side-channel, flow-skip-outer, event-context-emit, typed-response' },
    title: { type: 'string' },
    summary: { type: 'string', description: '3-6 sentence executive summary' },
    mechanism: { type: 'string', description: 'Precise code-level design: types, signatures, where each change goes (file:line anchors), sketch of key code' },
    files_touched: { type: 'array', items: { type: 'string' } },
    response_field_semantics: { type: 'string', description: 'What ends up in the api_events response column and where the WebhookResponseTracker data goes' },
    error_path: { type: 'string', description: 'What gets logged when webhook processing fails' },
    coverage: { type: 'string', description: 'How the design handles v1, v2, relay, network-token webhooks and any other emit sites' },
    downstream_impact: { type: 'string', description: 'Effect on ClickHouse schema, analytics queries, dashboards, Kafka consumers' },
    risks: { type: 'array', items: { type: 'string' } },
    effort: { type: 'string', description: 'rough size: files/LOC touched, migration needs' },
    why_best: { type: 'string' },
  },
  required: ['approach_key', 'title', 'summary', 'mechanism', 'files_touched', 'response_field_semantics', 'error_path', 'coverage', 'downstream_impact', 'risks', 'effort', 'why_best'],
}

const VERDICT = {
  type: 'object',
  properties: {
    lens: { type: 'string' },
    verdict: { type: 'string', enum: ['sound', 'fixable-issues', 'fatal'] },
    issues: { type: 'array', items: { type: 'string' }, description: 'Each issue with file:line evidence and severity' },
    notes: { type: 'string' },
  },
  required: ['lens', 'verdict', 'issues', 'notes'],
}

const JUDGE = {
  type: 'object',
  properties: {
    ranking: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          approach_key: { type: 'string' },
          score: { type: 'number', description: '0-10' },
          rationale: { type: 'string' },
        },
        required: ['approach_key', 'score', 'rationale'],
      },
    },
    winner: { type: 'string' },
    hybrid_improvements: { type: 'string', description: 'Ideas from losing designs worth grafting onto the winner' },
    dissent: { type: 'string', description: 'Strongest argument against the winner' },
  },
  required: ['ranking', 'winner', 'hybrid_improvements', 'dissent'],
}

const COMMON = `You are analyzing the Hyperswitch payment router (Rust, actix-web) at ${ROOT}.\n${ISSUE}\n${PRIOR}\nYour final output is consumed programmatically by an orchestrator — return complete raw findings, not a chatty summary. Cite exact file:line for every claim. Read the actual code; do not trust the PRIOR block without checking.`

phase('Facts')
log('Fanning out 5 fact-finding readers')

const FACT_TASKS = [
  {
    key: 'event-machinery',
    effort: 'high',
    prompt: `${COMMON}
TOPIC: The event-emission machinery. Investigate exhaustively:
1. Read crates/router/src/services/api.rs server_wrap_util end to end. Document exactly how the ApiEvent is built (all inputs) and every place event_type/serialized_request/serialized_response/status_code come from.
2. Find and read the EventContext type used in ReqState (crates/router/src/routes/app.rs:116 -> events::EventContext<crate::events::EventType, EventsHandler>). Where is it defined (likely crates/router/src/events.rs or crates/events)? What does record_info do? What is the EventInfo trait? Does info recorded on EventContext flow into any emitted event — could ApiEvent be emitted through EventContext so handler-recorded info lands on it? Is EventContext Clone, and does the closure passed to server_wrap get a usable handle (ReqState) it could mutate or record into?
3. What is EventsHandler / state.event_handler().log_event — variants (logger vs kafka), and what KafkaMessage impl for ApiEvent does (topic, key=request_id).
4. Assess concretely: if we wanted the handler closure to hand metadata back to server_wrap_util after func() completes, what channels already exist (ReqState fields, EventContext, AppState, task-locals, response headers) and which would require new plumbing? Note Send/Sync/Clone constraints (ReqState is cloned into func; would an Arc<Mutex<_>> slot added to ReqState survive the clone and be readable after func returns? check how request_state is created at services/api.rs:239 and passed at :274).
Return every relevant type definition location.`,
  },
  {
    key: 'webhook-paths',
    effort: 'high',
    prompt: `${COMMON}
TOPIC: Every incoming-webhook code path and every ApiEvent emit site.
1. grep for log_event across crates/router/src — list EVERY call site outside services/api.rs. For each: file:line, which flow, what event_type, status_code handling, whether it duplicates a server_wrap emit.
2. Enumerate ALL incoming webhook entry points: crates/router/src/routes/webhooks.rs (v1+v2 receive_incoming_webhook, receive_incoming_relay_webhook, network token), any recovery/revenue_recovery webhook routes (v2), crates/router/src/core/webhooks/*. Is there a separate v2 incoming_webhooks_wrapper (cfg feature v2)? Compare its ApiEvent construction to v1.
3. For each path: what ApplicationResponse variants can incoming_webhooks_core return (StatusOk, TextPlain, Json...)? Find get_webhook_api_response impls in connectors (hyperswitch_interfaces webhooks.rs:335 default + overrides) — list which connectors return non-empty/TextPlain acks. This determines whether the actual HTTP response body carries information worth logging.
4. Error path per wrapper: exactly what is logged (and what is lost) when incoming_webhooks_core errors. Where does the error emit come from?
5. Confirm/refute: inner emit hardcodes 200 (incoming.rs:111); inner not logged on error; network_token wrapper double-logs identically; relay shares the v1 wrapper.`,
  },
  {
    key: 'consumers',
    effort: 'high',
    prompt: `${COMMON}
TOPIC: Every downstream consumer of api_events rows — what breaks under each candidate change.
1. Read crates/analytics/docs/clickhouse/scripts/api_events.sql fully: tables, materialized views, engine (ReplacingMergeTree? dedup?), all columns (which come from flattened event_type: payment_id, refund_id, connector...), masked_response, TTLs.
2. Read crates/analytics/src/api_event/* (core, events, filters, metrics, types): which queries filter on payment_id / refund_id / api_flow / flow_type / status_code? Which read the response column? Would any query double-count webhooks today because of the duplicate rows, or break if the duplicate disappears?
3. Search for OpenSearch or other sinks of ApiLogs events (grep EventType::ApiLogs, api_events, ApiLogs across crates/router, crates/analytics, config/). Check config/dashboards/ (Grafana) for panels querying api_events or webhook flows.
4. Search the repo (incl. docs/, config/) for anything relying on: (a) two rows per webhook request_id, (b) response column containing entity status for webhooks, (c) inner row status_code always 200, (d) event_type Miscellaneous rows for webhooks.
5. Verdict per scenario: impact of (i) deleting the inner emit, (ii) deleting/suppressing the outer emit, (iii) single merged row with event_type=Webhooks + real status_code + response=tracker JSON, (iv) same but response=actual ack body and tracker moved elsewhere (new column / extended event_type fields — would need ClickHouse DDL change? how are new flattened fields handled by the existing table — do unknown JSON fields get dropped silently?). Check how the kafka->clickhouse ingestion maps JSON to columns (JSONEachRow? any schema registry?).`,
  },
  {
    key: 'history',
    effort: 'medium',
    prompt: `${COMMON}
TOPIC: Git archaeology — why does the inner emit exist and which emitter came first?
Use targeted git commands in ${ROOT} (repo is large; avoid full-history -p scans):
1. git log --oneline -L around the inner ApiEvent block in crates/router/src/core/webhooks/incoming.rs (the ApiEvent::new + log_event in incoming_webhooks_wrapper) — or git log --follow --oneline on the file plus git blame on those lines. Identify the commit/PR that added it and its message/intent.
2. Same for the log_event call in crates/router/src/services/api.rs server_wrap_util — when was generic API-event logging added? Did webhooks double-log from that day, or did one predate the other?
3. git blame the hardcoded status_code = 200 line and the auth_type WebhookAuth construction — deliberate or copy-paste?
4. Search commit messages: git log --oneline --grep=-i for 'api_event', 'api events', 'webhook.*log', 'double'. Any prior attempt or known-issue mention?
5. Check if ApiEventsType::Webhooks fields ever changed (git log -L on the enum variant in crates/common_utils/src/events.rs) — evidence of how hard schema evolution has been in practice (accompanying ClickHouse migrations in the same PRs?).
Report commit hashes, dates, PR numbers, and quoted intent.`,
  },
  {
    key: 'mechanisms',
    effort: 'high',
    prompt: `${COMMON}
TOPIC: Catalog of EXISTING mechanisms by which a handler can influence the ApiEvent — so a fix can reuse an idiomatic pattern instead of inventing one.
1. List all server_wrap variants/wrappers in crates/router/src/services/api.rs (server_wrap, server_wrap_util, anything like server_wrap_with_*, oss vs enterprise hooks). Any existing knob to skip or customize event logging per call?
2. How do normal typed flows get rich event_type? e.g. payments: find impl ApiEventMetric for PaymentsResponse / payment types (crates/api_models/src/events/*) — confirm the pattern is response-driven via get_api_event_type at services/api.rs:320.
3. Find every type that implements get_api_event_type returning Webhooks or NetworkTokenWebhook. Is there ANY existing response wrapper type designed to carry event metadata alongside a raw body?
4. Look at ApplicationResponse (hyperswitch_domain_models/src/api.rs) — all variants. Is there any variant carrying both a body and metadata? How does server_wrap_util turn TextPlain/StatusOk into HTTP responses (http_response_* in services/api.rs)? Could a new variant (e.g. WithApiEventMetadata { inner, event_type }) be added without breaking the HTTP layer — who matches exhaustively on ApplicationResponse (list every match site across crates, incl. compatibility layers)?
5. Any existing request-scoped mutable side channels: fields on ReqState/SessionState/AppState using Arc<Mutex/RwLock> that handlers mutate and infra reads later (e.g. add_flow_name at services/api.rs:264 mutates app_state — but is that the same instance used later?). Document exactly which instance server_wrap_util reads after func returns.
6. Check PaymentsRedirectResponseData / redirection flows (they also return non-JSON responses like Form/JsonForRedirection but still get typed events — how? via payload T's get_api_event_type? note event precedence: response .or(payload)).`,
  },
]

const factResults = (await parallel(
  FACT_TASKS.map(t => () => agent(t.prompt, { label: `facts:${t.key}`, phase: 'Facts', schema: FACTS, effort: t.effort }))
)).filter(Boolean)

log(`Facts collected: ${factResults.length}/5 reports`)

const corrections = factResults.flatMap(r => r.corrections_to_prior || [])
const factPack = factResults
  .map(r => `### FACTS: ${r.topic}\nKEY FACTS:\n- ${r.key_facts.join('\n- ')}\n\nFULL REPORT:\n${r.report}`)
  .join('\n\n---\n\n')

phase('Design')
log('Spawning 4 independent designers with distinct stances')

const STANCES = [
  {
    key: 'minimal-risk',
    brief: 'STANCE: minimal-diff, lowest-risk. Propose the smallest safe change that eliminates the duplicate row and the hardcoded-200 problem. Favor deleting code over adding plumbing. You may accept imperfect semantics if documented, but the duplicate must go and analytics must keep finding webhook rows by payment_id.',
  },
  {
    key: 'idiomatic-architecture',
    brief: 'STANCE: idiomatic long-term architecture. Find the mechanism most consistent with how this codebase already flows event metadata (response-driven ApiEventMetric, EventContext.record_info, server_wrap variants, ApplicationResponse design). If EventContext can be made the single emission path, or a new ApplicationResponse variant / server_wrap signature is the honest fix, design that. Optimize for "the next flow with handler-known metadata reuses this mechanism instead of calling log_event by hand". You may propose moderate refactors.',
  },
  {
    key: 'observability-first',
    brief: 'STANCE: observability correctness above all. The single surviving row must have: real status_code, error rows on failure (with the webhook body as request!), full latency, actual masked webhook body as request, the entity outcome (WebhookResponseTracker) preserved somewhere queryable, and event_type=Webhooks metadata. You may propose ClickHouse schema additions if justified — but quantify the migration cost using the consumers facts.',
  },
  {
    key: 'compat-first',
    brief: 'STANCE: zero downstream breakage. Existing dashboards, analytics queries, and any external Kafka consumers must see rows indistinguishable from today\'s inner (Webhooks-typed) row — same columns populated, response still containing the tracker JSON — except the duplicate Miscellaneous row disappears and status_code becomes truthful. Design the fix that provably changes nothing else.',
  },
]

const designerPrompt = s => `${COMMON}
You are ONE of four independent designers; do not hedge across multiple designs — commit to the single best design under your stance.
${s.brief}

VERIFIED FACT PACK (gathered by parallel readers; trust file:line citations here over the PRIOR block):
${factPack}

${corrections.length ? 'CORRECTIONS TO PRIOR ANALYSIS:\n- ' + corrections.join('\n- ') : ''}

Requirements for your design:
- Must handle: v1 + v2 incoming webhooks, relay webhooks, network-token webhooks, and the error path.
- Must state exactly what lands in the api_events columns: request, response, status_code, event_type (payment_id/refund_id/connector), latency — for both success and failure.
- Must not change the HTTP response bodies/acks sent to connectors.
- Give concrete Rust-level mechanism (signatures, types, where the code goes) with file:line anchors. Read any additional code you need.
- Be honest about risks and downstream impact; use the consumers facts.`

const designs = (await parallel(
  STANCES.map(s => () => agent(designerPrompt(s), { label: `design:${s.key}`, phase: 'Design', schema: DESIGN }))
)).filter(Boolean)

// Merge designs that landed on the same mechanism so we don't verify duplicates
const byKey = {}
for (const d of designs) {
  if (byKey[d.approach_key]) {
    byKey[d.approach_key].merged_stances = (byKey[d.approach_key].merged_stances || 1) + 1
  } else {
    byKey[d.approach_key] = d
  }
}
const uniqueDesigns = Object.values(byKey)
log(`Designs: ${designs.length} produced, ${uniqueDesigns.length} unique mechanisms: ${uniqueDesigns.map(d => d.approach_key).join(', ')}`)

phase('Verify')

const LENSES = [
  {
    key: 'feasibility',
    prompt: 'LENS: compile-level and runtime feasibility. Read the actual code the design touches. Try to REFUTE it: generic bounds on server_wrap_util (T: ApiEventMetric+Serialize, Q, closure Fn not FnMut), Send/Sync across .await, borrow lifetimes on req/ReqState clones, cfg(feature v1/v2) splits, exhaustive matches on ApplicationResponse across ALL crates if a variant is added, masking (Secret) handling, actix extraction. An issue is fatal only if it cannot be fixed without changing the design\'s core mechanism.',
  },
  {
    key: 'breakage',
    prompt: 'LENS: downstream breakage. Using the ClickHouse DDL, analytics queries, dashboards and Kafka pipeline, try to REFUTE the design\'s claimed downstream impact: row counts (metrics that count api_events rows for webhooks), queries reading response/status_code/payment_id, ingestion of new/changed JSON fields (are unknown fields dropped or do they break ingestion?), external consumers of the Kafka topic (flag as unknown-risk if unverifiable). Also check: does the design change what error rows look like, and does anything alert on status_code>=500?',
  },
  {
    key: 'coverage',
    prompt: 'LENS: completeness of coverage. Try to find webhook paths or emit sites the design misses: v2 incoming_webhooks_wrapper, relay v1/v2, network-token wrapper, recovery webhooks, any other closure calling log_event, the error path (is a row emitted with the webhook body when the core fails? with what event_type?), locking/auth failures before func runs, and the SetupWebhook/Skipped early-return paths in incoming_webhooks_core. Also: does the design accidentally change behavior for NON-webhook flows going through server_wrap_util?',
  },
]

const verifyPrompt = (d, l) => `${COMMON}
You are an ADVERSARIAL verifier. A design has been proposed to fix the double-logging. Your job is to try to break it under one lens. Default to skepticism; if you cannot verify a claim from the code, flag it.
${l.prompt}

THE DESIGN UNDER TEST:
approach_key: ${d.approach_key}
title: ${d.title}
summary: ${d.summary}
mechanism: ${d.mechanism}
files_touched: ${JSON.stringify(d.files_touched)}
response_field_semantics: ${d.response_field_semantics}
error_path: ${d.error_path}
coverage: ${d.coverage}
downstream_impact: ${d.downstream_impact}
claimed_risks: ${JSON.stringify(d.risks)}

FACT PACK:
${factPack}

Verdict rules: 'sound' = you tried hard and found nothing material; 'fixable-issues' = real problems but solvable within the same mechanism (list the fixes); 'fatal' = the core mechanism cannot work or provably breaks something important (prove it with file:line).`

const verified = await pipeline(
  uniqueDesigns,
  d => parallel(LENSES.map(l => () => agent(verifyPrompt(d, l), { label: `verify:${d.approach_key}:${l.key}`, phase: 'Verify', schema: VERDICT })))
    .then(vs => ({ design: d, verdicts: vs.filter(Boolean) }))
)

const verifiedClean = verified.filter(Boolean)
log(`Verification complete for ${verifiedClean.length} designs`)

phase('Judge')

const judgeInput = verifiedClean.map(v => `## DESIGN ${v.design.approach_key} (${v.design.title})${v.design.merged_stances ? ' [proposed independently by ' + v.design.merged_stances + ' designers]' : ''}
summary: ${v.design.summary}
mechanism: ${v.design.mechanism}
response_field_semantics: ${v.design.response_field_semantics}
error_path: ${v.design.error_path}
coverage: ${v.design.coverage}
downstream_impact: ${v.design.downstream_impact}
effort: ${v.design.effort}
self-claimed risks: ${JSON.stringify(v.design.risks)}
ADVERSARIAL VERDICTS:
${v.verdicts.map(x => `- [${x.lens}] ${x.verdict}: ${x.issues.join(' | ') || 'no issues'} — ${x.notes}`).join('\n')}`).join('\n\n')

const judgePromptText = `${COMMON}
You are the final judge. Rank the verified designs for fixing the webhook double-logging. Criteria, in order: (1) correctness of the resulting observability data (one truthful row incl. errors), (2) downstream safety (analytics/dashboards/Kafka), (3) maintainability/idiomatic fit with the codebase, (4) implementation risk and size. A design with a 'fatal' verdict that was not convincingly disproven cannot win. Prefer concrete evidence over designer enthusiasm. Also propose hybrid improvements: elements from losers worth grafting onto the winner.

${judgeInput}

FACT PACK (for grounding):
${factPack}`

const criticPromptText = `${COMMON}
You are a completeness critic for this whole investigation. Given the fact pack and designs below, answer: what has EVERYONE missed? Specifically probe: (a) other duplicate-emit sites beyond webhooks (grep log_event yourself); (b) whether webhook retries/redeliveries make request_id-dedup assumptions wrong; (c) outgoing webhook events (is there a symmetric problem?); (d) whether the 'response holds entity status' convention is load-bearing for support/debugging teams (search docs); (e) any v2-only webhook flow (revenue recovery?) with its own emit; (f) whether fixing status_code from hardcoded 200 to real values could trip alerting; (g) anything about multi-tenancy/platform accounts affecting merchant_id on the merged row. Return a list of findings with evidence, each marked blocking/non-blocking for the recommended fix.

DESIGNS: ${uniqueDesigns.map(d => d.approach_key + ': ' + d.summary).join('\n')}

FACT PACK:
${factPack}`

const [judgeRes, criticRes] = await parallel([
  () => agent(judgePromptText, { label: 'judge:panel', phase: 'Judge', schema: JUDGE }),
  () => agent(criticPromptText, { label: 'judge:completeness-critic', phase: 'Judge' }),
])

return {
  corrections_to_prior: corrections,
  key_facts: factResults.flatMap(r => r.key_facts.map(k => `[${r.topic}] ${k}`)),
  designs: verifiedClean.map(v => ({
    approach_key: v.design.approach_key,
    title: v.design.title,
    summary: v.design.summary,
    mechanism: v.design.mechanism,
    response_field_semantics: v.design.response_field_semantics,
    error_path: v.design.error_path,
    downstream_impact: v.design.downstream_impact,
    effort: v.design.effort,
    verdicts: v.verdicts.map(x => ({ lens: x.lens, verdict: x.verdict, issues: x.issues, notes: x.notes })),
  })),
  judge: judgeRes,
  critic: criticRes,
}