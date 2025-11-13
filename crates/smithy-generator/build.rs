// crates/smithy-generator/build.rs

use std::{collections::HashSet, fs, path::Path};

use regex::Regex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_build()
}

fn run_build() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = get_workspace_root()?;

    // Detect enabled features for this build
    let enabled_features = detect_enabled_features();

    // Create feature resolver to map smithy-generator features to dependency features
    let feature_resolver = FeatureResolver::new(&enabled_features);

    let mut smithy_models = Vec::new();

    // Scan all crates in the workspace for SmithyModel derives
    let crates_dir = workspace_root.join("crates");
    if let Ok(entries) = fs::read_dir(&crates_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let crate_path = entry.path();
                let crate_name = match crate_path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => {
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

                // Check if this crate should be scanned based on enabled features
                if feature_resolver.should_scan_crate(&crate_name) {
                    let _ = scan_crate_for_smithy_models(
                        &crate_path,
                        &crate_name,
                        &mut smithy_models,
                        &feature_resolver,
                    );
                }
            }
        }
    }

    // Generate the registry file
    generate_model_registry(&smithy_models)?;

    Ok(())
}

/// Detects which features are enabled for the current build
fn detect_enabled_features() -> HashSet<String> {
    let mut features = HashSet::new();

    // Check all CARGO_FEATURE_* environment variables
    for (key, value) in std::env::vars() {
        if key.starts_with("CARGO_FEATURE_") && value == "1" {
            if let Some(feature_suffix) = key.strip_prefix("CARGO_FEATURE_") {
                let feature_name = feature_suffix.to_lowercase().replace('_', "-");
                features.insert(feature_name);
            }
        }
    }

    // Always include default features if none are explicitly enabled
    // Note: We should be more careful about default feature assumptions
    if features.is_empty() {
        features.insert("v1".to_string());
    }

    features
}

/// Resolves feature dependencies between smithy-generator and its dependencies
struct FeatureResolver {
    enabled_features: HashSet<String>,
    crate_feature_cache: std::collections::HashMap<String, HashSet<String>>,
}

impl FeatureResolver {
    fn new(enabled_features: &HashSet<String>) -> Self {
        let mut resolver = Self {
            enabled_features: enabled_features.clone(),
            crate_feature_cache: std::collections::HashMap::new(),
        };

        // Pre-compute feature mappings for known crates
        resolver.initialize_feature_mappings();
        resolver
    }

    fn initialize_feature_mappings(&mut self) {
        // Initialize feature mappings for each known crate
        for crate_name in &["api_models", "common_utils", "common_types", "common_enums"] {
            let features = self.compute_crate_features(crate_name);
            self.crate_feature_cache
                .insert(crate_name.to_string(), features);
        }
    }

    /// Determines if a crate should be scanned based on enabled features
    fn should_scan_crate(&self, crate_name: &str) -> bool {
        match crate_name {
            "api_models" => {
                // api_models is a core crate that should always be scanned
                // The feature gates within the crate will determine what's available
                true
            }
            "common_utils" => {
                // common_utils is a core utility crate, always scan it
                true
            }
            "common_types" => {
                // common_types should be scanned if any version features are enabled
                self.enabled_features.contains("v1") || self.enabled_features.contains("v2")
            }
            "common_enums" => {
                // common_enums is used by most other crates, always scan it
                true
            }
            "router_derive" => {
                // Proc macro crate, not relevant for SmithyModel scanning
                false
            }
            "external_services" | "storage_impl" | "router" => {
                // These are application crates, may contain models but less likely
                // Include them but with lower priority
                true
            }
            _ => {
                // For unknown crates, include them by default to be safe
                true
            }
        }
    }

    /// Determines which features are enabled for a dependency crate
    fn get_enabled_crate_features(&self, crate_name: &str) -> HashSet<String> {
        // Use cached result if available
        if let Some(cached_features) = self.crate_feature_cache.get(crate_name) {
            return cached_features.clone();
        }

        // Compute features for unknown crates
        self.compute_crate_features(crate_name)
    }

