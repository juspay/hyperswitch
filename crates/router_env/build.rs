mod vergen {
    include!("src/vergen.rs");
}

mod cargo_workspace {
    include!("src/cargo_workspace.rs");
}

fn main() {
    vergen::generate_cargo_instructions();

    #[cfg(feature = "set_workspace_members_on_build")]
    cargo_workspace::verify_cargo_metadata_format();
    #[cfg(feature = "set_workspace_members_on_build")]
    cargo_workspace::set_cargo_workspace_members_env();
}
