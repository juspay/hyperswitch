pub use diesel_models::customers::{Customer, CustomerNew, CustomerUpdateInternal};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub(crate) use crate::types::domain::CustomerGeneralUpdate;
pub(crate) use crate::types::domain::CustomerUpdate;
