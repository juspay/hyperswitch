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
/// Asynchronously creates a new customer. This method should be used to create a new customer record in the database.
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
/// Asynchronously retrieves a list of customers from the database.
pub async fn customers_retrieve() {
    // method implementation here
}

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
/// Asynchronously updates customer information.
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
/// Asynchronously deletes customer data from the database.
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
/// Asynchronously retrieves a list of customers.
pub async fn customers_list() {}
