#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "dynamic_routing")]
    {
        // Get the directory of the current crate
        let proto_file = router_env::workspace_path()
            .join("proto")
            .join("success_rate.proto");
        // Compile the .proto file
        tonic_build::compile_protos(proto_file).expect("Failed to compile success rate proto file");
    }
    Ok(())
}
