/// Header key used to specify the connector name in UCS requests.
pub const UCS_HEADER_CONNECTOR: &str = "x-connector";

/// Header key used to indicate the authentication type being used.
pub const UCS_HEADER_AUTH_TYPE: &str = "x-auth";

/// Header key for sending the API key used for authentication.
pub const UCS_HEADER_API_KEY: &str = "x-api-key";

/// Header key for sending an additional secret key used in some auth types.
pub const UCS_HEADER_KEY1: &str = "x-key1";

/// Header key for sending the API secret in signature-based authentication.
pub const UCS_HEADER_API_SECRET: &str = "x-api-secret";

/// Header value indicating that signature-key-based authentication is used.
pub const UCS_AUTH_SIGNATURE_KEY: &str = "signature-key";

/// Header value indicating that body-key-based authentication is used.
pub const UCS_AUTH_BODY_KEY: &str = "body-key";

/// Header value indicating that header-key-based authentication is used.
pub const UCS_AUTH_HEADER_KEY: &str = "header-key";