fn main() {
    // Set thread stack size to 4 MiB for debug builds
    // Reference: https://doc.rust-lang.org/std/thread/#stack-size
    #[cfg(debug_assertions)]
    println!("cargo:rustc-env=RUST_MIN_STACK=12291456"); // 6 * 1024 * 1024 = 6 MiB

    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
