mod vergen {
    include!("src/vergen.rs");
}

mod cargo_workspace {
    include!("src/cargo_workspace.rs");
}

/// This function is the entry point of the Rust program. It calls the `generate_cargo_instructions` function from the `vergen` crate to generate the cargo instructions. Then it calls the `verify_cargo_metadata_format` function and `set_cargo_workspace_members_env` function from the `cargo_workspace` crate to perform additional operations related to cargo metadata format verification and setting cargo workspace members environment.
fn main() {
    vergen::generate_cargo_instructions();

    cargo_workspace::verify_cargo_metadata_format();
    cargo_workspace::set_cargo_workspace_members_env();
}
