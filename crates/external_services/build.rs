fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compilation for revenue recovery protos
    #[cfg(feature = "revenue_recovery")]
    {
        let proto_base_path = router_env::workspace_path().join("proto");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
        let recovery_proto_files = [proto_base_path.join("recovery_decider.proto")];

        #[allow(clippy::expect_used, clippy::unwrap_in_result)]
        tonic_build::configure()
            .out_dir(&out_dir)
            .compile_well_known_types(true)
            .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
            .compile_protos(&recovery_proto_files, &[&proto_base_path])
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
        #[allow(clippy::expect_used, clippy::unwrap_in_result)]
        tonic_build::configure()
            .out_dir(out_dir)
            .compile_protos(
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
