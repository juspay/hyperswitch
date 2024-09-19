use std::{env, path::PathBuf};

#[allow(clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the directory of the current crate
    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let proto_file = PathBuf::from(crate_dir)
        .join("..")
        .join("..")
        .join("proto")
        .join("success_rate.proto");
    // Compile the .proto file
    tonic_build::compile_protos(proto_file).expect("Failed to compile protos ");
    Ok(())
}
