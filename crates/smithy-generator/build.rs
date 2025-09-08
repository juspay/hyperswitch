// crates/smithy-generator/build.rs

use regex::Regex;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../");
    
    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| Path::new(&dir).parent().unwrap().parent().unwrap().to_path_buf())
        .unwrap();
    
    println!("cargo:warning=Scanning workspace root: {}", workspace_root.display());
    
    let mut smithy_models = Vec::new();
    
    // Scan all crates in the workspace for SmithyModel derives
    let crates_dir = workspace_root.join("crates");
    if let Ok(entries) = fs::read_dir(&crates_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let crate_path = entry.path();
                let crate_name = crate_path.file_name().unwrap().to_string_lossy();
                
                // Skip the smithy crate itself to avoid self-dependency
                if crate_name == "smithy" || crate_name == "smithy-core" || crate_name == "smithy-generator" {
                    continue;
                }
                
                println!("cargo:warning=Scanning crate: {}", crate_name);
                scan_crate_for_smithy_models(&crate_path, &crate_name, &mut smithy_models);
            }
        }
    }
    
    println!("cargo:warning=Found {} SmithyModel structs", smithy_models.len());
    for model in &smithy_models {
        println!("cargo:warning=  - {}: {}", model.crate_name, model.struct_name);
    }
    
    // Generate the registry file
    generate_model_registry(&smithy_models);
}

fn scan_crate_for_smithy_models(crate_path: &Path, crate_name: &str, models: &mut Vec<SmithyModelInfo>) {
    let src_path = crate_path.join("src");
    if !src_path.exists() {
        return;
    }
    
    scan_directory(&src_path, crate_name, "", models);
}

fn scan_directory(dir: &Path, crate_name: &str, module_path: &str, models: &mut Vec<SmithyModelInfo>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap().to_string_lossy();
                let new_module_path = if module_path.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}::{}", module_path, dir_name)
                };
                scan_directory(&path, crate_name, &new_module_path, models);
            } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                scan_rust_file(&path, crate_name, module_path, models);
            }
        }
    }
}

fn scan_rust_file(file_path: &Path, crate_name: &str, module_path: &str, models: &mut Vec<SmithyModelInfo>) {
    if let Ok(content) = fs::read_to_string(file_path) {
        // Enhanced regex that handles comments, doc comments, and multiple attributes
        // between derive and struct/enum declarations
        let re = Regex::new(r"(?ms)^#\[derive\(([^)]*(?:\([^)]*\))*[^)]*)\)\]\s*(?:(?:#\[[^\]]*\]\s*)|(?://[^\r\n]*\s*)|(?:///[^\r\n]*\s*)|(?:/\*.*?\*/\s*))*(?:pub\s+)?(?:struct|enum)\s+([A-Z][A-Za-z0-9_]*)\s*[<\{\(]").unwrap();

        for captures in re.captures_iter(&content) {
            let derive_content = match captures.get(1) {
                Some(capture) => capture.as_str(),
                None => {
                    println!("cargo:warning=Missing derive content in regex capture for {}", file_path.display());
                    continue;
                }
            };
            let item_name = match captures.get(2) {
                Some(capture) => capture.as_str(),
                None => {
                    println!("cargo:warning=Missing item name in regex capture for {}", file_path.display());
                    continue;
                }
            };

            // Check if "SmithyModel" is present in the derive macro's content.
            if derive_content.contains("SmithyModel") {
                // Validate that the item name is a valid Rust identifier
                if is_valid_rust_identifier(item_name) {
                    let full_module_path = create_module_path(file_path, crate_name, module_path);

                    println!("cargo:warning=Found SmithyModel: {} in {}", item_name, file_path.display());
                    
                    models.push(SmithyModelInfo {
                        crate_name: crate_name.to_string(),
                        struct_name: item_name.to_string(),
                        module_path: full_module_path,
                    });
                } else {
                    println!("cargo:warning=Skipping invalid identifier: {} in {}", item_name, file_path.display());
                }
            }
        }
    }
}

fn is_valid_rust_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    // Rust identifiers must start with a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }
    
    // Must not be a Rust keyword
    let keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
        "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
        "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        "async", "await", "dyn", "is", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try"
    ];
    
    if keywords.contains(&name) {
        return false;
    }
    
    // All other characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn create_module_path(file_path: &Path, crate_name: &str, module_path: &str) -> String {
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    
    let crate_name_normalized = crate_name.replace('-', "_");
    
    if file_name == "lib" || file_name == "mod" {
        if module_path.is_empty() {
            crate_name_normalized
        } else {
            format!("{}::{}", crate_name_normalized, module_path)
        }
    } else if module_path.is_empty() {
        format!("{}::{}", crate_name_normalized, file_name)
    } else {
        format!("{}::{}::{}", crate_name_normalized, module_path, file_name)
    }
}

#[derive(Debug)]
struct SmithyModelInfo {
    crate_name: String,
    struct_name: String,
    module_path: String,
}

fn generate_model_registry(models: &[SmithyModelInfo]) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let registry_path = Path::new(&out_dir).join("model_registry.rs");
    
    let mut content = String::new();
    content.push_str("// Auto-generated model registry\n");
    content.push_str("// DO NOT EDIT - This file is generated by build.rs\n\n");
    
    if !models.is_empty() {
        content.push_str("use smithy_core::{SmithyModel, SmithyModelGenerator};\n\n");
        
        // Generate imports
        for model in models {
            content.push_str(&format!(
                "use {}::{};\n",
                model.module_path,
                model.struct_name
            ));
        }
        
        content.push_str("\npub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
        content.push_str("    let mut models = Vec::new();\n\n");
        
        // Generate model collection calls
        for model in models {
            content.push_str(&format!(
                "    models.push({}::generate_smithy_model());\n",
                model.struct_name
            ));
        }
        
        content.push_str("\n    models\n");
        content.push_str("}\n");
    } else {
        // Generate empty function if no models found
        content.push_str("use smithy_core::SmithyModel;\n\n");
        content.push_str("pub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
        content.push_str("    println!(\"No SmithyModel structs found in workspace\");\n");
        content.push_str("    Vec::new()\n");
        content.push_str("}\n");
    }
    
    fs::write(&registry_path, content).expect("Failed to write model registry");
    println!("cargo:warning=Generated model registry at: {}", registry_path.display());
}
