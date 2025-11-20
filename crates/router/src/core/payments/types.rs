use std::{collections::HashMap, num::TryFromIntError};

use api_models::payment_methods::SurchargeDetailsResponse;
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, OptionExt},
    types::{self as common_types, ConnectorTransactionIdTrait},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt;
pub use hyperswitch_domain_models::router_request_types::{
    self, AuthenticationData, SplitRefundsRequest, StripeSplitRefund, SurchargeDetails,
};
use redis_interface::errors::RedisError;
use router_env::{instrument, logger, tracing};

use crate::{
    consts as router_consts,
    core::errors::{self, RouterResult},
    routes::SessionState,
    types::{
        domain::Profile,
        storage::{self, enums as storage_enums},
        transformers::ForeignTryFrom,
    },
};

#[derive(Clone, Debug)]
pub struct MultipleCaptureData {
    // key -> capture_id, value -> Capture
    all_captures: HashMap<String, storage::Capture>,
    latest_capture: storage::Capture,
    pub expand_captures: Option<bool>,
    _private: Private, // to restrict direct construction of MultipleCaptureData
}
#[derive(Clone, Debug)]
struct Private {}

impl MultipleCaptureData {
    pub fn new_for_sync(
        captures: Vec<storage::Capture>,
        expand_captures: Option<bool>,
    ) -> RouterResult<Self> {
        let latest_capture = captures
            .last()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Cannot create MultipleCaptureData with empty captures list")?
            .clone();
        let multiple_capture_data = Self {
            all_captures: captures
                .into_iter()
                .map(|capture| (capture.capture_id.clone(), capture))
                .collect(),
            latest_capture,
            _private: Private {},
            expand_captures,
        };
        Ok(multiple_capture_data)
    }

    pub fn new_for_create(
        mut previous_captures: Vec<storage::Capture>,
        new_capture: storage::Capture,
    ) -> Self {
        previous_captures.push(new_capture.clone());
        Self {
            all_captures: previous_captures
                .into_iter()
                .map(|capture| (capture.capture_id.clone(), capture))
                .collect(),
            latest_capture: new_capture,
            _private: Private {},
            expand_captures: None,
        }
    }

