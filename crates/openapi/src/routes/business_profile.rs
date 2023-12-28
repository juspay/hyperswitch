/// Business Profile - Create
///
/// Creates a new *business profile* for a merchant
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "The unique identifier for the merchant account")
    ),
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Create a business profile with minimal fields" = (
                    value = json!({})
                )
            ),
            (
                "Create a business profile with profile name" = (
                    value = json!({
                        "profile_name": "shoe_business"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Business Account Created", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Create A Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_create() {}

/// Business Profile - List
///
/// Lists all the *business profiles* under a merchant
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "Merchant Identifier"),
    ),
    responses(
        (status = 200, description = "Business profiles Retrieved", body = Vec<BusinessProfileResponse>)
    ),
    tag = "Business Profile",
    operation_id = "List Business Profiles",
    security(("api_key" = []))
)]
pub async fn business_profiles_list() {}

/// Business Profile - Update
///
/// Update the *business profile*
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Update business profile with profile name fields" = (
                    value = json!({
                        "profile_name" : "shoe_business"
                    })
                )
            )
    )),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Update a Business Profile",
    security(("api_key" = []))
)]
pub async fn business_profiles_update() {}

/// Business Profile - Delete
///
/// Delete the *business profile*
#[utoipa::path(
    delete,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    responses(
        (status = 200, description = "Business profiles Deleted", body = bool),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Delete the Business Profile",
    security(("api_key" = []))
)]
pub async fn business_profiles_delete() {}

/// Business Profile - Retrieve
///
/// Retrieve existing *business profile*
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Retrieve a Business Profile",
    security(("api_key" = []))
)]
pub async fn business_profiles_retrieve() {}
