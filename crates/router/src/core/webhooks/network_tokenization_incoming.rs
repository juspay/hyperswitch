use std::str::FromStr;

use ::payment_methods::controller::PaymentMethodsController;
use api_models::webhooks::WebhookResponseTracker;
use async_trait::async_trait;
use common_utils::{
    crypto::Encryptable,
    ext_traits::{AsyncExt, ByteSliceExt, ValueExt},
    id_type,
};
use error_stack::{report, ResultExt};
use http::HeaderValue;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::cards,
    },
    logger,
    routes::{app::SessionStateInfo, SessionState},
    types::{
        api, domain, payment_methods as pm_types,
        storage::{self, enums},
    },
    utils::{self as helper_utils, ext_traits::OptionExt},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NetworkTokenWebhookResponse {
    PanMetadataUpdate(pm_types::PanMetadataUpdateBody),
    NetworkTokenMetadataUpdate(pm_types::NetworkTokenMetaDataUpdateBody),
}

impl NetworkTokenWebhookResponse {
    fn get_network_token_requestor_ref_id(&self) -> String {
        match self {
            Self::PanMetadataUpdate(data) => data.card.card_reference.clone(),
            Self::NetworkTokenMetadataUpdate(data) => data.token.card_reference.clone(),
        }
    }

    pub fn get_response_data(self) -> Box<dyn NetworkTokenWebhookResponseExt> {
        match self {
            Self::PanMetadataUpdate(data) => Box::new(data),
            Self::NetworkTokenMetadataUpdate(data) => Box::new(data),
        }
    }

    pub async fn fetch_merchant_id_payment_method_id_customer_id_from_callback_mapper(
        &self,
        state: &SessionState,
    ) -> RouterResult<(id_type::MerchantId, String, id_type::CustomerId)> {
        let network_token_requestor_ref_id = &self.get_network_token_requestor_ref_id();

        let db = &*state.store;
        let callback_mapper_data = db
            .find_call_back_mapper_by_id(network_token_requestor_ref_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch callback mapper data")?;

        Ok(callback_mapper_data
            .data
            .get_network_token_webhook_details())
    }
}

pub fn get_network_token_resource_object(
    request_details: &api::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::NetworkTokenizationError> {
    let response: NetworkTokenWebhookResponse = request_details
        .body
        .parse_struct("NetworkTokenWebhookResponse")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    Ok(Box::new(response))
}

#[async_trait]
pub trait NetworkTokenWebhookResponseExt {
    fn decrypt_payment_method_data(
        &self,
        payment_method: &domain::PaymentMethod,
    ) -> CustomResult<api::payment_methods::CardDetailFromLocker, errors::ApiErrorResponse>;

    async fn update_payment_method(
        &self,
        state: &SessionState,
        payment_method: &domain::PaymentMethod,
        platform: &domain::Platform,
    ) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse>;
}

#[async_trait]
impl NetworkTokenWebhookResponseExt for pm_types::PanMetadataUpdateBody {
    fn decrypt_payment_method_data(
        &self,
        payment_method: &domain::PaymentMethod,
    ) -> CustomResult<api::payment_methods::CardDetailFromLocker, errors::ApiErrorResponse> {
        let decrypted_data = payment_method
            .payment_method_data
            .clone()
            .map(|payment_method_data| payment_method_data.into_inner().expose())
            .and_then(|val| {
                val.parse_value::<api::payment_methods::PaymentMethodsData>("PaymentMethodsData")
                    .map_err(|err| logger::error!(?err, "Failed to parse PaymentMethodsData"))
                    .ok()
            })
            .and_then(|pmd| match pmd {
                api::payment_methods::PaymentMethodsData::Card(token) => {
                    Some(api::payment_methods::CardDetailFromLocker::from(token))
                }
                _ => None,
            })
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to obtain decrypted token object from db")?;
        Ok(decrypted_data)
    }

    async fn update_payment_method(
        &self,
        state: &SessionState,
        payment_method: &domain::PaymentMethod,
        platform: &domain::Platform,
    ) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
        let decrypted_data = self.decrypt_payment_method_data(payment_method)?;
        handle_metadata_update(
            state,
            &self.card,
            payment_method
                .locker_id
                .clone()
                .get_required_value("locker_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Locker id is not found for the payment method")?,
            payment_method,
            platform,
            decrypted_data,
            true,
        )
        .await
    }
}

#[async_trait]
impl NetworkTokenWebhookResponseExt for pm_types::NetworkTokenMetaDataUpdateBody {
    fn decrypt_payment_method_data(
        &self,
        payment_method: &domain::PaymentMethod,
    ) -> CustomResult<api::payment_methods::CardDetailFromLocker, errors::ApiErrorResponse> {
        let decrypted_data = payment_method
            .network_token_payment_method_data
            .clone()
            .map(|x| x.into_inner().expose())
            .and_then(|val| {
                val.parse_value::<api::payment_methods::PaymentMethodsData>("PaymentMethodsData")
                    .map_err(|err| logger::error!(?err, "Failed to parse PaymentMethodsData"))
                    .ok()
            })
            .and_then(|pmd| match pmd {
                api::payment_methods::PaymentMethodsData::Card(token) => {
                    Some(api::payment_methods::CardDetailFromLocker::from(token))
                }
                _ => None,
            })
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to obtain decrypted token object from db")?;
        Ok(decrypted_data)
    }

