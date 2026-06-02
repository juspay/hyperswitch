fn main() {
    // Set thread stack size to 256 MiB for debug builds
    // Reference: https://doc.rust-lang.org/std/thread/#stack-size
    #[cfg(debug_assertions)]
    println!("cargo:rustc-env=RUST_MIN_STACK=268435456"); // 256 * 1024 * 1024 = 256 MiB

    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
