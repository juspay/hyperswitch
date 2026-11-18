# Step 8 — immutable recovery journal

Local implementation at H baseline `480e15c04a1524b7c297d1a8df761fcef55be4fa`,
branch `feat/superposition-required-fields-foundations`, 18 November 2026.
Only Step 8 is in this change. [Executed checks](checks.md) and [machine-readable evidence](evidence.json)
distinguish local results from missing certification. Hosted apply remains disabled.

## Implemented behavior

`PreparedSnapshot::capture` accepts the sealed complete inventory and accepted read-only
plan. It checks target/instance, inventory, bundle, grants, prerequisite and projection
hashes. The snapshot retains the full bundle (including effective inputs, policy, catalog,
build/features and release), raw rows/pages/workspace/version exports, exact action
before/after images, preservation hashes, ordered actions, accepted SDK report bytes,
approval, cases and contract material. The accepted planner now retains evidence it
previously discarded; its planning and approval rules are unchanged.

Recovery identity includes endpoint digest, coordinator identifier, executor image and
retained recovery image. Unreadable and server-managed attributes are explicit lists.
Functions/schema references remain in the original rows; the planner still rejects
unsupported functions rather than pretending their definitions were discovered.
Pending creates retain their ownership intent and condition without inventing server IDs.

`RunJournal` uses a narrow conditional-create/read interface. A prepared run writes the
target/instance's first-adoption baseline, run-scoped prepared record and immutable unresolved
marker before returning. Later runs must reference the exact baseline; an existing baseline
is never silently adopted or overwritten. Each append includes sequence, previous digest,
prepared digest and either a frozen request intent or observation. Every successful create
must return an immutable version and pass bounded exact checksum/version/retention read-back
before a `DurableReceipt` is issued. Errors and unknown storage acknowledgements stop that
writer handle; they do not trigger a hidden retry. Receipts are not publicly constructible
or deserializable.

Requests retain method, relative path, noncredential content-type and exact UTF-8 JSON
body bytes, including whitespace, decimal lexemes and ordered arrays. Their action indexes
must cover the plan once in order. This step does **not** implement or certify the API
serializer that maps an action to a wire request. The wire-contract digest is retained
evidence for that later adapter, not a dispatch authorization. No automatic batching occurs.

Transport observations retain status, response-body hash, version and request ID or an
explicit transport outcome. Direct-state images and verification evidence can be appended
for later recovery/three-way restoration. A 200, 512, timeout, direct-state observation or
verification attachment does not resolve the run here. All intents remain unresolved; no
retry, takeover, apply, restoration or lease is authorized by this module.

An independent reader can discover a run from registered target/run identity alone. It
checks the prepared marker, baseline and bounded event slots, rejects gaps, overflow and
hash/binding failures, and reconstructs exact requests and pending ownership. A persisted
`UnresolvedRunRecord` provides the future Step 9 minimum-head anchor, detecting rollback
below an acknowledged event. Unknown append outcomes can be recovered even when the last
receipt was never returned. There is no constructor that turns recovered history back into
a writer. Step 9 still needs the authoritative active-run record and writer exclusion.

Limits are explicit refusals: 64 MiB per prepared/baseline object, 32 frozen requests,
512 KiB per request body, 4 MiB per event, 128 events plus an overflow sentinel, and
160 MiB aggregate recovery bytes. Store operations time out after five seconds; recovery
has a 30-second scan budget. No partial history is reported as complete. The writer also
enforces the aggregate bound so it cannot append beyond the reader's capacity.

## Source evidence and reuse decision

At this revision, `external_services::file_storage::AwsFileStorageClient` delegates to
`cloud_services::storage::s3::S3Storage`; the plan's old direct AWS implementation reference
is stale. Relevant current sources:

- `crates/external_services/src/file_storage.rs:90`: upload returns `()`, and the same
  interface permits deletion and presigned downloads; no immutable version, conditional
  create, protected retention or bounded read contract.
- `crates/external_services/src/file_storage/aws_s3.rs:25`: delegates construction and
  file operations to the shared cloud client.
- `crates/cloud_services/src/storage/s3.rs:105`: ambient SDK configuration loading;
  `:117` uses ordinary `put_object`, discards response metadata, `:129` permits deletion,
  and `:142` collects the entire read. None was instantiated or called.
- `crates/external_services/src/file_storage/file_system.rs:31`: `File::create` truncates,
  with no file/directory sync; deletion and unbounded reads are available.

These are source findings, not claims about bucket policies or a running provider. The
existing adapters remain unchanged and are **not registered** for journaling. Ordinary
upload support cannot be promoted to a durable receipt. No new dependency, settings field,
production storage adapter or CLI command was added. Error-stack, existing digest/input
types and the lossless JSON parser are reused. `CertifiedJournalStore::from_registry`
always returns `CertificationPending`; validating a list of required capabilities does
not create a certificate.

## Restricted artifacts and sanitized reports

Raw tenant rules/configuration stay in restricted snapshots, never public reports. Reports
contain opaque digests, counts, synthetic/certified-storage scope, unresolved status and
`hosted_apply: disabled`; no URLs, presigned links, target names or request bodies. The
request API has no authorization/cookie header slot. Known credential-shaped keys/values
and caller-supplied ephemeral credential sentinels cause refusal **before storage**, rather
than destructive in-place redaction of an image needed for restore. Store error attachments
are discarded and replaced by bounded codes. Restricted values' Debug output shows size only.

Pattern/sentinel rejection is defense in depth, not proof that arbitrary configuration
contains no unknown secret. Q6 must approve input classification and credential exclusions,
encrypted restricted storage, decryption access and audit logging. Do not load hosted
credentials to exercise this code or place real snapshots in this documentation tree.

## Proposed retention/access contract and open decisions

Every object requires protected hold and retention of at least the greater of 90 days and
the stated rollback window. This is the plan's proposed floor, **not owner approval**.
First-adoption, active-release and unresolved artifacts stay held indefinitely. There is no
delete/unhold/expiry API in this step. Any future hold release needs an approved lifecycle
process; timestamps are supplied inputs, not ambient clock reads.

Q6 remains open: select and explicitly authorize a disposable test store; certify exact
provider/adapter versions, create-if-absent races, immutable version identity, complete
strong reads, lost acknowledgements, encryption/key grants, access logs, deny-overwrite/delete
across identities, real retention/expiry and protected holds, then prove separate recovery
role access with a retained recovery image. A local filesystem owner can bypass `create_new`
and delete files; the fixture demonstrates that gap. Same-user child-process recovery is
not cross-principal access or power-loss/media durability certification. Hashes detect
changes relative to trusted anchors; they cannot authenticate a malicious storage provider.

Q2 wire/server contracts, Q3/Q4 authority and exclusion, Step 11 V1–V4 and Step 12 recovery/
restoration remain independent blockers. The provisional planner is not promoted to an
executable plan by taking its snapshot. No successful stub, force flag or enabled apply exists.

After review, **Step 9 local authority/coordination work** is next on a separate request;
Q6 certification can proceed only with a named authorized disposable store. No Step 9 work
was started. Suggested H commit: `feat: journal required-fields plans and request intentions`.
