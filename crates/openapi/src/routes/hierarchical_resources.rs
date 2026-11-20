/// Hierarchical Resource - Generate
///
/// Generates an organization-scoped managed resource using the authenticated merchant and profile.
/// For Google Pay DIRECT, Hyperswitch generates a P-256 key pair and returns only the public setup
/// configuration. The public key is base64-encoded uncompressed EC point data, not PEM or SPKI.
/// The base64 PKCS#8 private key is encrypted through the organization key store and is never returned.
///
/// Register the public key in the Google Pay & Wallet Console and use it with protocolVersion ECv2
/// in the checkout's DIRECT tokenization specification. This API does not register the key with Google,
/// collect a payment token, decrypt a Google token, create an MCA, or authorize a payment. Google's
/// DIRECT integration, PCI and production-approval requirements still apply. The requested environment
/// must match the Hyperswitch deployment: TEST for development/sandbox, PRODUCTION for production.
#[utoipa::path(
    post,
    path = "/hierarchical_resources",
    params(("X-Profile-Id" = String, Header, description = "Profile owned by the authenticated merchant")),
    request_body(
        content = GenerateHierarchicalResourceRequest,
        examples(
            ("Google Pay DIRECT" = (value = json!({
                "type": "google_pay",
                "google_pay": {
                    "mode": "DIRECT",
                    "environment": "TEST",
                    "google_merchant_id": "12345678901234567890"
                }
            }))),
            ("Apple Pay CSR" = (value = json!({"type": "apple_pay_certificate"})))
        )
    ),
    responses(
        (status = 200, description = "Resource ID and public setup data; no private key", body = GenerateHierarchicalResourceResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Dashboard authentication required"),
        (status = 422, description = "Invalid Google configuration or deployment environment")
    ),
    tag = "Hierarchical Resources",
    operation_id = "Generate Hierarchical Resource",
    security(("jwt_key" = []))
)]
pub async fn generate_hierarchical_resource() {}

/// Hierarchical Resource - List
///
/// Lists linkable resources and their safe setup metadata. Google Pay results are restricted to the
/// originating merchant within the organization and never contain private-key material. Use scope_type
/// and scope_id together to mark the effective resource as is_linked. For Google Pay, a profile's own
/// configuration takes precedence; the merchant default is used only when profile configuration is absent.
#[utoipa::path(
    post,
    path = "/hierarchical_resources/list",
    params(("X-Profile-Id" = String, Header, description = "Profile owned by the authenticated merchant")),
    request_body(content = ListHierarchicalResourcesRequest),
    responses(
        (status = 200, description = "Safe resource metadata and effective linkage", body = ListHierarchicalResourcesResponse),
        (status = 401, description = "Dashboard authentication required"),
        (status = 404, description = "Target scope not found for the authenticated merchant")
    ),
    tag = "Hierarchical Resources",
    operation_id = "List Hierarchical Resources",
    security(("jwt_key" = []))
)]
pub async fn list_hierarchical_resources() {}

/// Hierarchical Resource - Link
///
/// Links a Google Pay DIRECT resource to an owned profile or to the authenticated merchant account.
/// The resource must belong to the same organization and originating merchant. Google Pay resources
/// cannot be linked to an MCA and do not accept certificate uploads. Linking records the public
/// configuration and re-encrypts the private-key cache under the merchant key through existing account
/// update and cache-invalidation paths. It does not return the private key or enable a Google Pay
/// session/token-decryption flow. Native v2 management routes are not added; compatible v2 models can
/// read the configuration linked through this v1 API. Apply the v2-compatible migrations for both versions.
#[utoipa::path(
    post,
    path = "/hierarchical_resources/{id}/link",
    params(
        ("id" = String, Path, description = "Managed resource ID"),
        ("X-Profile-Id" = String, Header, description = "Profile owned by the authenticated merchant")
    ),
    request_body(content = LinkHierarchicalResourceRequest),
    responses(
        (status = 200, description = "Linked resource ID", body = LinkHierarchicalResourceResponse),
        (status = 401, description = "Dashboard authentication required"),
        (status = 403, description = "Resource belongs to another organization"),
        (status = 404, description = "Resource or target not found for the authenticated merchant"),
        (status = 400, description = "Unsupported resource linkage")
    ),
    tag = "Hierarchical Resources",
    operation_id = "Link Hierarchical Resource",
    security(("jwt_key" = []))
)]
pub async fn link_hierarchical_resource() {}
