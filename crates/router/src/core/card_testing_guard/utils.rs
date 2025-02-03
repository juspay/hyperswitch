use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::logger;

use super::errors;
use crate::{
    core::errors::RouterResult,
    types::domain,
    utils::crypto::{self, SignMessage},
};

pub async fn generate_fingerprint(
    payment_method_data: Option<&api_models::payments::PaymentMethodData>,
    business_profile: &domain::Profile,
) -> RouterResult<Secret<String>> {
    let card_testing_secret_key = &business_profile.card_testing_secret_key;

    match card_testing_secret_key {
        Some(card_testing_secret_key) => {
            let card_number_fingerprint = payment_method_data
                .as_ref()
                .and_then(|pm_data| match pm_data {
                    api_models::payments::PaymentMethodData::Card(card) => {
                        crypto::HmacSha512::sign_message(
                            &crypto::HmacSha512,
                            card_testing_secret_key.get_inner().peek().as_bytes(),
                            card.card_number.clone().get_card_no().as_bytes(),
                        )
                        .attach_printable("error in pm fingerprint creation")
                        .map_or_else(
                            |err| {
                                logger::error!(error=?err);
                                None
                            },
                            Some,
                        )
                    }
                    _ => None,
                })
                .map(hex::encode);

            card_number_fingerprint.map(Secret::new).ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while masking fingerprint")
            })
        }
        None => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("card testing secret key not configured")?,
    }
}
