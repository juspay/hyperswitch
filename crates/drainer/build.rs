/// This method is the main entry point of the program. It conditionally calls the `generate_cargo_instructions` function from the `router_env::vergen` module if the "vergen" feature is enabled.
fn main() {
    #[cfg(feature = "vergen")]
    router_env::vergen::generate_cargo_instructions();
}
