fn main() {
    // Set thread stack size to 10 MiB for debug builds
    // Reference: https://doc.rust-lang.org/std/thread/#stack-size
    #[cfg(debug_assertions)]
    println!("cargo:rustc-env=RUST_MIN_STACK=10485760"); // 10 * 1024 * 1024 = 10 MiB

    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