    pub fn update_capture(&mut self, updated_capture: storage::Capture) {
        let capture_id = &updated_capture.capture_id;
        if self.all_captures.contains_key(capture_id) {
            self.all_captures
                .entry(capture_id.into())
                .and_modify(|capture| *capture = updated_capture.clone());
        }
    }
    pub fn get_total_blocked_amount(&self) -> common_types::MinorUnit {
        self.all_captures
            .iter()
            .fold(common_types::MinorUnit::new(0), |accumulator, capture| {
                accumulator
                    + match capture.1.status {
                        storage_enums::CaptureStatus::Charged
                        | storage_enums::CaptureStatus::Pending => capture.1.amount,
                        storage_enums::CaptureStatus::Started
                        | storage_enums::CaptureStatus::Failed => common_types::MinorUnit::new(0),
                    }
            })
    }
    pub fn get_total_charged_amount(&self) -> common_types::MinorUnit {
        self.all_captures
            .iter()
            .fold(common_types::MinorUnit::new(0), |accumulator, capture| {
                accumulator
                    + match capture.1.status {
                        storage_enums::CaptureStatus::Charged => capture.1.amount,
                        storage_enums::CaptureStatus::Pending
                        | storage_enums::CaptureStatus::Started
                        | storage_enums::CaptureStatus::Failed => common_types::MinorUnit::new(0),
                    }
            })
    }
    pub fn get_captures_count(&self) -> RouterResult<i16> {
        i16::try_from(self.all_captures.len())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while converting from usize to i16")
    }
    pub fn get_status_count(&self) -> HashMap<storage_enums::CaptureStatus, i16> {
        let mut hash_map: HashMap<storage_enums::CaptureStatus, i16> = HashMap::new();
        hash_map.insert(storage_enums::CaptureStatus::Charged, 0);
        hash_map.insert(storage_enums::CaptureStatus::Pending, 0);
        hash_map.insert(storage_enums::CaptureStatus::Started, 0);
        hash_map.insert(storage_enums::CaptureStatus::Failed, 0);
        self.all_captures
            .iter()
            .fold(hash_map, |mut accumulator, capture| {
                let current_capture_status = capture.1.status;
                accumulator
                    .entry(current_capture_status)
                    .and_modify(|count| *count += 1);
                accumulator
            })
    }
    pub fn get_attempt_status(
        &self,
        authorized_amount: common_types::MinorUnit,
    ) -> storage_enums::AttemptStatus {
        let total_captured_amount = self.get_total_charged_amount();
        if authorized_amount == total_captured_amount {
            return storage_enums::AttemptStatus::Charged;
        }
        let status_count_map = self.get_status_count();
        if status_count_map.get(&storage_enums::CaptureStatus::Charged) > Some(&0) {
            storage_enums::AttemptStatus::PartialChargedAndChargeable
        } else {
            storage_enums::AttemptStatus::CaptureInitiated
        }
    }
    pub fn get_pending_captures(&self) -> Vec<&storage::Capture> {
        self.all_captures
            .iter()
            .filter(|capture| capture.1.status == storage_enums::CaptureStatus::Pending)
            .map(|key_value| key_value.1)
            .collect()
    }
    pub fn get_all_captures(&self) -> Vec<&storage::Capture> {
        self.all_captures
            .iter()
            .map(|key_value| key_value.1)
            .collect()
    }
    pub fn get_capture_by_capture_id(&self, capture_id: String) -> Option<&storage::Capture> {
        self.all_captures.get(&capture_id)
    }
    pub fn get_capture_by_connector_capture_id(
        &self,
        connector_capture_id: &String,
    ) -> Option<&storage::Capture> {
        self.all_captures
            .iter()
            .find(|(_, capture)| {
                capture.get_optional_connector_transaction_id() == Some(connector_capture_id)
            })
            .map(|(_, capture)| capture)
    }
    pub fn get_latest_capture(&self) -> &storage::Capture {
        &self.latest_capture
    }
    pub fn get_pending_connector_capture_ids(&self) -> Vec<String> {
        let pending_connector_capture_ids = self
            .get_pending_captures()
            .into_iter()
            .filter_map(|capture| capture.get_optional_connector_transaction_id().cloned())
            .collect();
        pending_connector_capture_ids
    }
    pub fn get_pending_captures_without_connector_capture_id(&self) -> Vec<&storage::Capture> {
        self.get_pending_captures()
            .into_iter()
            .filter(|capture| capture.get_optional_connector_transaction_id().is_none())
            .collect()
    }
}

#[cfg(feature = "v2")]
impl ForeignTryFrom<(&SurchargeDetails, &PaymentAttempt)> for SurchargeDetailsResponse {
    type Error = TryFromIntError;
    fn foreign_try_from(
        (surcharge_details, payment_attempt): (&SurchargeDetails, &PaymentAttempt),
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl ForeignTryFrom<(&SurchargeDetails, &PaymentAttempt)> for SurchargeDetailsResponse {
    type Error = TryFromIntError;
    fn foreign_try_from(
        (surcharge_details, payment_attempt): (&SurchargeDetails, &PaymentAttempt),
    ) -> Result<Self, Self::Error> {
        let currency = payment_attempt.currency.unwrap_or_default();
        let display_surcharge_amount = currency
            .to_currency_base_unit_asf64(surcharge_details.surcharge_amount.get_amount_as_i64())?;
        let display_tax_on_surcharge_amount = currency.to_currency_base_unit_asf64(
            surcharge_details
                .tax_on_surcharge_amount
                .get_amount_as_i64(),
        )?;
        let display_total_surcharge_amount = currency.to_currency_base_unit_asf64(
            (surcharge_details.surcharge_amount + surcharge_details.tax_on_surcharge_amount)
                .get_amount_as_i64(),
        )?;
        Ok(Self {
            surcharge: surcharge_details.surcharge.clone().into(),
            tax_on_surcharge: surcharge_details.tax_on_surcharge.clone().map(Into::into),
            display_surcharge_amount,
            display_tax_on_surcharge_amount,
            display_total_surcharge_amount,
        })
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
        format!("surcharge_metadata_{payment_attempt_id}")
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
                format!("token_{token}")
            }
            SurchargeKey::PaymentMethodData(payment_method, payment_method_type, card_network) => {
                if let Some(card_network) = card_network {
                    format!("{payment_method}_{payment_method_type}_{card_network}")
                } else {
                    format!("{payment_method}_{payment_method_type}")
                }
            }
        }
    }
    #[instrument(skip_all)]
    pub async fn persist_individual_surcharge_details_in_redis(
        &self,
        state: &SessionState,
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
                .unwrap_or(router_consts::DEFAULT_FULFILLMENT_TIME);
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
        state: &SessionState,
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

impl ForeignTryFrom<&router_request_types::authentication::AuthenticationStore>
    for router_request_types::UcsAuthenticationData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        authentication_store: &router_request_types::authentication::AuthenticationStore,
    ) -> Result<Self, Self::Error> {
        let authentication = &authentication_store.authentication;
        if authentication.authentication_status == common_enums::AuthenticationStatus::Success {
            let threeds_server_transaction_id =
                authentication.threeds_server_transaction_id.clone();
            let message_version = authentication.message_version.clone();
            let cavv = authentication_store
                .cavv
                .clone()
                .get_required_value("cavv")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("cavv must not be null when authentication_status is success")?;
            Ok(Self {
                trans_status: authentication.trans_status.clone(),
                eci: authentication.eci.clone(),
                cavv: Some(cavv),
                threeds_server_transaction_id,
                message_version,
                ds_trans_id: authentication.ds_trans_id.clone(),
                acs_trans_id: authentication.acs_trans_id.clone(),
                transaction_id: authentication.connector_authentication_id.clone(),
                ucaf_collection_indicator: None,
            })
        } else {
            Err(errors::ApiErrorResponse::PaymentAuthenticationFailed { data: None }.into())
        }
    }
}

