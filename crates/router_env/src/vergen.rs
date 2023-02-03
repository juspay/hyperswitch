/// Configures the [`vergen`][::vergen] crate to generate the `cargo` build instructions.
///
/// This function should be typically called within build scripts to generate `cargo` build
/// instructions for the corresponding crate.
///
/// # Panics
///
/// Panics if `vergen` fails to generate `cargo` build instructions.
#[allow(clippy::expect_used)]
pub fn generate_cargo_instructions() {
    use std::io::Write;

    use vergen::EmitBuilder;

    EmitBuilder::builder()
        .cargo_debug()
        .cargo_opt_level()
        .cargo_target_triple()
        .git_commit_timestamp()
        .git_describe(true, true)
        .git_sha(true)
        .rustc_semver()
        .rustc_commit_hash()
        .emit()
        .expect("Failed to generate `cargo` build instructions");

    writeln!(
        &mut std::io::stdout(),
        "cargo:rustc-env=CARGO_PROFILE={}",
        std::env::var("PROFILE").expect("Failed to obtain `cargo` profile")
    )
    .expect("Failed to set `CARGO_PROFILE` environment variable");
}
