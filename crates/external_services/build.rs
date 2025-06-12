#[allow(clippy::expect_used)]
/// Entry point for the build script.
///
/// Conditionally compiles protobuf files if the "dynamic_routing" or "v2" feature is enabled.
///
/// # Returns
///
/// Returns `Ok(())` if successful, or propagates errors encountered during proto compilation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(any(feature = "dynamic_routing", feature = "v2"))]
    compile_protos()?;

    Ok(())
}

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
/// Compiles protobuf files required for enabled features using tonic-build.
///
/// Dynamically selects and compiles `.proto` files based on the "dynamic_routing" and "v2" feature flags. Applies custom Rust derive attributes to specific protobuf message types and outputs generated code to the directory specified by the `OUT_DIR` environment variable.
///
/// # Errors
///
/// Returns an error if the `OUT_DIR` environment variable is missing or if protobuf compilation fails.
///
/// # Examples
///
/// ```no_run
/// // This function is intended to be called from a build script.
/// compile_protos().unwrap();
/// ```
fn compile_protos() -> Result<(), Box<dyn std::error::Error>> {
    let mut proto_files_to_compile = Vec::new();
    let proto_base_path = router_env::workspace_path().join("proto");

    #[cfg(feature = "dynamic_routing")]
    {
        // Get the directory of the current crate
        proto_files_to_compile.push(proto_base_path.join("success_rate.proto"));
        proto_files_to_compile.push(proto_base_path.join("contract_routing.proto"));
        proto_files_to_compile.push(proto_base_path.join("elimination_rate.proto"));
        proto_files_to_compile.push(proto_base_path.join("health_check.proto"));
    }

    #[cfg(feature = "v2")]
    {
        proto_files_to_compile.push(proto_base_path.join("recovery_trainer.proto"));
    }

    if !proto_files_to_compile.is_empty() {
        // Ensure proto files are unique in case a file is needed by multiple features
        proto_files_to_compile.sort();
        proto_files_to_compile.dedup();

        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

        // Compile the .proto file
        tonic_build::configure()
            .out_dir(out_dir)
            .compile_well_known_types(true)
            .type_attribute(
                "trainer.TriggerTrainingRequest",
                "#[derive(masking::Deserialize, masking::Serialize)]",
            )
            .type_attribute(
                "trainer.TriggerTrainingResponse",
                "#[derive(serde::Serialize)]",
            )
            .type_attribute(
                "trainer.GetTrainingJobStatusResponse",
                "#[derive(serde::Serialize)]",
            )
            .compile(&proto_files_to_compile, &[proto_base_path])?;
    }
    Ok(())
}
