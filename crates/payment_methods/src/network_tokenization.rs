use std::collections::HashSet;

use common_enums::{enums, ApplicationError};
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, ConfigExt, Encode},
    id_type,
};
use error_stack::ResultExt;
use hyperswitch_domain_models as domain;
use hyperswitch_interfaces::secrets_interface::{self, secret_handler, secret_state};
use josekit::jwe;
use masking::{Mask, PeekInterface, Secret};
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::deserialize_hashset, core::errors::NetworkTokenizationError, headers,
    state::PaymentMethodsState, types::payment_methods as pm_transformers,
};

const NETWORK_TOKEN_SERVICE: &str = "network_tokenization";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetworkTokenizationSupportedCardNetworks {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub card_networks: HashSet<enums::CardNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenizationService {
    pub generate_token_url: url::Url,
    pub fetch_token_url: url::Url,
    pub token_service_api_key: Secret<String>,
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
    pub key_id: String,
    pub delete_token_url: url::Url,
    pub check_token_status_url: url::Url,
}

impl NetworkTokenizationService {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.token_service_api_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "token_service_api_key must not be empty".into(),
            ))
        })?;

        when(self.public_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "public_key must not be empty".into(),
            ))
        })?;

        when(self.key_id.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "key_id must not be empty".into(),
            ))
        })?;

        when(self.private_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "private_key must not be empty".into(),
            ))
        })
    }
}

#[async_trait::async_trait]
impl secret_handler::SecretsHandler for NetworkTokenizationService {
    async fn convert_to_raw_secret(
        value: secret_state::SecretStateContainer<Self, secret_state::SecuredSecret>,
        secret_management_client: &dyn secrets_interface::SecretManagementInterface,
    ) -> CustomResult<
        secret_state::SecretStateContainer<Self, secret_state::RawSecret>,
        secrets_interface::SecretsManagementError,
    > {
        let network_tokenization = value.get_inner();
        let token_service_api_key = secret_management_client
            .get_secret(network_tokenization.token_service_api_key.clone())
            .await?;
        let public_key = secret_management_client
            .get_secret(network_tokenization.public_key.clone())
            .await?;
        let private_key = secret_management_client
            .get_secret(network_tokenization.private_key.clone())
            .await?;

        Ok(value.transition_state(|network_tokenization| Self {
            public_key,
            private_key,
            token_service_api_key,
            ..network_tokenization
        }))
    }
}

pub type NetworkTokenizationResponse = (
    pm_transformers::CardNetworkTokenResponsePayload,
    Option<String>,
);

async fn mk_tokenization_req(
    state: &PaymentMethodsState,
    payload_bytes: &[u8],
    customer_id: id_type::CustomerId,
    tokenization_service: &NetworkTokenizationService,
) -> CustomResult<NetworkTokenizationResponse, NetworkTokenizationError> {
    let enc_key = tokenization_service.public_key.peek().clone();
    let key_id = tokenization_service.key_id.clone();

    let jwt = common_utils::encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        common_utils::encryption::EncryptionAlgorithm::A128GCM,
        Some(key_id.as_str()),
    )
    .await
    .change_context(NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable("Error on jwe encrypt")?;

    let order_data = pm_transformers::OrderData {
        consent_id: uuid::Uuid::new_v4().to_string(),
        customer_id,
    };

    let api_payload = pm_transformers::ApiPayload {
        service: NETWORK_TOKEN_SERVICE.to_string(),
        card_data: Secret::new(jwt),
        order_data,
        key_id,
        should_send_token: true,
    };

    let mut request = common_utils::request::Request::new(
        common_utils::request::Method::Post,
        tokenization_service.generate_token_url.as_str(),
    );
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service
            .token_service_api_key
            .peek()
            .clone()
            .into_masked(),
    );
    request.add_default_headers();

    request.set_body(common_utils::request::RequestContent::Json(Box::new(
        api_payload,
    )));

    logger::info!("Request to generate token: {:?}", request);

    let response = state
        .connector_api_client
        .call_connector_api(request, "generate_token".to_string())
        .await
        .change_context(NetworkTokenizationError::ApiError);

    let res = response
        .change_context(NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_transformers::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(NetworkTokenizationError::ResponseDeserializationFailed)?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let network_response: pm_transformers::CardNetworkTokenResponse = res
        .response
        .parse_struct("Card Network Tokenization Response")
        .change_context(NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::debug!("Network Token Response: {:?}", network_response);

    let dec_key = tokenization_service.private_key.peek().clone();

    let card_network_token_response = common_utils::encryption::decrypt_jwe(
        network_response.payload.peek(),
        common_utils::encryption::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .change_context(NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable(
        "Failed to decrypt the tokenization response from the tokenization service",
    )?;

    let cn_response: pm_transformers::CardNetworkTokenResponsePayload =
        serde_json::from_str(&card_network_token_response)
            .change_context(NetworkTokenizationError::ResponseDeserializationFailed)?;
    Ok((cn_response.clone(), Some(cn_response.card_reference)))
}

pub async fn make_card_network_tokenization_request(
    state: &PaymentMethodsState,
    card: &domain::payment_method_data::CardDetail,
    optional_cvc: Option<Secret<String>>,
    customer_id: &id_type::CustomerId,
) -> CustomResult<NetworkTokenizationResponse, NetworkTokenizationError> {
    let card_data = pm_transformers::CardData {
        card_number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        card_security_code: optional_cvc,
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(NetworkTokenizationError::RequestEncodingFailed)?;

    let payload_bytes = payload.as_bytes();
    if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
        mk_tokenization_req(
            state,
            payload_bytes,
            customer_id.clone(),
            network_tokenization_service.get_inner(),
        )
        .await
        .inspect_err(|e| logger::error!(error=?e, "Error while making tokenization request"))
    } else {
        Err(NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
            .inspect_err(|_| {
                logger::error!("Network Tokenization Service not configured");
            })
            .attach_printable("Network Tokenization Service not configured")
    }
}
