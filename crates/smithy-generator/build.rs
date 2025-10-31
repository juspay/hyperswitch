// crates/smithy-generator/build.rs

use std::{collections::HashSet, fs, path::Path};

use regex::Regex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../");
    run_build()
}

fn run_build() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = get_workspace_root()?;

    let mut smithy_models = Vec::new();
    let mut seen_models = HashSet::new();

    // Scan all crates in the workspace for SmithyModel derives
    let crates_dir = workspace_root.join("crates");
    if let Ok(entries) = fs::read_dir(&crates_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let crate_path = entry.path();
                let crate_name = match crate_path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => {
                        println!(
                            "cargo:warning=Skipping crate with invalid path: {}",
                            crate_path.display()
                        );
                        continue;
                    }
                };

                // Skip the smithy crate itself to avoid self-dependency
                if crate_name == "smithy"
                    || crate_name == "smithy-core"
                    || crate_name == "smithy-generator"
                {
                    continue;
                }

                if let Err(e) = scan_crate_for_smithy_models(
                    &crate_path,
                    &crate_name,
                    &mut smithy_models,
                    &mut seen_models,
                ) {
                    println!("cargo:warning=Failed to scan crate {}: {}", crate_name, e);
                }
            }
        }
    }

    // Generate the registry file
    generate_model_registry(&smithy_models)?;

    Ok(())
}

fn get_workspace_root() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR environment variable not set")?;

    let manifest_path = Path::new(&manifest_dir);

    let parent1 = manifest_path
        .parent()
        .ok_or("Cannot get parent directory of CARGO_MANIFEST_DIR")?;

    let workspace_root = parent1
        .parent()
        .ok_or("Cannot get workspace root directory")?;

    Ok(workspace_root.to_path_buf())
}

fn scan_crate_for_smithy_models(
    crate_path: &Path,
    crate_name: &str,
    models: &mut Vec<SmithyModelInfo>,
    seen_models: &mut HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let src_path = crate_path.join("src");
    if !src_path.exists() {
        return Ok(());
    }

    scan_directory(&src_path, crate_name, "", models, seen_models)?;
    Ok(())
}

fn scan_directory(
    dir: &Path,
    crate_name: &str,
    module_path: &str,
    models: &mut Vec<SmithyModelInfo>,
    seen_models: &mut HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = match path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => {
                        println!(
                            "cargo:warning=Skipping directory with invalid name: {}",
                            path.display()
                        );
                        continue;
                    }
                };
                let new_module_path = if module_path.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}::{}", module_path, dir_name)
                };
                scan_directory(&path, crate_name, &new_module_path, models, seen_models)?;
            } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                if let Err(e) = scan_rust_file(&path, crate_name, module_path, models, seen_models)
                {
                    println!(
                        "cargo:warning=Failed to scan Rust file {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }
    Ok(())
}

fn scan_rust_file(
    file_path: &Path,
    crate_name: &str,
    module_path: &str,
    models: &mut Vec<SmithyModelInfo>,
    seen_models: &mut HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(content) = fs::read_to_string(file_path) {
        // Enhanced regex that handles comments, doc comments, and multiple attributes
        // between derive and struct/enum declarations
        let re = Regex::new(r"(?ms)^#\[derive\(([^)]*(?:\([^)]*\))*[^)]*)\)\]\s*(?:(?:#\[[^\]]*\]\s*)|(?://[^\r\n]*\s*)|(?:///[^\r\n]*\s*)|(?:/\*.*?\*/\s*))*(?:pub\s+)?(?:struct|enum)\s+([A-Z][A-Za-z0-9_]*)\s*[<\{\(]")
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

        for captures in re.captures_iter(&content) {
            let derive_content = match captures.get(1) {
                Some(capture) => capture.as_str(),
                None => {
                    println!(
                        "cargo:warning=Missing derive content in regex capture for {}",
                        file_path.display()
                    );
                    continue;
                }
            };
            let item_name = match captures.get(2) {
                Some(capture) => capture.as_str(),
                None => {
                    println!(
                        "cargo:warning=Missing item name in regex capture for {}",
                        file_path.display()
                    );
                    continue;
                }
            };

            // Check if "SmithyModel" is present in the derive macro's content.
            if derive_content.contains("SmithyModel") {
                // Validate that the item name is a valid Rust identifier
                if is_valid_rust_identifier(item_name) {
                    let full_module_path = create_module_path(file_path, crate_name, module_path)?;
                    let cfg_attrs = extract_cfg_attributes(
                        &content,
                        captures.get(0).map(|c| c.start()).unwrap_or(0),
                    );
                    let dedupe_key = format!(
                        "{}::{}|{}",
                        full_module_path,
                        item_name,
                        cfg_attrs.join("&")
                    );

                    if seen_models.insert(dedupe_key) {
                        models.push(SmithyModelInfo {
                            struct_name: item_name.to_string(),
                            module_path: full_module_path,
                            cfg_attrs,
                        });
                    }
                } else {
                    println!(
                        "cargo:warning=Skipping invalid identifier: {} in {}",
                        item_name,
                        file_path.display()
                    );
                }
            }
        }
    }
    Ok(())
}

fn is_valid_rust_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Rust identifiers must start with a letter or underscore
    let first_char = match name.chars().next() {
        Some(ch) => ch,
        None => return false, // This shouldn't happen since we checked is_empty above, but being safe
    };
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Must not be a Rust keyword
    let keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "is", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];

    if keywords.contains(&name) {
        return false;
    }

    // All other characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn create_module_path(
    file_path: &Path,
    crate_name: &str,
    module_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            format!(
                "Cannot extract file name from path: {}",
                file_path.display()
            )
        })?;

    let crate_name_normalized = crate_name.replace('-', "_");

    let result = if file_name == "lib" || file_name == "mod" {
        if module_path.is_empty() {
            crate_name_normalized
        } else {
            format!("{}::{}", crate_name_normalized, module_path)
        }
    } else if module_path.is_empty() {
        format!("{}::{}", crate_name_normalized, file_name)
    } else {
        format!("{}::{}::{}", crate_name_normalized, module_path, file_name)
    };

    Ok(result)
}

