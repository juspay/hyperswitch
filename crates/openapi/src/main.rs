#[cfg(feature = "v1")]
mod openapi;
#[cfg(feature = "v2")]
mod openapi_v2;
mod routes;

#[allow(clippy::print_stdout)] // Using a logger is not necessary here
fn main() {
    #[cfg(all(feature = "v1", feature = "v2"))]
    compile_error!("features v1 and v2 are mutually exclusive, please enable only one of them");

    #[cfg(feature = "v1")]
    let relative_file_path = "api-reference/v1/openapi_spec_v1.json";

    #[cfg(feature = "v2")]
    let relative_file_path = "api-reference/v2/openapi_spec_v2.json";

    #[cfg(any(feature = "v1", feature = "v2"))]
    let mut file_path = router_env::workspace_path();

    #[cfg(any(feature = "v1", feature = "v2"))]
    file_path.push(relative_file_path);

    #[cfg(feature = "v1")]
    let openapi = <openapi::ApiDoc as utoipa::OpenApi>::openapi();
    #[cfg(feature = "v2")]
    let openapi = <openapi_v2::ApiDoc as utoipa::OpenApi>::openapi();

    #[allow(clippy::expect_used)]
    #[cfg(any(feature = "v1", feature = "v2"))]
    std::fs::write(
        &file_path,
        openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI specification as JSON"),
    )
    .expect("Failed to write OpenAPI specification to file");

    #[allow(clippy::expect_used)]
    #[cfg(feature = "v1")]
    {
        // TODO: Do this using utoipa::extensions after we have upgraded to 5.x
        let file_content =
            std::fs::read_to_string(&file_path).expect("Failed to read OpenAPI specification file");

        let mut lines: Vec<&str> = file_content.lines().collect();

        // Insert the new text at line 3 (index 2)
        if lines.len() > 2 {
            let new_line = "  \"x-mcp\": {\n    \"enabled\": true\n  },";
            lines.insert(2, new_line);
        }

        let modified_content = lines.join("\n");
        std::fs::write(&file_path, modified_content)
            .expect("Failed to write modified OpenAPI specification to file");
    }

    #[cfg(any(feature = "v1", feature = "v2"))]
    println!("Successfully saved OpenAPI specification file at '{relative_file_path}'");

    #[cfg(not(any(feature = "v1", feature = "v2")))]
    println!("No feature enabled to generate OpenAPI specification, please enable either 'v1' or 'v2' feature");
}

#[cfg(all(test, any(feature = "v1", feature = "v2")))]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::{HashMap, HashSet};

    fn openapi_spec() -> serde_json::Value {
        #[cfg(feature = "v1")]
        let openapi = <super::openapi::ApiDoc as utoipa::OpenApi>::openapi();
        #[cfg(feature = "v2")]
        let openapi = <super::openapi_v2::ApiDoc as utoipa::OpenApi>::openapi();

        serde_json::to_value(openapi).expect("Failed to serialize OpenAPI specification")
    }

    /// Returns `(endpoint, operation)` pairs, where endpoint is of the form `METHOD /path`.
    fn operations(spec: &serde_json::Value) -> Vec<(String, &serde_json::Value)> {
        spec["paths"]
            .as_object()
            .into_iter()
            .flatten()
            .flat_map(|(path, path_item)| {
                path_item
                    .as_object()
                    .into_iter()
                    .flatten()
                    .map(move |(method, operation)| {
                        (format!("{} {path}", method.to_uppercase()), operation)
                    })
            })
            .collect()
    }

    #[test]
    fn operation_ids_are_unique() {
        let spec = openapi_spec();
        let mut endpoints_by_operation_id: HashMap<&str, Vec<String>> = HashMap::new();

        for (endpoint, operation) in operations(&spec) {
            if let Some(operation_id) = operation["operationId"].as_str() {
                endpoints_by_operation_id
                    .entry(operation_id)
                    .or_default()
                    .push(endpoint);
            }
        }

        let duplicates: Vec<_> = endpoints_by_operation_id
            .into_iter()
            .filter(|(_, endpoints)| endpoints.len() > 1)
            .collect();

        assert!(
            duplicates.is_empty(),
            "Operation IDs used by more than one endpoint: {duplicates:#?}"
        );
    }

    #[test]
    fn path_parameters_match_path_template() {
        let spec = openapi_spec();
        let mut mismatches = Vec::new();

        for (endpoint, operation) in operations(&spec) {
            let in_template: HashSet<&str> = endpoint
                .split('{')
                .skip(1)
                .filter_map(|segment| segment.split_once('}').map(|(name, _)| name))
                .collect();
            let declared: HashSet<&str> = operation["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|parameter| parameter["in"] == "path")
                .filter_map(|parameter| parameter["name"].as_str())
                .collect();

            if in_template != declared {
                mismatches.push(format!(
                    "{endpoint}: not declared {:?}, not in path {:?}",
                    in_template.difference(&declared).collect::<Vec<_>>(),
                    declared.difference(&in_template).collect::<Vec<_>>(),
                ));
            }
        }

        assert!(
            mismatches.is_empty(),
            "Path parameters do not match the path template: {mismatches:#?}"
        );
    }
}
