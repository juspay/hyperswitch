/// Customers - Create
///
/// Creates a customer object and stores the customer details to be reused for future payments.
/// Incase the customer already exists in the system, this API will respond with the customer details.
#[utoipa::path(
    post,
    path = "/customers",
    request_body  (
        content = CustomerRequest,
        examples  (( "Update name and email of a customer" =(
        value =json!( {
            "email": "guest@example.com",
            "name": "John Doe"
        })
        )))
    ),
    responses(
        (status = 200, description = "Customer Created", body = CustomerResponse),
        (status = 400, description = "Invalid data")

    ),
    tag = "Customers",
    operation_id = "Create a Customer",
    security(("api_key" = []))
)]
pub async fn customers_create() {}

/// Customers - Retrieve
///
/// Retrieves a customer's details.
#[utoipa::path(
    get,
    path = "/customers/{customer_id}",
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer Retrieved", body = CustomerResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Retrieve a Customer",
    security(("api_key" = []), ("ephemeral_key" = []))
)]
pub async fn customers_retrieve() {}

/// Customers - Update
///
/// Updates the customer's details in a customer object.
#[utoipa::path(
    post,
    path = "/customers/{customer_id}",
    request_body (
        content = CustomerRequest,
        examples  (( "Update name and email of a customer" =(
        value =json!( {
            "email": "guest@example.com",
            "name": "John Doe"
        })
        )))
    ),
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer was Updated", body = CustomerResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Update a Customer",
    security(("api_key" = []))
)]
pub async fn customers_update() {}

/// Customers - Delete
///
/// Delete a customer record.
#[utoipa::path(
    delete,
    path = "/customers/{customer_id}",
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer was Deleted", body = CustomerDeleteResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Delete a Customer",
    security(("api_key" = []))
)]
pub async fn customers_delete() {}

/// Customers - List
///
/// Lists all the customers for a particular merchant id.
#[utoipa::path(
    post,
    path = "/customers/list",
    responses(
        (status = 200, description = "Customers retrieved", body = Vec<CustomerResponse>),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Customers List",
    operation_id = "List all Customers for a Merchant",
    security(("api_key" = []))
)]
pub async fn customers_list() {}

/// Customers - Mandates List
///
/// Lists all the mandates for a particular customer id.
#[utoipa::path(
    post,
    path = "/customers/{customer_id}/mandates",
    responses(
        (status = 200, description = "List of retrieved mandates for a customer", body = Vec<MandateResponse>),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Customers Mandates List",
    operation_id = "List all Mandates for a Customer",
    security(("api_key" = []))
)]
pub async fn customers_mandates_list() {}

/// Customers - Set Default Payment Method
///
/// Set the Payment Method as Default for the Customer.
#[utoipa::path(
    get,
    path = "/{customer_id}/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "Set the Payment Method as Default for the Customer"),
    ),
    responses(
        (status = 200, description = "Payment Method has been set as default", body =CustomerDefaultPaymentMethodResponse ),
        (status = 400, description = "Payment Method has already been set as default for that customer"),
        (status = 404, description = "Payment Method not found for the customer")
    ),
    tag = "Customer Set Default Payment Method",
    operation_id = "Set the Payment Method as Default",
    security(("ephemeral_key" = []))
)]
pub async fn default_payment_method_set_api() {}
