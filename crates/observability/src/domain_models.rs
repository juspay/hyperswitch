//! The types the layers exchange, and the conversions between them.
//!
//! A model here sits between the API types in `api_models` and the rows in `diesel_models`, so
//! neither [`crate::core`] nor [`crate::db`] has to know the other's shape. Validation of what a
//! model may hold lives with the model, so a value that reaches [`crate::db`] is already one the
//! table will accept.
//!
//! Distinct from [`crate::domain`], which holds the traits that say what delivering an alert *is*.

pub mod alerts_info;
pub mod blacklist;
pub mod dictionary;
pub mod lifecycle_events;
pub mod metadata;
pub mod rule_toggles;
pub mod thresholds;
