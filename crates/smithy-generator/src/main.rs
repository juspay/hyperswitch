// crates/smithy-generator/main.rs

use std::path::Path;

use router_env::logger;
use smithy_core::SmithyGenerator;

// Include the auto-generated model registry
include!(concat!(env!("OUT_DIR"), "/model_registry.rs"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut generator = SmithyGenerator::new();

    logger::info!("Discovering Smithy models from workspace...");

    // Automatically discover and add all models
    let models = discover_smithy_models();
    logger::info!("Found {} Smithy models", models.len());

    if models.is_empty() {
        logger::info!("No SmithyModel structs found. Make sure your structs:");
        logger::info!("  1. Derive SmithyModel: #[derive(SmithyModel)]");
        logger::info!("  2. Are in a crate that smithy can access");
        logger::info!("  3. Have the correct smithy attributes");
        return Ok(());
    }

    for model in models {
        logger::info!("  Processing namespace: {}", model.namespace);
        let shape_names: Vec<_> = model.shapes.keys().collect();
        logger::info!("    Shapes: {:?}", shape_names);
        generator.add_model(model);
    }

    logger::info!("Generating Smithy IDL files...");

    // Generate IDL files
    let output_dir = Path::new("smithy/models");
    let absolute_output_dir = std::env::current_dir()?.join(output_dir);

    logger::info!("Output directory: {}", absolute_output_dir.display());

    generator.generate_idl(output_dir)?;

    logger::info!("✅ Smithy models generated successfully!");
    logger::info!("Files written to: {}", absolute_output_dir.display());

    // List generated files
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        logger::info!("Generated files:");
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                logger::info!("  - {}", entry.file_name().to_string_lossy());
            }
        }
    }

    Ok(())
}
