// crates/smithy-generator/main.rs

use std::path::Path;

use smithy_core::SmithyGenerator;

// Include the auto-generated model registry
include!(concat!(env!("OUT_DIR"), "/model_registry.rs"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut generator = SmithyGenerator::new();

    println!("Discovering Smithy models from workspace...");

    // Automatically discover and add all models
    let models = discover_smithy_models();
    println!("Found {} Smithy models", models.len());

    if models.is_empty() {
        println!("No SmithyModel structs found. Make sure your structs:");
        println!("  1. Derive SmithyModel: #[derive(SmithyModel)]");
        println!("  2. Are in a crate that smithy can access");
        println!("  3. Have the correct smithy attributes");
        return Ok(());
    }

    for model in models {
        println!("  Processing namespace: {}", model.namespace);
        let shape_names: Vec<_> = model.shapes.keys().collect();
        println!("    Shapes: {:?}", shape_names);
        generator.add_model(model);
    }

    println!("\nGenerating Smithy IDL files...");

    // Generate IDL files
    let output_dir = Path::new("smithy/models");
    let absolute_output_dir = std::env::current_dir()?.join(output_dir);

    println!("Output directory: {}", absolute_output_dir.display());

    generator.generate_idl(output_dir)?;

    println!("\n✅ Smithy models generated successfully!");
    println!("Files written to: {}", absolute_output_dir.display());

    // List generated files
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        println!("\nGenerated files:");
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                println!("  - {}", entry.file_name().to_string_lossy());
            }
        }
    }

    Ok(())
}
