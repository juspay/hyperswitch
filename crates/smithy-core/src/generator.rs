// crates/smithy-core/generator.rs

use std::{collections::HashMap, fs, path::Path};

use crate::types::SmithyModel;

/// Generator for creating Smithy IDL files from models
pub struct SmithyGenerator {
    models: Vec<SmithyModel>,
}

impl SmithyGenerator {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    pub fn add_model(&mut self, model: SmithyModel) {
        self.models.push(model);
    }

    pub fn generate_idl(&self, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let mut namespace_models: HashMap<String, Vec<&SmithyModel>> = HashMap::new();

        for model in &self.models {
            namespace_models
                .entry(model.namespace.clone())
                .or_insert_with(Vec::new)
                .push(model);
        }

        for (namespace, models) in namespace_models {
            let filename = format!("{}.smithy", namespace.replace('.', "_"));
            let filepath = output_dir.join(filename);

            let mut content = String::new();
            content.push_str("$version: \"2\"\n\n");
            content.push_str(&format!("namespace {}\n\n", namespace));

            for model in models {
                for (shape_name, shape) in &model.shapes {
                    content.push_str(&self.generate_shape_definition(shape_name, shape));
                    content.push_str("\n\n");
                }
            }

            fs::write(filepath, content)?;
        }

        Ok(())
    }

    fn generate_shape_definition(&self, name: &str, shape: &crate::types::SmithyShape) -> String {
        match shape {
            crate::types::SmithyShape::Structure {
                members,
                documentation,
                traits,
            } => {
                let mut def = String::new();

                if let Some(doc) = documentation {
                    def.push_str(&format!("/// {}\n", doc));
                }

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("structure {} {{\n", name));

                for (member_name, member) in members {
                    if let Some(doc) = &member.documentation {
                        def.push_str(&format!("    /// {}\n", doc));
                    }

                    for smithy_trait in &member.traits {
                        def.push_str(&format!("    @{}\n", self.trait_to_string(smithy_trait)));
                    }

                    def.push_str(&format!("    {}: {}\n", member_name, member.target));
                }

                def.push_str("}");
                def
            }
            crate::types::SmithyShape::Union {
                members,
                documentation,
                traits,
            } => {
                let mut def = String::new();

                if let Some(doc) = documentation {
                    def.push_str(&format!("/// {}\n", doc));
                }

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("union {} {{\n", name));

                for (member_name, member) in members {
                    if let Some(doc) = &member.documentation {
                        def.push_str(&format!("    /// {}\n", doc));
                    }

                    for smithy_trait in &member.traits {
                        def.push_str(&format!("    @{}\n", self.trait_to_string(smithy_trait)));
                    }

                    def.push_str(&format!("    {}: {}\n", member_name, member.target));
                }

                def.push_str("}");
                def
            }
            crate::types::SmithyShape::StringEnum {
                values,
                documentation,
                traits,
            } => {
                let mut def = String::new();

                if let Some(doc) = documentation {
                    def.push_str(&format!("/// {}\n", doc));
                }

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("@enum([\n"));

                for (value_name, enum_value) in values {
                    def.push_str("    {\n");
                    def.push_str(&format!("        name: \"{}\"\n", value_name));
                    def.push_str(&format!("        value: \"{}\"\n", enum_value.name));
                    if let Some(doc) = &enum_value.documentation {
                        def.push_str(&format!("        documentation: \"{}\"\n", doc));
                    }
                    def.push_str("    }\n");
                }

                def.push_str("])\n");
                def.push_str(&format!("string {}", name));
                def
            }
            crate::types::SmithyShape::String { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("string {}", name));
                def
            }
            crate::types::SmithyShape::Integer { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("integer {}", name));
                def
            }
            crate::types::SmithyShape::Long { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("long {}", name));
                def
            }
            crate::types::SmithyShape::Boolean { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("boolean {}", name));
                def
            }
            crate::types::SmithyShape::List { member, traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("list {} {{\n", name));
                def.push_str(&format!("    member: {}\n", member.target));
                def.push_str("}");
                def
            }
        }
    }

    fn trait_to_string(&self, smithy_trait: &crate::types::SmithyTrait) -> String {
        match smithy_trait {
            crate::types::SmithyTrait::Pattern { pattern } => {
                format!("pattern(\"{}\")", pattern)
            }
            crate::types::SmithyTrait::Range { min, max } => match (min, max) {
                (Some(min), Some(max)) => format!("range(min: {}, max: {})", min, max),
                (Some(min), None) => format!("range(min: {})", min),
                (None, Some(max)) => format!("range(max: {})", max),
                (None, None) => "range".to_string(),
            },
            crate::types::SmithyTrait::Required => "required".to_string(),
            crate::types::SmithyTrait::Documentation { documentation } => {
                format!("documentation(\"{}\")", documentation)
            }
            crate::types::SmithyTrait::Length { min, max } => match (min, max) {
                (Some(min), Some(max)) => format!("length(min: {}, max: {})", min, max),
                (Some(min), None) => format!("length(min: {})", min),
                (None, Some(max)) => format!("length(max: {})", max),
                (None, None) => "length".to_string(),
            },
        }
    }
}

impl Default for SmithyGenerator {
    fn default() -> Self {
        Self::new()
    }
}