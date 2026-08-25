# Deja: sites deliberately left un-instrumented

Most of the record/replay work is about choosing where a boundary goes. An equally
load-bearing set of decisions is where a boundary deliberately does **not** go. Those
choices are invisible in the code — an absent boundary looks identical to an oversight —
so they are recorded here, and the call sites carry a one-line pointer back to this file.

Two failure modes motivate every entry below.

**Nesting.** A boundary that wraps other boundaries is substituted as a unit on replay.
Substituting the outer call returns the recorded reply *without executing the body*, so
the inner boundaries never run and their events are silently missing from the replay. An
outer boundary over instrumented inner calls therefore does not add fidelity; it removes it.

**Recording a secret that the seam already handles.** Where non-determinism enters at a
narrow seam, instrumenting the seam records the minimum. Instrumenting the surrounding
computation instead records its output — which may be the very secret the seam was chosen
to keep off disk.

## `crypto_operation` — `hyperswitch_domain_models::type_encryption`

Not a boundary. `crypto_operation` is pure computation over inputs that are all already
deterministic on replay, so it reproduces byte-identically when re-run live. Instrumenting
it would only record non-`serde` metadata that cannot be substituted, forcing a dishonest
verdict exclusion.

The single source of crypto non-determinism — the AEAD nonce — is recorded and replayed at
its own seam (`common_utils::crypto::NonceSequence::new`, a `deja::id` boundary). Given
substituted inputs the rest of the chain is pure:

- the master key is config, identical across record and replay;
- `DecryptLocally` of the merchant key store derives the DEK via `GcmAes256::decode_message`,
  which has no randomness;
- the encrypted ciphertext (DEK and PII) arrives from substituted DB reads;
- the plaintext arrives from the kernel-re-driven request.

Recording the output instead would write the plaintext DEK — the merchant's data key — into
the event log, where recording the seam does not.

> **Open item (tracked, not resolved here).** The last paragraph does not currently hold
> globally: `generate_aes256_key` *is* instrumented (`router::services`), and that is the DEK
> generator used by `db::merchant_key_store`, `db::merchant_connector_account` and `db::events`.
> The plaintext DEK therefore does reach the tape at that seam. Under the full-fidelity capture
> policy this may be intended, but the "keeps it off disk" reasoning above is not a guarantee
> the reader should rely on. Tape protection is a separate, deferred workstream.

## `delete_multiple_keys` — `redis_interface` (fred and redis-rs)

Not a boundary. It is a thin wrapper that maps `delete_key` over each key, and `delete_key`
is already hermetic. An outer boundary would nest: substituting the wrapper on replay
returns the recorded reply without executing, skipping the inner `delete_key` calls and
omitting their recorded `DEL` events. The inner boundaries carry record and replay.

## `perform_locking_action` and `free_lock_action` — `router::core::api_locking`

Not boundaries. Acquiring a lock is a redis SETNX-and-get retry loop
(`set_multiple_keys_if_not_exists_and_get_values`); releasing one is a `delete_key` on the
lock key. Neither is an independent source of non-determinism. Once the redis boundary is
hermetic the lock outcome replays deterministically from the recorded redis reply, so a
separate `deja::lock` boundary would only double-record — and substituting it would skip,
and therefore omit, the inner redis call.
