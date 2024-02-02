mod openapi;
mod routes;

/// Writes the OpenAPI specification to a file in JSON format and prints a success message.
fn main() {
    let file_path = "openapi/openapi_spec.json";
    #[allow(clippy::expect_used)]
    std::fs::write(
        file_path,
        <openapi::ApiDoc as utoipa::OpenApi>::openapi()
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI specification as JSON"),
    )
    .expect("Failed to write OpenAPI specification to file");
    println!("Successfully saved OpenAPI specification file at '{file_path}'");
}
