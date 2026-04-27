fn main() {
    let redis_rs = std::env::var("CARGO_FEATURE_REDIS_RS").is_ok();
    let fred_rs = std::env::var("CARGO_FEATURE_FRED_RS").is_ok();

    match (redis_rs, fred_rs) {
        (true, true) => panic!(
            "\n\nFeatures `redis-rs` and `fred-rs` are mutually exclusive.\n\
             Enable exactly one:\n\
             --features redis-rs   (default)\n\
             --features fred-rs\n"
        ),
        (false, false) => panic!(
            "\n\nExactly one of `redis-rs` or `fred-rs` must be enabled.\n\
             Neither is currently active.\n"
        ),
        _ => {}
    }
}