    async fn update_payment_method(
        &self,
        state: &SessionState,
        payment_method: &domain::PaymentMethod,
        platform: &domain::Platform,
    ) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
        let decrypted_data = self.decrypt_payment_method_data(payment_method)?;
        handle_metadata_update(
            state,
            &self.token,
            payment_method
                .network_token_locker_id
                .clone()
                .get_required_value("locker_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Locker id is not found for the payment method")?,
            payment_method,
            platform,
            decrypted_data,
            true,
        )
        .await
    }
}

pub struct Authorization {
    header: Option<HeaderValue>,
}

impl Authorization {
    pub fn new(header: Option<&HeaderValue>) -> Self {
        Self {
            header: header.cloned(),
        }
    }

    pub async fn verify_webhook_source(
        self,
        nt_service: &settings::NetworkTokenizationService,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let secret = nt_service.webhook_source_verification_key.clone();

        let source_verified = match self.header {
            Some(authorization_header) => match authorization_header.to_str() {
                Ok(header_value) => Ok(header_value == secret.expose()),
                Err(err) => {
                    logger::error!(?err, "Failed to parse authorization header");
                    Err(errors::ApiErrorResponse::WebhookAuthenticationFailed)
                }
            },
            None => Ok(false),
        }?;
        logger::info!(source_verified=?source_verified);

        helper_utils::when(!source_verified, || {
            Err(report!(
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            ))
        })?;

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_metadata_update(
    state: &SessionState,
    metadata: &pm_types::NetworkTokenRequestorData,
    locker_id: String,
    payment_method: &domain::PaymentMethod,
    platform: &domain::Platform,
    decrypted_data: api::payment_methods::CardDetailFromLocker,
    is_pan_update: bool,
) -> RouterResult<WebhookResponseTracker> {
    let merchant_id = platform.get_processor().get_account().get_id();
    let customer_id = &payment_method.customer_id;
    let payment_method_id = payment_method.get_id().clone();
    let status = payment_method.status;

    match metadata.is_update_required(decrypted_data) {
        false => {
            logger::info!(
                "No update required for payment method {} for locker_id {}",
                payment_method.get_id(),
                locker_id
            );
            Ok(WebhookResponseTracker::PaymentMethod {
                payment_method_id,
                status,
            })
        }
        true => {
            let mut card = cards::get_card_from_locker(state, customer_id, merchant_id, &locker_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch token information from the locker")?;

            card.card_exp_year = metadata.expiry_year.clone();
            card.card_exp_month = metadata.expiry_month.clone();

            let card_network = card
                .card_brand
                .clone()
                .map(|card_brand| enums::CardNetwork::from_str(&card_brand))
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "card network",
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid Card Network stored in vault")?;

            let card_data = api::payment_methods::CardDetail::from((card, card_network));

            let payment_method_request: api::payment_methods::PaymentMethodCreate =
                PaymentMethodCreateWrapper::from((&card_data, payment_method)).get_inner();

            let pm_cards = cards::PmCards { state, platform };

            pm_cards
                .delete_card_from_locker(customer_id, merchant_id, &locker_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to delete network token")?;

            let (res, _) = pm_cards
                .add_card_to_locker(payment_method_request, &card_data, customer_id, None)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to add network token")?;

            let pm_details = res.card.as_ref().map(|card| {
                api::payment_methods::PaymentMethodsData::Card(
                    api::payment_methods::CardDetailsPaymentMethod::from((card.clone(), None)),
                )
            });
            let key_manager_state = state.into();

            let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> = pm_details
                .async_map(|pm_card| {
                    cards::create_encrypted_data(
                        &key_manager_state,
                        platform.get_processor().get_key_store(),
                        pm_card,
                    )
                })
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt payment method data")?;

            let pm_update = if is_pan_update {
                storage::PaymentMethodUpdate::AdditionalDataUpdate {
                    locker_id: Some(res.payment_method_id),
                    payment_method_data: pm_data_encrypted.map(Into::into),
                    status: None,
                    payment_method: None,
                    payment_method_type: None,
                    payment_method_issuer: None,
                    network_token_requestor_reference_id: None,
                    network_token_locker_id: None,
                    network_token_payment_method_data: None,
                    last_modified_by: None,
                }
            } else {
                storage::PaymentMethodUpdate::AdditionalDataUpdate {
                    locker_id: None,
                    payment_method_data: None,
                    status: None,
                    payment_method: None,
                    payment_method_type: None,
                    payment_method_issuer: None,
                    network_token_requestor_reference_id: None,
                    network_token_locker_id: Some(res.payment_method_id),
                    network_token_payment_method_data: pm_data_encrypted.map(Into::into),
                    last_modified_by: None,
                }
            };
            let db = &*state.store;

            db.update_payment_method(
                platform.get_processor().get_key_store(),
                payment_method.clone(),
                pm_update,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update the payment method")?;

            Ok(WebhookResponseTracker::PaymentMethod {
                payment_method_id,
                status,
            })
        }
    }
}

pub struct PaymentMethodCreateWrapper(pub api::payment_methods::PaymentMethodCreate);

impl From<(&api::payment_methods::CardDetail, &domain::PaymentMethod)>
    for PaymentMethodCreateWrapper
{
    fn from(
        (data, payment_method): (&api::payment_methods::CardDetail, &domain::PaymentMethod),
    ) -> Self {
        Self(api::payment_methods::PaymentMethodCreate {
            customer_id: Some(payment_method.customer_id.clone()),
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            payment_method_issuer: payment_method.payment_method_issuer.clone(),
            payment_method_issuer_code: payment_method.payment_method_issuer_code,
            metadata: payment_method.metadata.clone(),
            payment_method_data: None,
            connector_mandate_details: None,
            client_secret: None,
            billing: None,
            card: Some(data.clone()),
            card_network: data
                .card_network
                .clone()
                .map(|card_network| card_network.to_string()),
            bank_transfer: None,
            wallet: None,
            network_transaction_id: payment_method.network_transaction_id.clone(),
        })
    }
}

impl PaymentMethodCreateWrapper {
    fn get_inner(self) -> api::payment_methods::PaymentMethodCreate {
        self.0
    }
}

pub async fn fetch_merchant_account_for_network_token_webhooks(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
) -> RouterResult<domain::Platform> {
    let db = &*state.store;

    let key_store = state
        .store()
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store().get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)
        .attach_printable("Failed to fetch merchant key store for the merchant id")?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant account for the merchant id")?;

    let platform = domain::Platform::new(
        merchant_account.clone(),
        key_store.clone(),
        merchant_account,
        key_store,
    );

    Ok(platform)
}

pub async fn fetch_payment_method_for_network_token_webhooks(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_method_id: &str,
) -> RouterResult<domain::PaymentMethod> {
    let db = &*state.store;

    let payment_method = db
        .find_payment_method(
            key_store,
            payment_method_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        .attach_printable("Failed to fetch the payment method")?;

    Ok(payment_method)
}
