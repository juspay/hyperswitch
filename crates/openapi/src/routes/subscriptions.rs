use serde_json::json;
use utoipa;

/// Subscription - Create and Confirm
///
/// Creates and confirms a subscription in a single request.
#[utoipa::path(
    post,
    path = "/subscriptions",
    request_body(
        content = CreateAndConfirmSubscriptionRequest,
        examples((
            "Create and confirm subscription" = (
                value = json!({
                    "customer_id": "cust_123456789",
                    "plan_id": "plan_monthly_basic",
                    "payment_method_id": "pm_1234567890",
                    "billing_details": {
                        "name": "John Doe",
                        "email": "john@example.com"
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription created and confirmed successfully", body = SubscriptionResponse),
        (status = 400, description = "Invalid subscription data"),
        (status = 404, description = "Customer or plan not found")
    ),
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    tag = "Subscriptions",
    operation_id = "Create and Confirm Subscription",
    security(("api_key" = []))
)]
pub async fn create_and_confirm_subscription() {}

/// Subscription - Create
///
/// Creates a subscription that requires separate confirmation.
#[utoipa::path(
    post,
    path = "/subscriptions/create",
    request_body(
        content = CreateSubscriptionRequest,
        examples((
            "Create subscription" = (
                value = json!({
                    "customer_id": "cust_123456789",
                    "plan_id": "plan_monthly_basic",
                    "trial_days": 7,
                    "metadata": {
                        "source": "web_app"
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription created successfully", body = SubscriptionResponse),
        (status = 400, description = "Invalid subscription data"),
        (status = 404, description = "Customer or plan not found")
    ),
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    tag = "Subscriptions",
    operation_id = "Create Subscription",
    security(("api_key" = []))
)]
pub async fn create_subscription() {}

/// Subscription - Confirm
///
/// Confirms a previously created subscription.
#[utoipa::path(
    post,
    path = "/subscriptions/{subscription_id}/confirm",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    request_body(
        content = ConfirmSubscriptionRequest,
        examples((
            "Confirm subscription" = (
                value = json!({
                    "payment_method_id": "pm_1234567890",
                    "client_secret": "seti_1234567890_secret_abcdef",
                    "return_url": "https://example.com/return"
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription confirmed successfully", body = SubscriptionResponse),
        (status = 400, description = "Invalid confirmation data"),
        (status = 404, description = "Subscription not found"),
        (status = 409, description = "Subscription already confirmed")
    ),
    tag = "Subscriptions",
    operation_id = "Confirm Subscription",
    security(("api_key" = []), ("client_secret" = []))
)]
pub async fn confirm_subscription() {}

/// Subscription - Retrieve
///
/// Retrieves subscription details by ID.
#[utoipa::path(
    get,
    path = "/subscriptions/{subscription_id}",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    responses(
        (status = 200, description = "Subscription retrieved successfully", body = SubscriptionResponse),
        (status = 404, description = "Subscription not found")
    ),
    tag = "Subscriptions",
    operation_id = "Retrieve Subscription",
    security(("api_key" = []))
)]
pub async fn get_subscription() {}

/// Subscription - Update
///
/// Updates an existing subscription.
#[utoipa::path(
    put,
    path = "/subscriptions/{subscription_id}/update",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    request_body(
        content = UpdateSubscriptionRequest,
        examples((
            "Update subscription" = (
                value = json!({
                    "plan_id": "plan_yearly_premium",
                    "proration_behavior": "create_prorations",
                    "metadata": {
                        "updated_reason": "plan_upgrade"
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription updated successfully", body = SubscriptionResponse),
        (status = 400, description = "Invalid update data"),
        (status = 404, description = "Subscription not found")
    ),
    tag = "Subscriptions",
    operation_id = "Update Subscription",
    security(("api_key" = []))
)]
pub async fn update_subscription() {}

/// Subscription - Get Plans
///
/// Retrieves available subscription plans.
#[utoipa::path(
    get,
    path = "/subscriptions/plans",
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication"),
        ("limit" = Option<u32>, Query, description = "Number of plans to retrieve"),
        ("offset" = Option<u32>, Query, description = "Number of plans to skip"),
        ("product_id" = Option<String>, Query, description = "Filter by product ID")
    ),
    responses(
        (status = 200, description = "Plans retrieved successfully", body = Vec<GetPlansResponse>),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "Subscriptions",
    operation_id = "Get Subscription Plans",
    security(("api_key" = []), ("client_secret" = []))
)]
pub async fn get_subscription_plans() {}

/// Subscription - Get Estimate
///
/// Gets pricing estimate for a subscription.
#[utoipa::path(
    get,
    path = "/subscriptions/estimate",
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication"),
        ("plan_id" = String, Query, description = "Plan ID for estimation"),
        ("customer_id" = Option<String>, Query, description = "Customer ID for personalized pricing"),
        ("coupon_id" = Option<String>, Query, description = "Coupon ID to apply discount"),
        ("trial_days" = Option<u32>, Query, description = "Number of trial days")
    ),
    responses(
        (status = 200, description = "Estimate retrieved successfully", body = EstimateSubscriptionResponse),
        (status = 400, description = "Invalid estimation parameters"),
        (status = 404, description = "Plan not found")
    ),
    tag = "Subscriptions",
    operation_id = "Get Subscription Estimate",
    security(("api_key" = []))
)]
pub async fn get_estimate() {}
