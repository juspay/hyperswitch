// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
//! Detached work must carry the request span.
//!
//! deja attributes a boundary to a correlation by reading the tracing span the
//! work is running under (`DejaCorrelationLayer`, keyed on the `request_id`
//! field the ingress root span stamps). A `tokio::spawn` whose future carries no
//! span hands the child neither the correlation nor anything to resolve one
//! from, so every boundary it crosses records uncorrelated — or, once the
//! request's recording is closed, records nothing at all and leaves the tape
//! ending mid-correlation with no marker to say why.
//!
//! So the rule is the tracing convention, not a deja-specific one: **a spawned
//! future is instrumented**. `.in_current_span()` for work that belongs to the
//! request, or `.instrument(some_span)` where the site already builds its own.
//! This test inspects the argument of every `tokio::spawn` in the crates that
//! serve a request and fires only when one is bare, so adding a correctly
//! instrumented spawn needs no change here.
//!
//! Crates outside the request path (`drainer`, `scheduler`, the redis and
//! database connection pools) are not scanned: nothing there runs under a
//! correlation, so nothing there can truncate one.
//!
//! Carrying the request span attributes the work; it does not give the child a
//! span of its own, so its boundaries still address below rank 2. That is a
//! deliberate deferral — naming those spans is address-additive and measurable
//! on its own — not an oversight.

use std::{collections::BTreeMap, fs, path::Path};

/// Crates in which an HTTP request correlation is live.
const SCANNED_ROOTS: &[&str] = &[
    "crates/router/src",
    "crates/router_env/src",
    "crates/hyperswitch_interfaces/src",
];

/// Files permitted to spawn an uninstrumented future, with the count in that
/// file and the reason no correlation is being lost.
const ALLOWED_BARE: &[(&str, usize, &str)] = &[
    (
        "crates/router/src/routes/metrics/bg_metrics_collector.rs",
        1,
        "process-lifetime: the background metrics collector, spawned at startup \
         with no request in scope",
    ),
    (
        "crates/router/src/db/events.rs",
        1,
        "test-only: the concurrent webhook-creation test in this module's \
         #[cfg(test)] block",
    ),
    (
        "crates/router/src/services/authentication/decision.rs",
        1,
        "the #[cfg(not(feature = \"deja\"))] arm of spawn_tracked_job; the deja \
         arm carries context through deja::spawn_fork, so nothing is lost from a \
         recording. The bare arm costs log context only, and is worth tidying \
         separately — changing it under the deja feature would move an existing \
         fork region and re-address the tape",
    ),
];

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/<name> is two levels below the workspace root")
}

/// The argument text of each `tokio::spawn(...)` in `source`, by paren matching,
/// with the 1-based line the call starts on. Matching the parens rather than
/// scanning lines is what lets the check look at the spawned expression itself
/// instead of guessing from nearby text.
fn spawned_arguments(source: &str) -> Vec<(usize, String)> {
    const CALL: &str = "tokio::spawn(";
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = source[from..].find(CALL) {
        let start = from + offset;
        let line = source[..start].matches('\n').count() + 1;
        let after_call = start + CALL.len();
        // Walk to the paren that closes the call. An unbalanced tail leaves `end`
        // just past `CALL`, which still advances `from` — no rescan, no loop.
        let mut depth = 1usize;
        let mut end = after_call;
        for (index, character) in source[after_call..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                end = after_call + index + character.len_utf8();
                break;
            }
        }
        found.push((line, source[start..end].to_owned()));
        from = end;
    }
    found
}

fn is_instrumented(argument: &str) -> bool {
    argument.contains(".in_current_span()") || argument.contains(".instrument(")
}

fn collect(root: &Path, relative: &Path, bare: &mut BTreeMap<String, Vec<usize>>) {
    let directory = root.join(relative);
    let entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        let child = relative.join(entry.file_name());
        if path.is_dir() {
            collect(root, &child, bare);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            for (line, argument) in spawned_arguments(&source) {
                if !is_instrumented(&argument) {
                    bare.entry(child.to_string_lossy().replace('\\', "/"))
                        .or_default()
                        .push(line);
                }
            }
        }
    }
}

#[test]
fn every_spawned_future_on_the_request_path_carries_a_span() {
    let root = workspace_root();
    let mut bare = BTreeMap::new();
    for scanned in SCANNED_ROOTS {
        collect(root, Path::new(scanned), &mut bare);
    }

    let allowed: BTreeMap<&str, (usize, &str)> = ALLOWED_BARE
        .iter()
        .map(|(path, count, reason)| (*path, (*count, *reason)))
        .collect();

    let mut problems = Vec::new();
    for (path, lines) in &bare {
        match allowed.get(path.as_str()) {
            None => problems.push(format!(
                "{path}: spawns an uninstrumented future at line(s) {lines:?}.\n    \
                 Work detached from a request must carry its span, or deja cannot \
                 attribute it and its boundaries record uncorrelated — or, past \
                 the request's teardown, not at all. Add `.in_current_span()` (or \
                 `.instrument(..)` if the site builds its own span). If no request \
                 is in scope here, add the file to ALLOWED_BARE with that reason."
            )),
            Some((allowed_count, reason)) if allowed_count != &lines.len() => {
                problems.push(format!(
                    "{path}: {} uninstrumented spawn(s) at {lines:?}, ALLOWED_BARE \
                     says {allowed_count} ({reason}).\n    Check the new one \
                     against the same question before updating the count.",
                    lines.len()
                ));
            }
            Some(_) => {}
        }
    }
    for (path, (count, reason)) in &allowed {
        if !bare.contains_key(*path) {
            problems.push(format!(
                "{path}: ALLOWED_BARE expects {count} uninstrumented spawn(s) \
                 ({reason}) but the file has none. Drop the stale entry."
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "detached work is losing its request span:\n  - {}",
        problems.join("\n  - ")
    );
}
