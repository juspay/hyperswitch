#[cfg(feature = "v1")]
mod openapi;
#[cfg(feature = "v2")]
mod openapi_v2;
mod routes;

#[allow(clippy::print_stdout)] // Using a logger is not necessary here
fn main() {
    #[cfg(feature = "v1")]
    let relative_file_path = "api-reference/openapi_spec.json";

    #[cfg(feature = "v2")]
    let relative_file_path = "api-reference/openapi_spec_v2.json";

    let mut file_path = router_env::workspace_path();
    file_path.push(relative_file_path);

    #[cfg(feature = "v1")]
    let openapi = <openapi::ApiDoc as utoipa::OpenApi>::openapi();
    #[cfg(feature = "v2")]
    let openapi = <openapi_v2::ApiDoc as utoipa::OpenApi>::openapi();

    #[allow(clippy::expect_used)]
    std::fs::write(
        file_path,
        openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI specification as JSON"),
    )
    .expect("Failed to write OpenAPI specification to file");
    println!("Successfully saved OpenAPI specification file at '{relative_file_path}'");
}
