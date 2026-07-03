fn main() {
    // NOTE: the worker-thread stack size for debug builds is set at *runtime*
    // in `src/bin/router.rs` (via `RUST_MIN_STACK`), not here. A
    // `cargo:rustc-env` instruction only affects the compile-time environment
    // (`env!`) and does not size threads spawned by `std`/Tokio/actix at
    // runtime, so it cannot prevent stack overflows in deep async flows.
    // Reference: https://doc.rust-lang.org/std/thread/#stack-size

    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
