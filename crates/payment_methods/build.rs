#[cfg(all(
    feature = "required_fields_sync",
    any(not(feature = "v1"), feature = "v2")
))]
compile_error!("required_fields_sync supports v1 only; enable v1 and disable v2");

fn main() {
    #[cfg(feature = "required_fields_sync")]
    {
        let features = std::env::vars()
            .filter_map(|(key, _)| key.strip_prefix("CARGO_FEATURE_").map(str::to_owned))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",");
        println!("cargo:rustc-env=REQUIRED_FIELDS_FEATURE_KEYS={features}");
    }
}
