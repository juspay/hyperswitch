use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods,
    },
    pii::Secret,
    routes::AppState,
    types::{
        api::{self, enums as api_enums},
        storage::{self, enums as storage_enums},
    },
};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateId {
    pub mandate_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateRevokedResponse {
    pub mandate_id: String,
    pub status: api_enums::MandateStatus,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateResponse {
    pub mandate_id: String,
    pub status: api_enums::MandateStatus,
    pub payment_method_id: String,
    pub payment_method: String,
    pub card: Option<MandateCardDetails>,
    pub customer_acceptance: Option<api::payments::CustomerAcceptance>,
}

impl MandateResponse {
    pub async fn from_db_mandate(
        state: &AppState,
        mandate: storage::Mandate,
    ) -> RouterResult<Self> {
        let db = &*state.store;
        let payment_method = db
            .find_payment_method(&mandate.payment_method_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
            })?;
        let card = if payment_method.payment_method == storage_enums::PaymentMethodType::Card {
            let get_card_resp = payment_methods::cards::get_card_from_legacy_locker(
                state,
                &payment_method.merchant_id,
                &payment_method.payment_method_id,
            )
            .await?;
            let card_detail =
                payment_methods::transformers::get_card_detail(&payment_method, get_card_resp.card)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
            Some(MandateCardDetails::from(card_detail))
        } else {
            None
        };
        Ok(MandateResponse {
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
            status: mandate.mandate_status.into(),
            payment_method: payment_method.payment_method.to_string(),
            payment_method_id: mandate.payment_method_id,
        })
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateCardDetails {
    pub last4_digits: Option<String>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_token: Option<Secret<String>>,
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub card_fingerprint: Option<Secret<String>>,
}

impl From<api::payment_methods::CardDetailFromLocker> for MandateCardDetails {
    fn from(card_details_from_locker: api::payment_methods::CardDetailFromLocker) -> Self {
        Self {
            last4_digits: card_details_from_locker.last4_digits,
            card_exp_month: card_details_from_locker.expiry_month.clone(),
            card_exp_year: card_details_from_locker.expiry_year.clone(),
            card_holder_name: card_details_from_locker.card_holder_name,
            card_token: card_details_from_locker.card_token,
            scheme: card_details_from_locker.scheme,
            issuer_country: card_details_from_locker.issuer_country,
            card_fingerprint: card_details_from_locker.card_fingerprint,
        }
    }
}
