fn main() {
    // Set thread stack size to 4 MiB for debug builds
    // Reference: https://doc.rust-lang.org/std/thread/#stack-size
    #[cfg(debug_assertions)]
    println!("cargo:rustc-env=RUST_MIN_STACK=4194304"); // 4 * 1024 * 1024 = 4 MiB

    router_env::vergen::generate_cargo_instructions();
}
