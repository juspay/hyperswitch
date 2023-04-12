include!("src/vergen.rs");

fn main() {
    generate_cargo_instructions();

    #[allow(clippy::expect_used)] // Safety: panicking in build scripts is okay for us
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Failed to obtain cargo metadata");
    let workspace_members = metadata.workspace_members;

    let package_id_entry_prefix =
        format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    assert!(
        workspace_members
            .iter()
            .any(|package_id| package_id.repr.starts_with(&package_id_entry_prefix)),
        "Unknown workspace members package ID format. \
         Please run `cargo metadata --format-version=1 | jq '.workspace_members'` and update this \
         build script to match the updated package ID format."
    );

    let workspace_members = workspace_members
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
    println!("cargo:rustc-env=CARGO_WORKSPACE_MEMBERS={workspace_members}");
}
