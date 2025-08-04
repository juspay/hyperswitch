#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compilation for v2 protos
    #[cfg(feature = "revenue_recovery")]
    {
        let proto_base_path = router_env::workspace_path().join("proto");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
        let v2_proto_files = [proto_base_path.join("recovery_decider.proto")];
        tonic_build::configure()
            .out_dir(&out_dir)
            .compile_well_known_types(true)
            .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
            .type_attribute(
                "google.protobuf.Timestamp",
                "#[derive(serde::Serialize, serde::Deserialize)]",
            )
            .compile_protos(&v2_proto_files, &[&proto_base_path])
            .expect("Failed to compile revenue-recovery proto files");
    }

    // Compilation for dynamic_routing protos
    #[cfg(feature = "dynamic_routing")]
    {
        let proto_base_path = router_env::workspace_path().join("proto");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
        let dr_proto_files = [
            proto_base_path.join("success_rate.proto"),
            proto_base_path.join("contract_routing.proto"),
            proto_base_path.join("elimination_rate.proto"),
            proto_base_path.join("health_check.proto"),
        ];
        println!(
            "Compiling dynamic_routing proto files: {:?}",
            dr_proto_files
        );
        tonic_build::configure()
            .out_dir(&out_dir)
            .compile_protos(&dr_proto_files, &[&proto_base_path])
            .expect("Failed to compile dynamic routing proto files");
    }
    Ok(())
}
