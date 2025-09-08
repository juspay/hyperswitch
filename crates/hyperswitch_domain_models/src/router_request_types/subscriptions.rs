use common_utils::id_type::CustomerId;
#[derive(Debug, Clone)]
pub struct CreateCustomerRequest {
    pub customer_id: CustomerId,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub locale: Option<String>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Clone)]
pub struct BillingAddress {
    pub first_name: String,
    pub last_name: String,
    pub line1: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}
