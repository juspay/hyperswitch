use std::path::PathBuf;

use utoipa::OpenApi;

mod openapi;
mod routes;

fn main() {
    #[cfg(not(feature = "v2"))]
    let relative_file_path = "api-reference/openapi_spec.json";

    #[cfg(feature = "v2")]
    let relative_file_path = "api-reference/v2/openapi_spec.json";

    let mut file_path = router_env::workspace_path();
    file_path.push(relative_file_path);

    #[allow(unused_mut)]
    let openapi = <openapi::ApiDoc as OpenApi>::openapi();

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
