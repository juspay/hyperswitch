/// PaymentMethods - Create
///
/// Creates and stores a payment method against a customer.
/// In case of cards, this API should be used only by PCI compliant merchants.
#[utoipa::path(
    post,
    path = "/payment_methods",
    request_body (
        content = PaymentMethodCreate,
        examples  (( "Save a card" =(
        value =json!( {
            "payment_method": "card",
            "payment_method_type": "credit",
            "payment_method_issuer": "Visa",
            "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "11",
            "card_exp_year": "25",
            "card_holder_name": "John Doe"
            },
            "customer_id": "{{customer_id}}"
        })
        )))
    ),
    responses(
        (status = 200, description = "Payment Method Created", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data")

    ),
    tag = "Payment Methods",
    operation_id = "Create a Payment Method",
    security(("api_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn create_payment_method_api() {}

/// List payment methods for a Merchant
///
/// Lists the applicable payment methods for a particular Merchant ID.
/// Use the client secret and publishable key authorization to list all relevant payment methods of the merchant for the payment corresponding to the client secret.
#[utoipa::path(
    get,
    path = "/account/payment_methods",
    params (
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved", body = PaymentMethodListResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Merchant",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub async fn list_payment_method_api() {}

/// List payment methods for a Customer
///
/// Lists all the applicable payment methods for a particular Customer ID.
#[utoipa::path(
    get,
    path = "/customers/{customer_id}/payment_methods",
    params (
        ("customer_id" = String, Path, description = "The unique identifier for the customer account"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved", body = CustomerPaymentMethodsListResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Customer",
    security(("api_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn list_customer_payment_method_api() {}

/// List customer saved payment methods for a Payment
///
/// Lists all the applicable payment methods for a particular payment tied to the `client_secret`.
#[utoipa::path(
    get,
    path = "/customers/payment_methods",
    params (
        ("client-secret" = String, Path, description = "A secret known only to your client and the authorization server. Used for client side authentication"),
        ("customer_id" = String, Path, description = "The unique identifier for the customer account"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved for customer tied to its respective client-secret passed in the param", body = CustomerPaymentMethodsListResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Customer",
    security(("publishable_key" = []))
)]
pub async fn list_customer_payment_method_api_client() {}

/// Payment Method - Retrieve
///
/// Retrieves a payment method of a customer.
#[utoipa::path(
    get,
    path = "/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method retrieved", body = PaymentMethodResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Retrieve a Payment method",
    security(("api_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn payment_method_retrieve_api() {}

/// Payment Method - Update
///
/// Update an existing payment method of a customer.
/// This API is useful for use cases such as updating the card number for expired cards to prevent discontinuity in recurring payments.
#[utoipa::path(
    post,
    path = "/payment_methods/{method_id}/update",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    request_body = PaymentMethodUpdate,
    responses(
        (status = 200, description = "Payment Method updated", body = PaymentMethodResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Update a Payment method",
    security(("api_key" = []), ("publishable_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn payment_method_update_api() {}

/// Payment Method - Delete
///
/// Deletes a payment method of a customer.
#[utoipa::path(
    delete,
    path = "/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method deleted", body = PaymentMethodDeleteResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Delete a Payment method",
    security(("api_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn payment_method_delete_api() {}

/// Payment Method - Set Default Payment Method for Customer
///
/// Set the Payment Method as Default for the Customer.
#[utoipa::path(
    get,
    path = "/{customer_id}/payment_methods/{payment_method_id}/default",
    params (
        ("customer_id" = String,Path, description ="The unique identifier for the Customer"),
        ("payment_method_id" = String,Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method has been set as default", body =CustomerDefaultPaymentMethodResponse ),
        (status = 400, description = "Payment Method has already been set as default for that customer"),
        (status = 404, description = "Payment Method not found for the customer")
    ),
    tag = "Payment Methods",
    operation_id = "Set the Payment Method as Default",
    security(("ephemeral_key" = []))
)]
pub async fn default_payment_method_set_api() {}

/// Payment Method - Create Intent
///
/// Creates a payment method for customer with billing information and other metadata.
#[utoipa::path(
    post,
    path = "/v2/payment-methods/create-intent",
    request_body(
    content = PaymentMethodIntentCreate,
    // TODO: Add examples
    ),
    responses(
        (status = 200, description = "Payment Method Intent Created", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Payment Methods",
    operation_id = "Create Payment Method Intent",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn create_payment_method_intent_api() {}

/// Payment Method - Confirm Intent
///
/// Update a payment method with customer's payment method related information.
#[utoipa::path(
    post,
    path = "/v2/payment-methods/{id}/confirm-intent",
    request_body(
    content = PaymentMethodIntentConfirm,
    // TODO: Add examples
    ),
    responses(
        (status = 200, description = "Payment Method Intent Confirmed", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Payment Methods",
    operation_id = "Confirm Payment Method Intent",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn confirm_payment_method_intent_api() {}

/// Payment Method - Create
///
/// Creates and stores a payment method against a customer. In case of cards, this API should be used only by PCI compliant merchants.
#[utoipa::path(
    post,
    path = "/v2/payment-methods",
    request_body(
    content = PaymentMethodCreate,
    // TODO: Add examples
    ),
    responses(
        (status = 200, description = "Payment Method Created", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Payment Methods",
    operation_id = "Create Payment Method",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn create_payment_method_api() {}

/// Payment Method - Retrieve
///
/// Retrieves a payment method of a customer.
#[utoipa::path(
    get,
    path = "/v2/payment-methods/{id}",
    params (
        ("id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method Retrieved", body = PaymentMethodResponse),
        (status = 404, description = "Payment Method Not Found"),
    ),
    tag = "Payment Methods",
    operation_id = "Retrieve Payment Method",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn payment_method_retrieve_api() {}

/// Payment Method - Update
///
/// Update an existing payment method of a customer.
#[utoipa::path(
    patch,
    path = "/v2/payment-methods/{id}/update-saved-payment-method",
    request_body(
    content = PaymentMethodUpdate,
    // TODO: Add examples
    ),
    responses(
        (status = 200, description = "Payment Method Update", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Payment Methods",
    operation_id = "Update Payment Method",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn payment_method_update_api() {}

/// Payment Method - Delete
///
/// Deletes a payment method of a customer.
#[utoipa::path(
    delete,
    path = "/v2/payment-methods/{id}",
    params (
        ("id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method Retrieved", body = PaymentMethodDeleteResponse),
        (status = 404, description = "Payment Method Not Found"),
    ),
    tag = "Payment Methods",
    operation_id = "Delete Payment Method",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn payment_method_delete_api() {}

/// Payment Methods - Payment Methods List
///
/// List the payment methods eligible for a payment method.
#[cfg(feature = "v2")]
#[utoipa::path(
    get,
    path = "/v2/payment-methods/{id}/list-enabled-payment-methods",
    params(
        ("id" = String, Path, description = "The global payment method id"),
        (
          "X-Profile-Id" = String, Header,
          description = "Profile ID associated to the payment method intent",
          example = json!({"X-Profile-Id": "pro_abcdefghijklmnop"})
        ),
    ),
    responses(
        (status = 200, description = "Get the payment methods", body = PaymentMethodListResponseForPayments),
        (status = 404, description = "No payment method found with the given id")
    ),
    tag = "Payment Methods",
    operation_id = "List Payment methods for a Payment Method Intent",
    security(("api_key" = [], "ephemeral_key" = []))
)]
pub fn list_payment_methods() {}