    fn compute_crate_features(&self, crate_name: &str) -> HashSet<String> {
        let mut crate_features = HashSet::new();

        match crate_name {
            "api_models" => {
                // api_models has v1 and v2 features that directly map to smithy-generator features
                if self.enabled_features.contains("v1") {
                    crate_features.insert("v1".to_string());
                }
                if self.enabled_features.contains("v2") {
                    crate_features.insert("v2".to_string());
                }

                // api_models also has some derived features
                for feature in &self.enabled_features {
                    match feature.as_str() {
                        "frm" | "payouts" | "disputes" | "routing" => {
                            crate_features.insert(feature.clone());
                        }
                        _ => {}
                    }
                }
            }
            "common_utils" => {
                // common_utils features typically map directly
                if self.enabled_features.contains("v1") {
                    crate_features.insert("v1".to_string());
                }
                if self.enabled_features.contains("v2") {
                    crate_features.insert("v2".to_string());
                }
            }
            "common_types" => {
                // common_types has payment-related features
                if self.enabled_features.contains("v2") {
                    crate_features.insert("v2".to_string());
                }
                // Some features are version-independent
                for feature in &self.enabled_features {
                    match feature.as_str() {
                        "frm" | "payouts" | "disputes" => {
                            crate_features.insert(feature.clone());
                        }
                        _ => {}
                    }
                }
            }
            "common_enums" => {
                // common_enums typically doesn't have version-specific features
                // but may have functional features
                for feature in &self.enabled_features {
                    match feature.as_str() {
                        "frm" | "payouts" | "disputes" | "routing" => {
                            crate_features.insert(feature.clone());
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                // For unknown crates, be conservative and only pass through
                // features that are commonly used
                for feature in &self.enabled_features {
                    match feature.as_str() {
                        "v1" | "v2" => {
                            crate_features.insert(feature.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        crate_features
    }
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
    feature_resolver: &FeatureResolver,
) -> Result<(), Box<dyn std::error::Error>> {
    let src_path = crate_path.join("src");
    if !src_path.exists() {
        return Ok(());
    }

    let _enabled_features = feature_resolver.get_enabled_crate_features(crate_name);

    scan_directory(&src_path, crate_name, "", models, feature_resolver)?;
    Ok(())
}

fn scan_directory(
    dir: &Path,
    crate_name: &str,
    module_path: &str,
    models: &mut Vec<SmithyModelInfo>,
    feature_resolver: &FeatureResolver,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = match path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => {
                        continue;
                    }
                };
                let new_module_path = if module_path.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}::{}", module_path, dir_name)
                };
                scan_directory(
                    &path,
                    crate_name,
                    &new_module_path,
                    models,
                    feature_resolver,
                )?;
            } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                let _ = scan_rust_file(&path, crate_name, module_path, models, feature_resolver);
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
    feature_resolver: &FeatureResolver,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(content) = fs::read_to_string(file_path) {
        // More precise regex that captures cfg attributes, derive, and struct/enum declarations
        // This captures cfg attributes that may be separated from derive by comments/other attributes
        let re = Regex::new(r"(?ms)((?:#\[cfg\([^\]]*\)\]\s*)*)((?:///[^\r\n]*\s*|#\[[^\]]*\]\s*)*)(#\[derive\([^)]*\bSmithyModel\b[^)]*\)\])\s*(?:(?:#\[[^\]]*\]\s*)|(?://[^\r\n]*\s*)|(?:///[^\r\n]*\s*)|(?:/\*.*?\*/\s*))*(?:pub\s+)?(?:struct|enum)\s+([A-Z][A-Za-z0-9_]*)\s*[<\{\(]")
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

        for captures in re.captures_iter(&content) {
            let cfg_attrs = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let derive_attr = captures.get(3).map(|m| m.as_str()).unwrap_or("");
            let item_name = match captures.get(4) {
                Some(capture) => capture.as_str(),
                None => {
                    continue;
                }
            };

            // Verify this is actually a SmithyModel derive by checking the derive attribute more precisely
            if !derive_attr.contains("SmithyModel") {
                continue;
            }

            // For items with the same name but different feature gates (like FeatureMetadata),
            // we need to ensure we only include the version that actually has SmithyModel
            // under the current feature configuration
            let enabled_features = feature_resolver.get_enabled_crate_features(crate_name);

            // Special handling for items that have multiple definitions with different derives
            if item_name == "FeatureMetadata" {
                // FeatureMetadata only has SmithyModel in v1 version
                if !enabled_features.contains("v1") {
                    continue;
                }
            }

            // Check if this item is available under current feature gates
            if is_item_available_for_features(cfg_attrs, crate_name, feature_resolver) {
                // Validate that the item name is a valid Rust identifier
                if is_valid_rust_identifier(item_name) {
                    let full_module_path = create_module_path(file_path, crate_name, module_path)?;

                    models.push(SmithyModelInfo {
                        struct_name: item_name.to_string(),
                        module_path: full_module_path,
                        cfg_attrs: cfg_attrs.to_string(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Checks if an item is available under the current feature configuration
fn is_item_available_for_features(
    cfg_attrs: &str,
    crate_name: &str,
    feature_resolver: &FeatureResolver,
) -> bool {
    if cfg_attrs.trim().is_empty() {
        // No feature gates, item is always available
        return true;
    }

    let enabled_features = feature_resolver.get_enabled_crate_features(crate_name);

    // Parse each cfg attribute line
    let cfg_lines: Vec<&str> = cfg_attrs
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && line.starts_with("#[cfg("))
        .collect();

    // If no cfg attributes found, item is available
    if cfg_lines.is_empty() {
        return true;
    }

    // All cfg attributes must be satisfied (AND logic between different cfg attributes)
    for cfg_line in cfg_lines {
        if !evaluate_cfg_condition(cfg_line, &enabled_features) {
            return false;
        }
    }

    true
}

/// Evaluates a single cfg condition
fn evaluate_cfg_condition(cfg_line: &str, enabled_features: &HashSet<String>) -> bool {
    // Simple feature check: #[cfg(feature = "v1")]
    if let Some(captures) = Regex::new(r#"#\[cfg\(feature\s*=\s*"([^"]+)"\)\]"#)
        .ok()
        .and_then(|re| re.captures(cfg_line))
    {
        if let Some(feature_match) = captures.get(1) {
            let feature_name = feature_match.as_str();
            return enabled_features.contains(feature_name);
        }
    }

    // any() function: #[cfg(any(feature = "v1", feature = "v2"))]
    if let Some(captures) = Regex::new(r#"#\[cfg\(any\(([^)]+)\)\)\]"#)
        .ok()
        .and_then(|re| re.captures(cfg_line))
    {
        if let Some(any_content) = captures.get(1) {
            return evaluate_any_condition(any_content.as_str(), enabled_features);
        }
    }

    // all() function: #[cfg(all(feature = "v1", not(feature = "v2")))]
    if let Some(captures) = Regex::new(r#"#\[cfg\(all\(([^)]+)\)\)\]"#)
        .ok()
        .and_then(|re| re.captures(cfg_line))
    {
        if let Some(all_content) = captures.get(1) {
            return evaluate_all_condition(all_content.as_str(), enabled_features);
        }
    }

    // not() function: #[cfg(not(feature = "v1"))]
    if let Some(captures) = Regex::new(r#"#\[cfg\(not\(([^)]+)\)\)\]"#)
        .ok()
        .and_then(|re| re.captures(cfg_line))
    {
        if let Some(not_content) = captures.get(1) {
            return !evaluate_simple_feature_condition(not_content.as_str(), enabled_features);
        }
    }

    // If we can't parse the cfg condition, assume it's available to avoid false negatives
    true
}

/// Evaluates any() condition - returns true if any condition is met
fn evaluate_any_condition(condition: &str, enabled_features: &HashSet<String>) -> bool {
    let parts: Vec<&str> = condition.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if evaluate_simple_feature_condition(part, enabled_features) {
            return true;
        }
    }

    false
}

/// Evaluates all() condition - returns true if all conditions are met
fn evaluate_all_condition(condition: &str, enabled_features: &HashSet<String>) -> bool {
    let parts: Vec<&str> = condition.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if part.starts_with("not(") && part.ends_with(')') {
            let inner = &part[4..part.len() - 1];
            if evaluate_simple_feature_condition(inner, enabled_features) {
                return false; // not() condition failed
            }
        } else if !evaluate_simple_feature_condition(part, enabled_features) {
            return false;
        }
    }

    true
}

/// Evaluates a simple feature condition like 'feature = "v1"'
fn evaluate_simple_feature_condition(condition: &str, enabled_features: &HashSet<String>) -> bool {
    if let Some(captures) = Regex::new(r#"feature\s*=\s*"([^"]+)""#)
        .ok()
        .and_then(|re| re.captures(condition))
    {
        if let Some(feature_match) = captures.get(1) {
            let feature_name = feature_match.as_str();
            return enabled_features.contains(feature_name);
        }
    }

    // If we can't parse it, assume it's false for safety
    false
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

#[derive(Debug)]
struct SmithyModelInfo {
    struct_name: String,
    module_path: String,
    cfg_attrs: String,
}

fn generate_model_registry(models: &[SmithyModelInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR").map_err(|_| "OUT_DIR environment variable not set")?;
    let registry_path = Path::new(&out_dir).join("model_registry.rs");

    let mut content = String::new();
    content.push_str("// Auto-generated model registry\n");
    content.push_str("// DO NOT EDIT - This file is generated by build.rs\n\n");

    if !models.is_empty() {
        content.push_str("use smithy_core::{SmithyModel, SmithyModelGenerator};\n\n");

        // Generate feature-gated imports
        for model in models {
            if !model.cfg_attrs.trim().is_empty() {
                // Add cfg attributes for conditional compilation
                content.push_str(&format!("{}\n", model.cfg_attrs.trim()));
            }
            content.push_str(&format!(
                "use {}::{};\n",
                model.module_path, model.struct_name
            ));
        }

        content.push_str("\npub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
        content.push_str("    vec![\n");

        // Generate feature-gated model collection calls
        for model in models.iter() {
            if !model.cfg_attrs.trim().is_empty() {
                // Add cfg attributes for conditional compilation
                content.push_str(&format!("        {}\n", model.cfg_attrs.trim()));
            }
            content.push_str(&format!(
                "        {}::generate_smithy_model(),\n",
                model.struct_name
            ));
        }

        content.push_str("    ]\n");
        content.push_str("}\n");
    } else {
        // Generate empty function if no models found
        content.push_str("use smithy_core::SmithyModel;\n\n");
        content.push_str("pub fn discover_smithy_models() -> Vec<SmithyModel> {\n");
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
