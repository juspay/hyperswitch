pub use diesel_models::customers::{Customer, CustomerNew, CustomerUpdateInternal};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub use crate::types::domain::CustomerGeneralUpdate;
pub use crate::types::domain::CustomerUpdate;
