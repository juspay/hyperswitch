//! Raw BIN metadata scoped to one request; no PAN or derived card data.
pub type CardInfoCache = super::request_cache::RequestCache<diesel_models::CardInfo>;
