include!("src/vergen.rs");

fn main() {
    generate_cargo_instructions();

    #[allow(clippy::expect_used)] // Safety: panicking in build scripts is okay for us
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Failed to obtain cargo metadata");
    let workspace_members = metadata
        .workspace_members
        .iter()
        .map(|package_id| {
            #[allow(clippy::expect_used)] // Safety: panicking in build scripts is okay for us
            package_id
                .repr
                .split_once(' ')
                .expect("Unknown cargo metadata package ID format")
                .0
        })
        .collect::<Vec<_>>()
        .join(",");
    println!("cargo:rustc-env=CARGO_WORKSPACE_MEMBERS={workspace_members}",);
}
