use std::{collections::HashMap, num::TryFromIntError};

use api_models::{payment_methods::SurchargeDetailsResponse, payments::RequestSurchargeDetails};
use common_utils::{consts, errors::CustomResult, ext_traits::Encode, types as common_types};
use data_models::payments::payment_attempt::PaymentAttempt;
use diesel_models::business_profile::BusinessProfile;
use error_stack::{IntoReport, ResultExt};
use redis_interface::errors::RedisError;
use router_env::{instrument, tracing};

use crate::{
    consts as router_consts,
    core::errors::{self, RouterResult},
    routes::AppState,
    types::{
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
        /// Creates a new MultipleCaptureData instance for synchronous operations.
    ///
    /// It takes a vector of Capture objects and an optional boolean flag to expand captures. 
    /// It generates a new MultipleCaptureData instance with the provided captures and expand_captures flag, 
    /// and returns a RouterResult containing the new MultipleCaptureData instance.
    pub fn new_for_sync(
        captures: Vec<storage::Capture>,
        expand_captures: Option<bool>,
    ) -> RouterResult<Self> {
        let latest_capture = captures
            .last()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .into_report()
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

        /// Creates a new instance of the struct for capturing data. It takes a vector of previous captures and a new capture, adds the new capture to the vector of previous captures, and then constructs a new instance of the struct with the updated captures, the latest capture, and other default values.
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

        /// Updates the capture with the provided capture id with the new updated capture.
    /// If the capture id exists in the all_captures map, the method will update the existing capture with the new updated capture.
    pub fn update_capture(&mut self, updated_capture: storage::Capture) {
        let capture_id = &updated_capture.capture_id;
        if self.all_captures.contains_key(capture_id) {
            self.all_captures
                .entry(capture_id.into())
                .and_modify(|capture| *capture = updated_capture.clone());
        }
    }
        /// Get the total amount of blocked funds based on the status of all captures.
    pub fn get_total_blocked_amount(&self) -> i64 {
        self.all_captures.iter().fold(0, |accumulator, capture| {
            accumulator
                + match capture.1.status {
                    storage_enums::CaptureStatus::Charged
                    | storage_enums::CaptureStatus::Pending => capture.1.amount,
                    storage_enums::CaptureStatus::Started
                    | storage_enums::CaptureStatus::Failed => 0,
                }
        })
    }
        /// Calculates the total charged amount by iterating through all captures and summing up the amounts of those that are in a 'Charged' status. 
    /// Returns the total charged amount as an i64.
    pub fn get_total_charged_amount(&self) -> i64 {
        self.all_captures.iter().fold(0, |accumulator, capture| {
            accumulator
                + match capture.1.status {
                    storage_enums::CaptureStatus::Charged => capture.1.amount,
                    storage_enums::CaptureStatus::Pending
                    | storage_enums::CaptureStatus::Started
                    | storage_enums::CaptureStatus::Failed => 0,
                }
        })
    }
        /// Returns the number of captures as a result of converting the length of all_captures to i16.
    pub fn get_captures_count(&self) -> RouterResult<i16> {
        i16::try_from(self.all_captures.len())
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while converting from usize to i16")
    }
        /// This method returns a HashMap containing the count of each capture status (Charged, Pending, Started, Failed) present in the all_captures field of the struct.
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
        /// Returns the status of the attempt based on the authorized amount and the total captured amount.
    /// If the authorized amount matches the total captured amount, it returns AttemptStatus::Charged.
    /// If there are partial charges and still some amount is chargeable, it returns AttemptStatus::PartialChargedAndChargeable.
    /// If no amount has been charged yet, it returns AttemptStatus::CaptureInitiated.
    pub fn get_attempt_status(&self, authorized_amount: i64) -> storage_enums::AttemptStatus {
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
        /// Returns a vector containing references to all pending captures in the internal storage.
    pub fn get_pending_captures(&self) -> Vec<&storage::Capture> {
        self.all_captures
            .iter()
            .filter(|capture| capture.1.status == storage_enums::CaptureStatus::Pending)
            .map(|key_value| key_value.1)
            .collect()
    }
        /// Returns a vector containing references to all the captures stored in the `storage` module. 
    pub fn get_all_captures(&self) -> Vec<&storage::Capture> {
        self.all_captures
            .iter()
            .map(|key_value| key_value.1)
            .collect()
    }
        /// Returns an optional reference to a `Capture` object based on the provided `capture_id`.
    /// If a `Capture` with the given `capture_id` exists in the `all_captures` map, it returns a reference to it. If not, it returns `None`.
    pub fn get_capture_by_capture_id(&self, capture_id: String) -> Option<&storage::Capture> {
        self.all_captures.get(&capture_id)
    }
        /// Retrieves a capture from the list of all captures based on the provided connector_capture_id.
    /// If a capture with the given connector_capture_id is found, it returns a reference to the capture,
    /// otherwise it returns None.
    pub fn get_capture_by_connector_capture_id(
        &self,
        connector_capture_id: String,
    ) -> Option<&storage::Capture> {
        self.all_captures
            .iter()
            .find(|(_, capture)| capture.connector_capture_id == Some(connector_capture_id.clone()))
            .map(|(_, capture)| capture)
    }
        /// Returns a reference to the latest capture in the storage.
    pub fn get_latest_capture(&self) -> &storage::Capture {
        &self.latest_capture
    }
        /// Returns a vector of pending connector capture IDs.
    pub fn get_pending_connector_capture_ids(&self) -> Vec<String> {
        let pending_connector_capture_ids = self
            .get_pending_captures()
            .into_iter()
            .filter_map(|capture| capture.connector_capture_id.clone())
            .collect();
        pending_connector_capture_ids
    }
        /// Returns a vector of pending captures that do not have a connector_capture_id.
    pub fn get_pending_captures_without_connector_capture_id(&self) -> Vec<&storage::Capture> {
        self.get_pending_captures()
            .into_iter()
            .filter(|capture| capture.connector_capture_id.is_none())
            .collect()
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SurchargeDetails {
    /// original_amount
    pub original_amount: i64,
    /// surcharge value
    pub surcharge: common_types::Surcharge,
    /// tax on surcharge value
    pub tax_on_surcharge:
        Option<common_types::Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>>,
    /// surcharge amount for this payment
    pub surcharge_amount: i64,
    /// tax on surcharge amount for this payment
    pub tax_on_surcharge_amount: i64,
    /// sum of original amount,
    pub final_amount: i64,
}

impl From<(&RequestSurchargeDetails, &PaymentAttempt)> for SurchargeDetails {
        /// Constructs a new instance of Self using the given request_surcharge_details and payment_attempt.
    /// The surcharge_amount and tax_on_surcharge_amount are obtained from request_surcharge_details, while original_amount
    /// is obtained from payment_attempt. The final_amount is calculated by adding original_amount, surcharge_amount, and
    /// tax_on_surcharge_amount.
    fn from(
        (request_surcharge_details, payment_attempt): (&RequestSurchargeDetails, &PaymentAttempt),
    ) -> Self {
        let surcharge_amount = request_surcharge_details.surcharge_amount;
        let tax_on_surcharge_amount = request_surcharge_details.tax_amount.unwrap_or(0);
        Self {
            original_amount: payment_attempt.amount,
            surcharge: common_types::Surcharge::Fixed(request_surcharge_details.surcharge_amount),
            tax_on_surcharge: None,
            surcharge_amount,
            tax_on_surcharge_amount,
            final_amount: payment_attempt.amount + surcharge_amount + tax_on_surcharge_amount,
        }
    }
}

impl ForeignTryFrom<(&SurchargeDetails, &PaymentAttempt)> for SurchargeDetailsResponse {
    type Error = TryFromIntError;
        /// Converts the given SurchargeDetails and PaymentAttempt into the current type, handling currency conversions and creating a new instance.
    fn foreign_try_from(
        (surcharge_details, payment_attempt): (&SurchargeDetails, &PaymentAttempt),
    ) -> Result<Self, Self::Error> {
        let currency = payment_attempt.currency.unwrap_or_default();
        let display_surcharge_amount =
            currency.to_currency_base_unit_asf64(surcharge_details.surcharge_amount)?;
        let display_tax_on_surcharge_amount =
            currency.to_currency_base_unit_asf64(surcharge_details.tax_on_surcharge_amount)?;
        let display_final_amount =
            currency.to_currency_base_unit_asf64(surcharge_details.final_amount)?;
        let display_total_surcharge_amount = currency.to_currency_base_unit_asf64(
            surcharge_details.surcharge_amount + surcharge_details.tax_on_surcharge_amount,
        )?;
        Ok(Self {
            surcharge: surcharge_details.surcharge.clone().into(),
            tax_on_surcharge: surcharge_details.tax_on_surcharge.clone().map(Into::into),
            display_surcharge_amount,
            display_tax_on_surcharge_amount,
            display_total_surcharge_amount,
            display_final_amount,
        })
    }
}

impl SurchargeDetails {
        /// Check if the request surcharge details match the surcharge amount and tax amount of the current object.
    pub fn is_request_surcharge_matching(
        &self,
        request_surcharge_details: RequestSurchargeDetails,
    ) -> bool {
        request_surcharge_details.surcharge_amount == self.surcharge_amount
            && request_surcharge_details.tax_amount.unwrap_or(0) == self.tax_on_surcharge_amount
    }
        /// This method calculates the total surcharge amount by adding the surcharge amount and the tax on surcharge amount.
    pub fn get_total_surcharge_amount(&self) -> i64 {
        self.surcharge_amount + self.tax_on_surcharge_amount
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
        /// Creates a new instance of PaymentResult with the provided payment_attempt_id.
    pub fn new(payment_attempt_id: String) -> Self {
        Self {
            surcharge_results: HashMap::new(),
            payment_attempt_id,
        }
    }
        /// Checks if the `surcharge_results` field of the struct is empty.
    pub fn is_empty_result(&self) -> bool {
        self.surcharge_results.is_empty()
    }
        /// Returns the size of the surcharge_results vector, which represents the number of surcharge results stored in the object.
    pub fn get_surcharge_results_size(&self) -> usize {
        self.surcharge_results.len()
    }
        /// Inserts the surcharge details for a given surcharge key into the surcharge_results map.
    ///
    /// # Arguments
    ///
    /// * `surcharge_key` - The key for the surcharge details to be inserted.
    /// * `surcharge_details` - The details of the surcharge to be inserted.
    ///
    pub fn insert_surcharge_details(
        &mut self,
        surcharge_key: SurchargeKey,
        surcharge_details: SurchargeDetails,
    ) {
        self.surcharge_results
            .insert(surcharge_key, surcharge_details);
    }
        /// Returns the surcharge details for the given surcharge key, if it exists.
    /// 
    /// # Arguments
    /// * `surcharge_key` - The key used to retrieve the surcharge details.
    ///
    /// # Returns
    /// * `Option<&SurchargeDetails>` - Some(surcharge_details) if the surcharge details exist for the given key, None if not.
    pub fn get_surcharge_details(&self, surcharge_key: SurchargeKey) -> Option<&SurchargeDetails> {
        self.surcharge_results.get(&surcharge_key)
    }
        /// Constructs a Redis key for retrieving surcharge metadata based on the given payment attempt ID.
    ///
    /// # Arguments
    ///
    /// * `payment_attempt_id` - A reference to a string representing the payment attempt ID
    ///
    /// # Returns
    ///
    /// A string representing the Redis key for retrieving surcharge metadata
    ///
    pub fn get_surcharge_metadata_redis_key(payment_attempt_id: &str) -> String {
        format!("surcharge_metadata_{}", payment_attempt_id)
    }
    /// Returns a vector of key-value pairs where each key is a string representing the Redis hashset key for a surcharge, and each value is the surcharge details associated with that key. The method iterates through the surcharge results, maps each surcharge key to its corresponding Redis hashset key, and collects the resulting key-value pairs into a vector.
    pub fn get_individual_surcharge_key_value_pairs(&self) -> Vec<(String, SurchargeDetails)> {
        self.surcharge_results
            .iter()
            .map(|(surcharge_key, surcharge_details)| {
                let key = Self::get_surcharge_details_redis_hashset_key(surcharge_key);
                (key, surcharge_details.to_owned())
            })
            .collect()
    }
    /// Constructs a Redis hashset key for storing surcharge details based on the given SurchargeKey.
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
        /// Persists individual surcharge details in Redis for a given business profile and app state.
    pub async fn persist_individual_surcharge_details_in_redis(
        &self,
        state: &AppState,
        business_profile: &BusinessProfile,
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
                    Encode::<SurchargeDetails>::encode_to_string_of_json(&value)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to encode to string of json")?,
                ));
            }
            let intent_fulfillment_time = business_profile
                .intent_fulfillment_time
                .unwrap_or(router_consts::DEFAULT_FULFILLMENT_TIME);
            redis_conn
                .set_hash_fields(&redis_key, value_list, Some(intent_fulfillment_time))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to write to redis")?;
        }
        Ok(())
    }

    #[instrument(skip_all)]
        /// Retrieves individual surcharge details from Redis based on the provided surcharge key and payment attempt ID.
    pub async fn get_individual_surcharge_detail_from_redis(
        state: &AppState,
        surcharge_key: SurchargeKey,
        payment_attempt_id: &str,
    ) -> CustomResult<SurchargeDetails, RedisError> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .attach_printable("Failed to get redis connection")?;
        let redis_key = Self::get_surcharge_metadata_redis_key(payment_attempt_id);
        let value_key = Self::get_surcharge_details_redis_hashset_key(&surcharge_key);
        redis_conn
            .get_hash_field_and_deserialize(&redis_key, &value_key, "SurchargeDetails")
            .await
    }
}
