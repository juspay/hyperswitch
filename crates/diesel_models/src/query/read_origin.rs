//! Compact source locations for duplicate-read diagnostics.

use std::path::Path;

const MAX_LOCATIONS: usize = 16;

pub(super) fn capture() -> Option<String> {
    let backtrace = backtrace::Backtrace::new();
    compact(
        backtrace
            .frames()
            .iter()
            .flat_map(|frame| frame.symbols())
            .map(|symbol| (symbol.filename(), symbol.lineno())),
    )
}

/// Filter structured source locations before formatting. Missing debug
/// information yields no origin; symbol names and generic types are never
/// formatted, even for frames that are discarded.
fn compact<'a>(
    symbols: impl IntoIterator<Item = (Option<&'a Path>, Option<u32>)>,
) -> Option<String> {
    let mut locations = Vec::new();
    for (file, line) in symbols {
        let (Some(file), Some(line)) = (file, line) else {
            continue;
        };
        let file = file.to_string_lossy();
        // Exclude dependency sources even when their paths contain `crates/`.
        if file.contains("/.cargo/") || file.starts_with("/rustc/") {
            continue;
        }
        let relative = file
            .strip_prefix("crates/")
            .or_else(|| file.rsplit_once("/crates/").map(|(_, path)| path));
        let Some(relative) = relative else {
            continue;
        };
        // The query funnel and this capture helper are identical for all reads.
        if matches!(
            relative,
            "diesel_models/src/query/generics.rs" | "diesel_models/src/query/read_origin.rs"
        ) {
            continue;
        }
        let location = format!("crates/{relative}:{line}");
        if locations.last() == Some(&location) {
            continue;
        }
        if locations.len() == MAX_LOCATIONS {
            locations.push("…".to_owned());
            break;
        }
        locations.push(location);
    }
    (!locations.is_empty()).then(|| locations.join(" <- "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_application_locations_without_runtime_or_generic_noise() {
        let symbols = [
            ("/rustc/hash/library/std/src/backtrace.rs", 331),
            ("./crates/diesel_models/src/query/generics.rs", 217),
            (
                "/workspace/crates/diesel_models/src/query/read_origin.rs",
                6,
            ),
            ("./crates/diesel_models/src/query/cards_info.rs", 21),
            (
                "/home/user/.cargo/registry/src/tracing/src/instrument.rs",
                321,
            ),
            (
                "/home/user/.cargo/registry/src/example/crates/lib/src/lib.rs",
                1,
            ),
            (
                "/workspace/crates/router/src/core/payments/helpers.rs",
                6240,
            ),
            (
                "/workspace/crates/router/src/core/payments/helpers.rs",
                6240,
            ),
            (
                "crates/router/src/core/payments/operations/payment_create.rs",
                1475,
            ),
        ];

        assert_eq!(compact(symbols.map(|(file, line)| (Some(Path::new(file)), Some(line)))).as_deref(), Some(
            "crates/diesel_models/src/query/cards_info.rs:21 <- crates/router/src/core/payments/helpers.rs:6240 <- crates/router/src/core/payments/operations/payment_create.rs:1475"
        ));
    }

    #[test]
    fn missing_debug_information_and_external_sources_have_no_origin() {
        assert_eq!(compact([]), None);
        assert_eq!(
            compact([
                (None, None),
                (Some(Path::new("crates/router/src/lib.rs")), None),
                (None, Some(42)),
                (Some(Path::new("/library/core/src/future.rs")), Some(133)),
            ]),
            None
        );
    }

    #[test]
    fn long_chains_are_explicitly_truncated() {
        let symbols =
            (0..20).map(|line| (Some(Path::new("./crates/router/src/lib.rs")), Some(line)));
        let result = compact(symbols).expect("source locations");
        assert_eq!(result.split(" <- ").count(), MAX_LOCATIONS + 1);
        assert!(result.ends_with(" <- …"));
    }
}
