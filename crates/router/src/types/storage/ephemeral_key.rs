pub use diesel_models::ephemeral_key::{EphemeralKey, EphemeralKeyNew};
#[cfg(feature = "v2")]
pub use diesel_models::ephemeral_key::{EphemeralKeyType, EphemeralKeyTypeNew, ResourceType};

#[cfg(feature = "v2")]
use crate::types::transformers::ForeignFrom;
#[cfg(feature = "v2")]
impl ForeignFrom<EphemeralKeyType> for api_models::ephemeral_key::EphemeralKeyResponse {
    fn foreign_from(from: EphemeralKeyType) -> Self {
        Self {
            customer_id: from.customer_id,
            created_at: from.created_at,
            expires: from.expires,
            secret: from.secret,
            id: from.id,
        }
    }
}
