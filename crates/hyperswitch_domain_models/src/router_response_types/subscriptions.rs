#[derive(Debug, Clone)]
pub struct CreateCustomerResponse {
    pub customer_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub locale: Option<String>,
    pub preferred_currency_code: Option<String>,
    pub billing_address: Option<BillingAddressResponse>,
}

#[derive(Debug, Clone)]
pub struct BillingAddressResponse {
    pub first_name: String,
    pub last_name: String,
    pub line1: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub zip: String,
}


