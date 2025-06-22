#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compilation for v2 protos
    #[cfg(all(feature = "v2", feature = "revenue_recovery"))]
    {
        let proto_base_path = router_env::workspace_path().join("proto");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
        let v2_proto_files = [proto_base_path.join("recovery_trainer.proto")];
        println!("Compiling v2 proto files: {:?}", v2_proto_files);
        tonic_build::configure()
            .out_dir(&out_dir)
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
            .compile(&v2_proto_files, &[&proto_base_path])
            .expect("Failed to compile revenue-recovery proto files");
    }

    // Compilation for dynamic_routing protos
    #[cfg(feature = "dynamic_routing")]
    {
        // Get the directory of the current crate

        let proto_path = router_env::workspace_path().join("proto");
        let success_rate_proto_file = proto_path.join("success_rate.proto");
        let contract_routing_proto_file = proto_path.join("contract_routing.proto");
        let elimination_proto_file = proto_path.join("elimination_rate.proto");
        let health_check_proto_file = proto_path.join("health_check.proto");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

        // Compile the .proto file
        tonic_build::configure()
            .out_dir(out_dir)
            .compile(
                &[
                    success_rate_proto_file,
                    health_check_proto_file,
                    elimination_proto_file,
                    contract_routing_proto_file,
                ],
                &[proto_path],
            )
            .expect("Failed to compile proto files");
    }
    Ok(())
}
