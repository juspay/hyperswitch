use std::{collections::HashMap, num::TryFromIntError};

use api_models::{payment_methods::SurchargeDetailsResponse, payments::RequestSurchargeDetails};
use common_utils::{consts, types as common_types};
use data_models::payments::payment_attempt::PaymentAttempt;
use error_stack::{IntoReport, ResultExt};

use crate::{
    core::errors::{self, RouterResult},
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
    pub fn get_captures_count(&self) -> RouterResult<i16> {
        i16::try_from(self.all_captures.len())
            .into_report()
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
        connector_capture_id: String,
    ) -> Option<&storage::Capture> {
        self.all_captures
            .iter()
            .find(|(_, capture)| capture.connector_capture_id == Some(connector_capture_id.clone()))
            .map(|(_, capture)| capture)
    }
    pub fn get_latest_capture(&self) -> &storage::Capture {
        &self.latest_capture
    }
    pub fn get_pending_connector_capture_ids(&self) -> Vec<String> {
        let pending_connector_capture_ids = self
            .get_pending_captures()
            .into_iter()
            .filter_map(|capture| capture.connector_capture_id.clone())
            .collect();
        pending_connector_capture_ids
    }
    pub fn get_pending_captures_without_connector_capture_id(&self) -> Vec<&storage::Capture> {
        self.get_pending_captures()
            .into_iter()
            .filter(|capture| capture.connector_capture_id.is_none())
            .collect()
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SurchargeDetails {
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
    fn from(
        (request_surcharge_details, payment_attempt): (&RequestSurchargeDetails, &PaymentAttempt),
    ) -> Self {
        let surcharge_amount = request_surcharge_details.surcharge_amount;
        let tax_on_surcharge_amount = request_surcharge_details.tax_amount.unwrap_or(0);
        Self {
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
        Ok(Self {
            surcharge: surcharge_details.surcharge.clone(),
            tax_on_surcharge: surcharge_details.tax_on_surcharge.clone(),
            display_surcharge_amount,
            display_tax_on_surcharge_amount,
            display_total_surcharge_amount: display_surcharge_amount
                + display_tax_on_surcharge_amount,
            display_final_amount,
        })
    }
}

impl SurchargeDetails {
    pub fn is_request_surcharge_matching(
        &self,
        request_surcharge_details: RequestSurchargeDetails,
    ) -> bool {
        request_surcharge_details.surcharge_amount == self.surcharge_amount
            && request_surcharge_details.tax_amount.unwrap_or(0) == self.tax_on_surcharge_amount
    }
    pub fn get_total_surcharge_amount(&self) -> i64 {
        self.surcharge_amount + self.tax_on_surcharge_amount
    }
}

#[derive(Clone, Debug)]
pub struct SurchargeMetadata {
    surcharge_results: HashMap<
        (
            common_enums::PaymentMethod,
            common_enums::PaymentMethodType,
            Option<common_enums::CardNetwork>,
        ),
        SurchargeDetails,
    >,
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
        payment_method: &common_enums::PaymentMethod,
        payment_method_type: &common_enums::PaymentMethodType,
        card_network: Option<&common_enums::CardNetwork>,
        surcharge_details: SurchargeDetails,
    ) {
        let key = (
            payment_method.to_owned(),
            payment_method_type.to_owned(),
            card_network.cloned(),
        );
        self.surcharge_results.insert(key, surcharge_details);
    }
    pub fn get_surcharge_details(
        &self,
        payment_method: &common_enums::PaymentMethod,
        payment_method_type: &common_enums::PaymentMethodType,
        card_network: Option<&common_enums::CardNetwork>,
    ) -> Option<&SurchargeDetails> {
        let key = &(
            payment_method.to_owned(),
            payment_method_type.to_owned(),
            card_network.cloned(),
        );
        self.surcharge_results.get(key)
    }
    pub fn get_surcharge_metadata_redis_key(payment_attempt_id: &str) -> String {
        format!("surcharge_metadata_{}", payment_attempt_id)
    }
    pub fn get_individual_surcharge_key_value_pairs(&self) -> Vec<(String, SurchargeDetails)> {
        self.surcharge_results
            .iter()
            .map(|((pm, pmt, card_network), surcharge_details)| {
                let key =
                    Self::get_surcharge_details_redis_hashset_key(pm, pmt, card_network.as_ref());
                (key, surcharge_details.to_owned())
            })
            .collect()
    }
    pub fn get_surcharge_details_redis_hashset_key(
        payment_method: &common_enums::PaymentMethod,
        payment_method_type: &common_enums::PaymentMethodType,
        card_network: Option<&common_enums::CardNetwork>,
    ) -> String {
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
