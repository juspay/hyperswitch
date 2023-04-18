fn main() {
    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
