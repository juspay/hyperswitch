use crate::surcharge::errors::{CustomResult, RedisError, RouterResult};
use crate::{
    client::PaymentMethodsState,
    core::{
        domain::{
            api::SurchargeDetailsResponse,
            diesel as storage,
            types::{AuthenticationData, SurchargeDetails},
            Profile,
        },
        errors,
    },
    surcharge,
};
use common_utils::ext_traits::OptionExt;
use common_utils::ext_traits::Encode;
use error_stack::ResultExt;
use common_utils::{consts, transformers::ForeignTryFrom};
use router_env::{instrument, logger, tracing};
use std::collections::HashMap;
use std::num::TryFromIntError;
pub mod conditional_configs;

#[cfg(feature = "v2")]
impl ForeignTryFrom<(&SurchargeDetails, &PaymentAttempt)> for SurchargeDetailsResponse {
    type Error = TryFromIntError;
    fn foreign_try_from(
        (surcharge_details, payment_attempt): (&SurchargeDetails, &PaymentAttempt),
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug, strum::Display)]
pub enum SurchargeKey {
    Token(String),
    PaymentMethodData(
        common_enums::PaymentMethod,
        common_enums::PaymentMethodType,
        Option<common_enums::CardNetwork>,
    ),
}

#[derive(Clone, Debug)]
pub struct SurchargeMetadata {
    surcharge_results: HashMap<SurchargeKey, SurchargeDetails>,
    pub payment_attempt_id: String,
}

impl SurchargeMetadata {
    pub fn new(payment_attempt_id: String) -> Self {
        Self {
            surcharge_results: HashMap::new(),
            payment_attempt_id,
        }
    }
    pub fn is_empty_result(&self) -> bool {
        self.surcharge_results.is_empty()
    }
    pub fn get_surcharge_results_size(&self) -> usize {
        self.surcharge_results.len()
    }
    pub fn insert_surcharge_details(
        &mut self,
        surcharge_key: SurchargeKey,
        surcharge_details: SurchargeDetails,
    ) {
        self.surcharge_results
            .insert(surcharge_key, surcharge_details);
    }
    pub fn get_surcharge_details(&self, surcharge_key: SurchargeKey) -> Option<&SurchargeDetails> {
        self.surcharge_results.get(&surcharge_key)
    }
    pub fn get_surcharge_metadata_redis_key(payment_attempt_id: &str) -> String {
        format!("surcharge_metadata_{}", payment_attempt_id)
    }
    pub fn get_individual_surcharge_key_value_pairs(&self) -> Vec<(String, SurchargeDetails)> {
        self.surcharge_results
            .iter()
            .map(|(surcharge_key, surcharge_details)| {
                let key = Self::get_surcharge_details_redis_hashset_key(surcharge_key);
                (key, surcharge_details.to_owned())
            })
            .collect()
    }
    pub fn get_surcharge_details_redis_hashset_key(surcharge_key: &SurchargeKey) -> String {
        match surcharge_key {
            SurchargeKey::Token(token) => {
                format!("token_{}", token)
            }
            SurchargeKey::PaymentMethodData(payment_method, payment_method_type, card_network) => {
                if let Some(card_network) = card_network {
                    format!(
                        "{}_{}_{}",
                        payment_method, payment_method_type, card_network
                    )
                } else {
                    format!("{}_{}", payment_method, payment_method_type)
                }
            }
        }
    }
    #[instrument(skip_all)]
    pub async fn persist_individual_surcharge_details_in_redis(
        &self,
        state: &PaymentMethodsState,
        business_profile: &Profile,
    ) -> RouterResult<()> {
        if !self.is_empty_result() {
            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;
            let redis_key = Self::get_surcharge_metadata_redis_key(&self.payment_attempt_id);

            let mut value_list = Vec::with_capacity(self.get_surcharge_results_size());
            for (key, value) in self.get_individual_surcharge_key_value_pairs().into_iter() {
                value_list.push((
                    key,
                    value
                        .encode_to_string_of_json()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to encode to string of json")?,
                ));
            }
            let intent_fulfillment_time = business_profile
                .get_order_fulfillment_time()
                .unwrap_or(consts::DEFAULT_FULFILLMENT_TIME);
            redis_conn
                .set_hash_fields(
                    &redis_key.as_str().into(),
                    value_list,
                    Some(intent_fulfillment_time),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to write to redis")?;
            logger::debug!("Surcharge results stored in redis with key = {}", redis_key);
        }
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn get_individual_surcharge_detail_from_redis(
        state: &PaymentMethodsState,
        surcharge_key: SurchargeKey,
        payment_attempt_id: &str,
    ) -> CustomResult<SurchargeDetails, RedisError> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .attach_printable("Failed to get redis connection")?;
        let redis_key = Self::get_surcharge_metadata_redis_key(payment_attempt_id);
        let value_key = Self::get_surcharge_details_redis_hashset_key(&surcharge_key);
        let result = redis_conn
            .get_hash_field_and_deserialize(
                &redis_key.as_str().into(),
                &value_key,
                "SurchargeDetails",
            )
            .await;
        logger::debug!(
            "Surcharge result fetched from redis with key = {} and {}",
            redis_key,
            value_key
        );
        result
    }
}

impl ForeignTryFrom<&storage::Authentication> for AuthenticationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(authentication: &storage::Authentication) -> Result<Self, Self::Error> {
        if authentication.authentication_status == common_enums::AuthenticationStatus::Success {
            let threeds_server_transaction_id =
                authentication.threeds_server_transaction_id.clone();
            let message_version = authentication.message_version.clone();
            let cavv = authentication
                .cavv
                .clone()
                .get_required_value("cavv")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("cavv must not be null when authentication_status is success")?;
            Ok(Self {
                eci: authentication.eci.clone(),
                cavv,
                threeds_server_transaction_id,
                message_version,
                ds_trans_id: authentication.ds_trans_id.clone(),
            })
        } else {
            Err(errors::ApiErrorResponse::PaymentAuthenticationFailed { data: None }.into())
        }
    }
}
