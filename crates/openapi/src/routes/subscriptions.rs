use serde_json::json;

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
                    "item_price_id": "standard-plan-USD-Monthly",
                    "customer_id": "cust_123456789",
                    "description": "Hello this is description",
                    "merchant_reference_id": "mer_ref_123456789",
                    "shipping": {
                        "address": {
                            "state": "zsaasdas",
                            "city": "Banglore",
                            "country": "US",
                            "line1": "sdsdfsdf",
                            "line2": "hsgdbhd",
                            "line3": "alsksoe",
                            "zip": "571201",
                            "first_name": "joseph",
                            "last_name": "doe"
                        },
                        "phone": {
                            "number": "123456789",
                            "country_code": "+1"
                        }
                    },
                    "billing": {
                        "address": {
                            "line1": "1467",
                            "line2": "Harrison Street",
                            "line3": "Harrison Street",
                            "city": "San Fransico",
                            "state": "California",
                            "zip": "94122",
                            "country": "US",
                            "first_name": "joseph",
                            "last_name": "Doe"
                        },
                        "phone": {
                            "number": "123456789",
                            "country_code": "+1"
                        }
                    },
                    "payment_details": {
                        "payment_type": "setup_mandate",
                        "payment_method": "card",
                        "payment_method_type": "credit",
                        "payment_method_data": {
                            "card": {
                                "card_number": "4000000000000002",
                                "card_exp_month": "03",
                                "card_exp_year": "2030",
                                "card_holder_name": "CLBRW dffdg",
                                "card_cvc": "737"
                            }
                        },
                        "authentication_type": "no_three_ds",
                        "setup_future_usage": "off_session",
                        "capture_method": "automatic",
                        "return_url": "https://google.com",
                        "customer_acceptance": {
                            "acceptance_type": "online",
                            "accepted_at": "1963-05-03T04:07:52.723Z",
                            "online": {
                                "ip_address": "127.0.0.1",
                                "user_agent": "amet irure esse"
                            }
                        }
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
                    "item_price_id": "standard-plan-USD-Monthly",
                    "payment_details": {
                        "authentication_type": "no_three_ds",
                        "setup_future_usage": "off_session",
                        "capture_method": "automatic",
                        "return_url": "https://google.com"
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
                    "payment_details": {
                        "shipping": {
                            "address": {
                                "state": "zsaasdas",
                                "city": "Banglore",
                                "country": "US",
                                "line1": "sdsdfsdf",
                                "line2": "hsgdbhd",
                                "line3": "alsksoe",
                                "zip": "571201",
                                "first_name": "joseph",
                                "last_name": "doe"
                            },
                            "phone": {
                                "number": "123456789",
                                "country_code": "+1"
                            }
                        },
                        "billing": {
                            "address": {
                                "line1": "1467",
                                "line2": "Harrison Street",
                                "line3": "Harrison Street",
                                "city": "San Fransico",
                                "state": "California",
                                "zip": "94122",
                                "country": "US",
                                "first_name": "joseph",
                                "last_name": "Doe"
                            },
                            "phone": {
                                "number": "123456789",
                                "country_code": "+1"
                            }
                        },
                        "payment_method": "card",
                        "payment_method_type": "credit",
                        "payment_method_data": {
                            "card": {
                                "card_number": "4111111111111111",
                                "card_exp_month": "03",
                                "card_exp_year": "2030",
                                "card_holder_name": "CLBRW dffdg",
                                "card_cvc": "737"
                            }
                        },
                        "customer_acceptance": {
                            "acceptance_type": "online",
                            "accepted_at": "1963-05-03T04:07:52.723Z",
                            "online": {
                                "ip_address": "127.0.0.1",
                                "user_agent": "amet irure esse"
                            }
                        }
                    }
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
                    "plan_id":"cbdemo_enterprise-suite",
                    "item_price_id":"cbdemo_enterprise-suite-monthly"
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
        (status = 200, description = "List of available subscription plans", body = Vec<GetPlansResponse>),
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

/// Subscription - Pause Subscription
///
/// Pause the subscription
#[utoipa::path(
    post,
    path = "/subscriptions/{subscription_id}/pause",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    request_body(
        content = PauseSubscriptionRequest,
        examples((
            "Pause subscription" = (
                value = json!({
                    "pause_option": "immediately"
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription paused successfully", body = PauseSubscriptionResponse),
        (status = 400, description = "Invalid pause data"),
        (status = 404, description = "Subscription not found")
    ),
    tag = "Subscriptions",
    operation_id = "Pause Subscription",
    security(("api_key" = []))
)]
pub async fn pause_subscription() {}

/// Subscription - Resume Subscription
///
/// Resume the subscription
#[utoipa::path(
    post,
    path = "/subscriptions/{subscription_id}/resume",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    request_body(
        content = ResumeSubscriptionRequest,
        examples((
            "Resume subscription" = (
                value = json!({
                    "resume_option": "immediately",
                    "unpaid_invoices_handling": "schedule_payment_collection"
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription resumed successfully", body = ResumeSubscriptionResponse),
        (status = 400, description = "Invalid resume data"),
        (status = 404, description = "Subscription not found")
    ),
    tag = "Subscriptions",
    operation_id = "Resume Subscription",
    security(("api_key" = []))
)]
pub async fn resume_subscription() {}

/// Subscription - Cancel Subscription
///
/// Cancel the subscription
#[utoipa::path(
    post,
    path = "/subscriptions/{subscription_id}/cancel",
    params(
        ("subscription_id" = String, Path, description = "The unique identifier for the subscription"),
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")
    ),
    request_body(
        content = CancelSubscriptionRequest,
        examples((
            "Cancel subscription" = (
                value = json!({
                    "cancel_option": "immediately",
                    "unbilled_charges_option": "invoice",
                    "credit_option_for_current_term_charges": "prorate",
                    "refundable_credits_handling": "schedule_refund"
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Subscription cancelled successfully", body = CancelSubscriptionResponse),
        (status = 400, description = "Invalid cancel data"),
        (status = 404, description = "Subscription not found")
    ),
    tag = "Subscriptions",
    operation_id = "Cancel Subscription",
    security(("api_key" = []))
)]
pub async fn cancel_subscription() {}