impl ForeignTryFrom<&router_request_types::authentication::AuthenticationStore>
    for AuthenticationData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        authentication_store: &router_request_types::authentication::AuthenticationStore,
    ) -> Result<Self, Self::Error> {
        let authentication = &authentication_store.authentication;
        if authentication.authentication_status == common_enums::AuthenticationStatus::Success {
            let threeds_server_transaction_id =
                authentication.threeds_server_transaction_id.clone();
            let message_version = authentication.message_version.clone();
            let cavv = authentication_store
                .cavv
                .clone()
                .get_required_value("cavv")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("cavv must not be null when authentication_status is success")?;
            Ok(Self {
                eci: authentication.eci.clone(),
                created_at: authentication.created_at,
                cavv,
                threeds_server_transaction_id,
                message_version,
                ds_trans_id: authentication.ds_trans_id.clone(),
                authentication_type: authentication.authentication_type,
                challenge_code: authentication.challenge_code.clone(),
                challenge_cancel: authentication.challenge_cancel.clone(),
                challenge_code_reason: authentication.challenge_code_reason.clone(),
                message_extension: authentication.message_extension.clone(),
                acs_trans_id: authentication.acs_trans_id.clone(),
                transaction_status: authentication.trans_status.clone(),
                exemption_indicator: None,
                cb_network_params: None,
            })
        } else {
            Err(errors::ApiErrorResponse::PaymentAuthenticationFailed { data: None }.into())
        }
    }
}

impl ForeignTryFrom<&api_models::payments::ExternalThreeDsData> for AuthenticationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        external_auth_data: &api_models::payments::ExternalThreeDsData,
    ) -> Result<Self, Self::Error> {
        let cavv = match &external_auth_data.authentication_cryptogram {
            api_models::payments::Cryptogram::Cavv {
                authentication_cryptogram,
            } => authentication_cryptogram.clone(),
        };

        Ok(Self {
            eci: Some(external_auth_data.eci.clone()),
            cavv,
            threeds_server_transaction_id: Some(external_auth_data.ds_trans_id.clone()),
            message_version: Some(external_auth_data.version.clone()),
            ds_trans_id: Some(external_auth_data.ds_trans_id.clone()),
            created_at: time::PrimitiveDateTime::new(
                time::OffsetDateTime::now_utc().date(),
                time::OffsetDateTime::now_utc().time(),
            ),
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            acs_trans_id: None,
            authentication_type: None,
            transaction_status: Some(external_auth_data.transaction_status.clone()),
            exemption_indicator: external_auth_data.exemption_indicator.clone(),
            cb_network_params: external_auth_data.network_params.clone(),
        })
    }
}
