use serde::{Deserialize, Serialize};

pub use crate::types::storage::customers::{
    Customer as CustomerResponse, CustomerNew as CustomerUpdateRequest,
    CustomerNew as CreateCustomerRequest,
};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerId {
    pub customer_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CustomerDeleteResponse {
    pub customer_id: String,
    pub deleted: bool,
}
