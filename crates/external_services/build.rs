#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "dynamic_routing")]
    {
        // Get the directory of the current crate
        let success_rate_proto_file = router_env::workspace_path()
            .join("proto")
            .join("success_rate.proto");

        let health_check_proto_file = router_env::workspace_path()
            .join("proto")
            .join("health_check.proto");
        // Compile the .proto file
        tonic_build::compile_protos(success_rate_proto_file)
            .expect("Failed to compile success rate proto file");
        tonic_build::compile_protos(health_check_proto_file)
            .expect("Failed to compile gRPC health check proto file");
    }
    Ok(())
}
