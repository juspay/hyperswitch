//! Test-only oracle for H's current Rust resolver. No server/client transport.

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use superposition_core::{eval_config, Config, MergeStrategy};

    fn check_case(index: usize, expected_id: &str) {
        let pack: Value = serde_json::from_str(include_str!("../../cases.json"))
            .expect("independently authored case pack must parse");
        let case = &pack["resolver_cases"][index];
        assert_eq!(
            case["id"], expected_id,
            "case order changed; review the harness"
        );
        let config: Config = serde_json::from_value(case["config"].clone())
            .expect("fixture must deserialize with the actual pinned types");
        let context = serde_json::from_value(case["context"].clone())
            .expect("fixture context must be an object");
        let actual = eval_config(config, context, MergeStrategy::MERGE, None, None);
        // expected_rust is independent: absent default keys deliberately differ
        // from expected_js. A JS result is never used to manufacture this oracle.
        assert_eq!(
            Value::Object(actual),
            case["expected_rust"],
            "{expected_id}"
        );
    }

    macro_rules! case {
        ($name:ident, $index:expr, $id:literal) => {
            #[test]
            fn $name() {
                check_case($index, $id);
            }
        };
    }

    case!(explicit_default_options, 0, "R01-explicit-default-options");
    case!(omitted_mandate_options, 1, "R02-omitted-mandate-options");
    case!(baseline_after_removal, 2, "R03-baseline-after-removal");
    case!(explicit_false, 3, "R04-explicit-false");
    case!(supplied_order, 4, "R05-supplied-order");
    case!(absent_mandate, 5, "R06-absent-mandate");
    case!(exact_case, 6, "R07-exact-case");
    case!(scalar_not_array, 7, "R08-scalar-not-array");
    case!(missing_default, 8, "R09-missing-default");
    case!(ordered_duplicates, 9, "R10-ordered-duplicates");
    case!(empty_options, 10, "R11-empty-options");
    case!(null_options, 11, "R12-null-options");
    case!(later_tenant, 12, "R13-later-tenant");
    case!(missing_reference, 13, "R14-missing-reference");
}