fn extract_cfg_attributes(source: &str, item_start: usize) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut idx = item_start;

    while idx > 0 {
        if let Some(prev_newline) = source[..idx].rfind('\n') {
            let line = source[prev_newline + 1..idx].trim();
            if line.is_empty() {
                idx = prev_newline;
                continue;
            }

            if line.starts_with("#[cfg(") {
                attrs.push(line.to_string());
                idx = prev_newline;
                continue;
            }

            if line.starts_with("#[") {
                idx = prev_newline;
                continue;
            }

            break;
        } else {
            break;
        }
    }

    attrs.reverse();
    attrs
}

#[derive(Debug)]
struct SmithyModelInfo {
    struct_name: String,
    module_path: String,
    cfg_attrs: Vec<String>,
}

fn generate_model_registry(models: &[SmithyModelInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR").map_err(|_| "OUT_DIR environment variable not set")?;
    let registry_path = Path::new(&out_dir).join("model_registry.rs");

    let mut content = String::new();
    content.push_str("// Auto-generated model registry\n");
    content.push_str("// DO NOT EDIT - This file is generated by build.rs\n\n");

    if !models.is_empty() {
        content.push_str("use smithy_core::{SmithyModel, SmithyModelGenerator};\n\n");

        // Generate imports
        for model in models {
            for attr in &model.cfg_attrs {
                content.push_str(attr);
                content.push('\n');
            }
            content.push_str(&format!(
                "use {}::{};\n",
                model.module_path, model.struct_name
            ));
        }

        content.push_str("\npub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
        content.push_str("    let mut models = Vec::new();\n\n");

        // Generate model collection calls
        for model in models {
            if model.cfg_attrs.is_empty() {
                content.push_str(&format!(
                    "    models.push({}::generate_smithy_model());\n",
                    model.struct_name
                ));
            } else {
                for attr in &model.cfg_attrs {
                    content.push_str("    ");
                    content.push_str(attr);
                    content.push('\n');
                }
                content.push_str("    {\n");
                content.push_str(&format!(
                    "        models.push({}::generate_smithy_model());\n",
                    model.struct_name
                ));
                content.push_str("    }\n");
            }
        }

        content.push_str("\n    models\n");
        content.push_str("}\n");
    } else {
        // Generate empty function if no models found
        content.push_str("use smithy_core::SmithyModel;\n\n");
        content.push_str("pub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
        content.push_str(
            "    router_env::logger::info!(\"No SmithyModel structs found in workspace\");\n",
        );
        content.push_str("    Vec::new()\n");
        content.push_str("}\n");
    }

    fs::write(&registry_path, content).map_err(|e| {
        format!(
            "Failed to write model registry to {}: {}",
            registry_path.display(),
            e
        )
    })?;

    Ok(())
}
