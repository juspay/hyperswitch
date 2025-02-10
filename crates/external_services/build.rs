#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
