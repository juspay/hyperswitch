$version: "2.0"

namespace alloy.openapi

/// This traits allows the encoding of OpenAPI Extensions
/// as defined in https://swagger.io/docs/specification/openapi-extensions/.
@trait
map openapiExtensions {
    key: String
    value: Document
}

@length(
    min: 1
)
@trait(
    selector: "operation"
)
string summary
