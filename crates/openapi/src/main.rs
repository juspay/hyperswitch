use utoipa::OpenApi;

#[cfg(feature = "v2")]
use crate::openapi::ApiDocV2;

mod openapi;
mod routes;

fn main() {
    let file_path = "api-reference/openapi_spec.json";

    #[allow(unused_mut)]
    let mut openapi = <openapi::ApiDoc as OpenApi>::openapi();

    // Get the paths and schemas from v2 api reference and add it to the same openapi file
    #[cfg(feature = "v2")]
    {
        let api_v2_paths = ApiDocV2::openapi().paths;
        let api_v2_components = ApiDocV2::openapi().components;

        if let Some((components, api_v2_components)) =
            openapi.components.as_mut().zip(api_v2_components)
        {
            components.schemas.extend(api_v2_components.schemas);
        }

        openapi.paths.paths.extend(api_v2_paths.paths);
    }

    #[allow(clippy::expect_used)]
    std::fs::write(
        file_path,
        openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI specification as JSON"),
    )
    .expect("Failed to write OpenAPI specification to file");
    println!("Successfully saved OpenAPI specification file at '{file_path}'");
}
