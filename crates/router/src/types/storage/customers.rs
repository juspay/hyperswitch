pub use diesel_models::customers::{Customer, CustomerNew, CustomerUpdateInternal};

#[cfg(feature = "v2")]
pub use crate::types::domain::CustomerGeneralUpdate;
pub use crate::types::domain::CustomerUpdate;
