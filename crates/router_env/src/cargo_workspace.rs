/// Sets the `CARGO_WORKSPACE_MEMBERS` environment variable to include a comma-separated list of
/// names of all crates in the current cargo workspace.
///
/// This function should be typically called within build scripts, so that the environment variable
/// is available to the corresponding crate at compile time.
///
/// # Panics
///
/// Panics if running the `cargo metadata` command fails.
#[allow(clippy::expect_used)]
/// Retrieves the cargo workspace members and sets them as environment variables for the Rust compiler.
pub fn set_cargo_workspace_members_env() {
    use std::io::Write;

    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Failed to obtain cargo metadata");
    let workspace_members = metadata.workspace_members;

    let workspace_members = workspace_members
        .iter()
        .map(|package_id| {
            package_id
                .repr
                .split_once(' ')
                .expect("Unknown cargo metadata package ID format")
                .0
        })
        .collect::<Vec<_>>()
        .join(",");

    writeln!(
        &mut std::io::stdout(),
        "cargo:rustc-env=CARGO_WORKSPACE_MEMBERS={workspace_members}"
    )
    .expect("Failed to set `CARGO_WORKSPACE_MEMBERS` environment variable");
}

/// Verify that the cargo metadata workspace members format matches that expected by
/// [`set_cargo_workspace_members_env`] to set the `CARGO_WORKSPACE_MEMBERS` environment variable.
///
/// This function should be typically called within build scripts, before the
/// [`set_cargo_workspace_members_env`] function is called.
///
/// # Panics
///
/// Panics if running the `cargo metadata` command fails, or if the workspace members package ID
/// format cannot be determined.
pub fn verify_cargo_metadata_format() {
    #[allow(clippy::expect_used)]
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
}

/// Obtain the crates in the current cargo workspace as a `HashSet`.
///
/// This macro requires that [`set_cargo_workspace_members_env()`] function be called in the
/// build script of the crate where this macro is being called.
///
/// # Errors
///
/// Causes a compilation error if the `CARGO_WORKSPACE_MEMBERS` environment variable is unset.
#[macro_export]
macro_rules! cargo_workspace_members {
    () => {
        std::env!("CARGO_WORKSPACE_MEMBERS")
            .split(',')
            .collect::<std::collections::HashSet<&'static str>>()
    };
}
