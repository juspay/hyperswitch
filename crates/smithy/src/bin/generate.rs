// crates/smithy/bin/generate.rs

use std::path::Path;
use smithy_core::{SmithyGenerator, SmithyModelGenerator};

// Import your struct that derives SmithyModel
// Try different import paths - uncomment the one that works:
// use hyperswitch_domain_models::payment_method::CardToken;
// use hyperswitch_types::payment_method::CardToken;
// use crate::types::CardToken;

// For now, let's comment out the CardToken import and test with a simple struct
// use hyperswitch_domain_models::payment_method::CardToken;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut generator = SmithyGenerator::new();

    // Let's create a test struct directly in this file first
    use smithy_core::{SmithyModel, SmithyShape, SmithyMember, SmithyTrait};
    use std::collections::HashMap;
    
    // Create a test model manually to verify the generator works
    let mut shapes = HashMap::new();
    let mut members = HashMap::new();
    
    members.insert("test_field".to_string(), SmithyMember {
        target: "smithy.api#String".to_string(),
        documentation: Some("A test field".to_string()),
        traits: vec![SmithyTrait::Required],
    });
    
    shapes.insert("TestStruct".to_string(), SmithyShape::Structure {
        members,
        documentation: Some("A test structure".to_string()),
        traits: vec![],
    });
    
    let test_model = SmithyModel {
        namespace: "com.hyperswitch.test".to_string(),
        shapes,
    };
    
    generator.add_model(test_model);

    // TODO: Uncomment this once we fix the import
    // let card_token_model = CardToken::generate_smithy_model();
    // println!("Generated model for CardToken:");
    // println!("  Namespace: {}", card_token_model.namespace);
    // println!("  Shapes: {:?}", card_token_model.shapes.keys().collect::<Vec<_>>());
    // generator.add_model(card_token_model);
    
    // Add any other structs that derive SmithyModel here
    // generator.add_model(AnotherStruct::generate_smithy_model());

    println!("Generating Smithy models...");

    // Generate IDL files
    let output_dir = Path::new("generated/smithy");
    let absolute_output_dir = std::env::current_dir()?.join(output_dir);
    
    println!("Current working directory: {}", std::env::current_dir()?.display());
    println!("Attempting to create directory: {}", absolute_output_dir.display());
    
    generator.generate_idl(output_dir)?;

    println!(
        "Smithy models generated successfully in {}",
        absolute_output_dir.display()
    );
    Ok(())
}