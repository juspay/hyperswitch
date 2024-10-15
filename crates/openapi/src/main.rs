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
    let relative_file_path = "api-reference/openapi_spec.json";

    #[cfg(feature = "v2")]
    let relative_file_path = "api-reference-v2/openapi_spec.json";

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
        file_path,
        openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI specification as JSON"),
    )
    .expect("Failed to write OpenAPI specification to file");

    #[cfg(any(feature = "v1", feature = "v2"))]
    println!("Successfully saved OpenAPI specification file at '{relative_file_path}'");

    #[cfg(not(any(feature = "v1", feature = "v2")))]
    println!("No feature enabled to generate OpenAPI specification, please enable either 'v1' or 'v2' feature");
}
