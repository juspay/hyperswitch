// crates/smithy-core/generator.rs

use std::{collections::HashMap, fmt::Write, fs, path::Path};

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
            writeln!(content, "$version: \"2\"\n")?;
            writeln!(content, "namespace {}\n", namespace)?;

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
                )?;
                writeln!(content, "{}\n", shape_definition)?;
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
    ) -> Result<String, std::fmt::Error> {
        let resolve_target =
            |target: &str| self.resolve_type(target, current_namespace, shape_to_namespace);

        let def = match shape {
            types::SmithyShape::Structure {
                members,
                documentation,
                traits,
            } => {
                let mut def = String::new();

                if let Some(doc) = documentation {
                    writeln!(def, "/// {}", doc)?;
                }

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "structure {} {{", name)?;

                for (member_name, member) in members {
                    if let Some(doc) = &member.documentation {
                        writeln!(def, "    /// {}", doc)?;
                    }

                    for smithy_trait in &member.traits {
                        writeln!(def, "    @{}", smithy_trait)?;
                    }

                    let resolved_target = resolve_target(&member.target);
                    writeln!(def, "    {}: {}", member_name, resolved_target)?;
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
                    writeln!(def, "/// {}", doc)?;
                }

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "union {} {{", name)?;

                for (member_name, member) in members {
                    if let Some(doc) = &member.documentation {
                        writeln!(def, "    /// {}", doc)?;
                    }

                    for smithy_trait in &member.traits {
                        writeln!(def, "    @{}", smithy_trait)?;
                    }

                    let resolved_target = resolve_target(&member.target);
                    writeln!(def, "    {}: {}", member_name, resolved_target)?;
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
                    writeln!(def, "/// {}", doc)?;
                }

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "enum {} {{", name)?;

                for (value_name, enum_value) in values {
                    if let Some(doc) = &enum_value.documentation {
                        writeln!(def, "    /// {}", doc)?;
                    }
                    writeln!(def, "    {}", value_name)?;
                }

                def.push('}');
                def
            }
            types::SmithyShape::String { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "string {}", name)?;
                def
            }
            types::SmithyShape::Integer { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "integer {}", name)?;
                def
            }
            types::SmithyShape::Long { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "long {}", name)?;
                def
            }
            types::SmithyShape::Boolean { traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "boolean {}", name)?;
                def
            }
            types::SmithyShape::List { member, traits } => {
                let mut def = String::new();

                for smithy_trait in traits {
                    writeln!(def, "@{}", smithy_trait)?;
                }

                writeln!(def, "list {} {{", name)?;
                let resolved_target = resolve_target(&member.target);
                writeln!(def, "    member: {}", resolved_target)?;
                def.push('}');
                def
            }
        };
        Ok(def)
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
