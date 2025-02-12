#[cfg(feature = "v2")]
pub use diesel_models::ephemeral_key::{ClientSecretType, ClientSecretTypeNew};
pub use diesel_models::ephemeral_key::{EphemeralKey, EphemeralKeyNew};

#[cfg(feature = "v2")]
use crate::db::errors;
#[cfg(feature = "v2")]
use crate::types::transformers::ForeignTryFrom;
#[cfg(feature = "v2")]
impl ForeignTryFrom<ClientSecretType> for api_models::ephemeral_key::ClientSecretResponse {
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(from: ClientSecretType) -> Result<Self, errors::ApiErrorResponse> {
        match from.resource_id {
            common_utils::types::authentication::ResourceId::Payment(global_payment_id) => {
                Err(errors::ApiErrorResponse::InternalServerError)
            }
            common_utils::types::authentication::ResourceId::PaymentMethodSession(
                global_payment_id,
            ) => Err(errors::ApiErrorResponse::InternalServerError),
            common_utils::types::authentication::ResourceId::Customer(global_customer_id) => {
                Ok(Self {
                    resource_id: api_models::ephemeral_key::ResourceId::Customer(
                        global_customer_id.clone(),
                    ),
                    created_at: from.created_at,
                    expires: from.expires,
                    secret: from.secret,
                    id: from.id,
                })
            }
        }
    }
}
