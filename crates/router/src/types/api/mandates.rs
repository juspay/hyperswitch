use api_models::mandates;
pub use api_models::mandates::{MandateId, MandateResponse, MandateRevokedResponse};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods,
    },
    newtype,
    routes::AppState,
    types::{
        api,
        storage::{self, enums as storage_enums},
        transformers::ForeignInto,
    },
};

newtype!(
    pub MandateCardDetails = mandates::MandateCardDetails,
    derives = (Default, Debug, Deserialize, Serialize)
);

#[async_trait::async_trait]
pub(crate) trait MandateResponseExt: Sized {
    async fn from_db_mandate(
        state: &AppState,
        mandate: storage::Mandate,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self>;
}

#[async_trait::async_trait]
impl MandateResponseExt for MandateResponse {
    async fn from_db_mandate(
        state: &AppState,
        mandate: storage::Mandate,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        let db = &*state.store;
        let payment_method = db
            .find_payment_method(&mandate.payment_method_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
            })?;

        let card = if payment_method.payment_method == storage_enums::PaymentMethod::Card {
            let card = payment_methods::cards::get_card_from_locker(
                state,
                &payment_method.customer_id,
                &payment_method.merchant_id,
                &payment_method.payment_method_id,
                merchant_account.locker_id.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting card from card vault")?;
            let card_detail = payment_methods::transformers::get_card_detail(&payment_method, card)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting card details")?;
            Some(MandateCardDetails::from(card_detail).into_inner())
        } else {
            None
        };

        Ok(Self {
            mandate_id: mandate.mandate_id,
            customer_acceptance: Some(api::payments::CustomerAcceptance {
                acceptance_type: if mandate.customer_ip_address.is_some() {
                    api::payments::AcceptanceType::Online
                } else {
                    api::payments::AcceptanceType::Offline
                },
                accepted_at: mandate.customer_accepted_at,
                online: Some(api::payments::OnlineMandate {
                    ip_address: mandate.customer_ip_address.unwrap_or_default(),
                    user_agent: mandate.customer_user_agent.unwrap_or_default(),
                }),
            }),
            card,
            status: mandate.mandate_status.foreign_into(),
            payment_method: payment_method.payment_method.to_string(),
            payment_method_id: mandate.payment_method_id,
        })
    }
}

impl From<api::payment_methods::CardDetailFromLocker> for MandateCardDetails {
    fn from(card_details_from_locker: api::payment_methods::CardDetailFromLocker) -> Self {
        mandates::MandateCardDetails {
            last4_digits: card_details_from_locker.last4_digits,
            card_exp_month: card_details_from_locker.expiry_month.clone(),
            card_exp_year: card_details_from_locker.expiry_year.clone(),
            card_holder_name: card_details_from_locker.card_holder_name,
            card_token: card_details_from_locker.card_token,
            scheme: card_details_from_locker.scheme,
            issuer_country: card_details_from_locker.issuer_country,
            card_fingerprint: card_details_from_locker.card_fingerprint,
        }
        .into()
    }
}
