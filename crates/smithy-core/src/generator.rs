// crates/smithy-core/generator.rs

use std::{collections::HashMap, fs, path::Path};

use crate::types::{self as types, SmithyModel};

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

        let mut namespace_models: HashMap<&str, Vec<&SmithyModel>> = HashMap::new();
        let mut shape_to_namespace: HashMap<&str, &str> = HashMap::new();

        // First, build a map of all shape names to their namespaces
        for model in &self.models {
            for shape_name in model.shapes.keys() {
                shape_to_namespace.insert(shape_name, model.namespace.as_str());
            }
        }

        // Group models by namespace for file generation
        for model in &self.models {
            namespace_models
                .entry(model.namespace.as_str())
                .or_default()
                .push(model);
        }

        for (namespace, models) in namespace_models {
            let filename = format!("{}.smithy", namespace.replace('.', "_"));
            let filepath = output_dir.join(filename);

            let mut content = String::new();
            content.push_str("$version: \"2\"\n\n");
            content.push_str(&format!("namespace {}\n\n", namespace));

            // Collect all unique shape definitions for the current namespace
            let mut shapes_in_namespace = HashMap::new();
            for model in models {
                for (shape_name, shape) in &model.shapes {
                    shapes_in_namespace.insert(shape_name, shape);
                }
            }

            // Generate definitions for each shape in the namespace
            for (shape_name, shape) in shapes_in_namespace {
                let shape_definition = self.generate_shape_definition(
                    shape_name,
                    shape,
                    namespace,
                    &shape_to_namespace,
                ));
                content.push_str("\n\n");
            }

            fs::write(filepath, content)?;
        }

        Ok(())
    }

    fn generate_shape_definition(
        &self,
        name: &str,
        shape: &types::SmithyShape,
        current_namespace: &str,
        shape_to_namespace: &HashMap<&str, &str>,
    ) -> String {
        let resolve_target =
            |target: &str| self.resolve_type(target, current_namespace, shape_to_namespace);

        match shape {
            types::SmithyShape::Structure {
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

                    let resolved_target = resolve_target(&member.target);
                    def.push_str(&format!("    {}: {}\n", member_name, resolved_target));
                }

                def.push('}');
                def
            }
            types::SmithyShape::Union {
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

                    let resolved_target = resolve_target(&member.target);
                    def.push_str(&format!("    {}: {}\n", member_name, resolved_target));
                }

                def.push('}');
                def
            }
            types::SmithyShape::Enum {
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

                def.push_str(&format!("enum {} {{\n", name));

                for (value_name, enum_value) in values {
                    if let Some(doc) = &enum_value.documentation {
                        def.push_str(&format!("    /// {}\n", doc));
                    }
                    def.push_str(&format!("    {}\n", value_name));
                }

                def.push('}');
                def
            }
            types::SmithyShape::String { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("string {}", name));
                def
            }
            types::SmithyShape::Integer { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("integer {}", name));
                def
            }
            types::SmithyShape::Long { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("long {}", name));
                def
            }
            types::SmithyShape::Boolean { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("boolean {}", name));
                def
            }
            types::SmithyShape::List { member, traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    def.push_str(&format!("@{}\n", self.trait_to_string(smithy_trait)));
                }

                def.push_str(&format!("list {} {{\n", name));
                let resolved_target = resolve_target(&member.target);
                def.push_str(&format!("    member: {}\n", resolved_target));
                def.push('}');
                def
            }
        }
    }

    fn resolve_type(
        &self,
        target: &str,
        current_namespace: &str,
        shape_to_namespace: &HashMap<&str, &str>,
    ) -> String {
        // If the target is a primitive or a fully qualified Smithy type, return it as is
        if target.starts_with("smithy.api#") {
            return target.to_string();
        }

        // If the target is a custom type, resolve its namespace
        if let Some(target_namespace) = shape_to_namespace.get(target) {
            if *target_namespace == current_namespace {
                // The type is in the same namespace, so no qualification is needed
                target.to_string()
            } else {
                // The type is in a different namespace, so it needs to be fully qualified
                format!("{}#{}", target_namespace, target)
            }
        } else {
            // If the type is not found in the shape map, it might be a primitive
            // or an unresolved type. For now, return it as is.
            target.to_string()
        }
    }
}

impl Default for SmithyGenerator {
    fn default() -> Self {
        Self::new()
    }
}
