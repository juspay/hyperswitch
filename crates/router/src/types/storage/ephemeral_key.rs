pub use diesel_models::ephemeral_key::{EphemeralKey, EphemeralKeyNew};
#[cfg(feature = "v2")]
pub use diesel_models::ephemeral_key::{EphemeralKeyType, EphemeralKeyTypeNew};

#[cfg(feature = "v2")]
use crate::db::errors;
#[cfg(feature = "v2")]
use crate::types::transformers::ForeignTryFrom;
#[cfg(feature = "v2")]
impl ForeignTryFrom<EphemeralKeyType> for api_models::ephemeral_key::EphemeralKeyResponse {
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(from: EphemeralKeyType) -> Result<Self, errors::ApiErrorResponse> {
        let result = from.resource_id.iter().find_map(|item| {
            if let diesel_models::ResourceId::Customer(customer_id) = item {
                Some(Self {
                    resource_id: api_models::ephemeral_key::ResourceId::Customer(
                        customer_id.clone(),
                    ),
                    created_at: from.created_at,
                    expires: from.expires,
                    secret: from.secret.clone(),
                    id: from.id.clone(),
                })
            } else {
                None
            }
        });
        match result {
            Some(val) => Ok(val),
            None => Err(errors::ApiErrorResponse::InternalServerError),
        }
    }
}
