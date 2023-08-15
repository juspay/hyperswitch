use std::collections::HashMap;

use error_stack::{IntoReport, ResultExt};

use crate::{
    core::errors::{self, RouterResult},
    types::storage::{self, enums as storage_enums},
};

#[derive(Clone, Debug)]
pub struct MultipleCaptureData {
    //key -> capture_id, value -> Capture
    all_captures: HashMap<String, storage::Capture>,
    pub capture_operation: CaptureOperation,
    _private: Private, // to restrict direct construction of MultipleCaptureData
}
#[derive(Clone, Debug)]
struct Private {}

#[derive(Clone, Debug)]
pub enum CaptureOperation {
    CaptureSync,
    CaptureCreate {
        latest_capture: Box<storage::Capture>,
    },
}

impl MultipleCaptureData {
    pub fn new_sync(captures: Vec<storage::Capture>) -> RouterResult<Self> {
        let multiple_capture_data = Self {
            all_captures: captures
                .into_iter()
                .map(|capture| (capture.capture_id.clone(), capture))
                .collect(),
            capture_operation: CaptureOperation::CaptureSync,
            _private: Private {},
        };
        Ok(multiple_capture_data)
    }

    pub fn new_create(
        mut previous_captures: Vec<storage::Capture>,
        new_capture: storage::Capture,
    ) -> RouterResult<Self> {
        previous_captures.push(new_capture.clone());
        Ok(Self {
            all_captures: previous_captures
                .into_iter()
                .map(|capture| (capture.capture_id.clone(), capture))
                .collect(),
            capture_operation: CaptureOperation::CaptureCreate {
                latest_capture: Box::new(new_capture),
            },
            _private: Private {},
        })
    }

    pub fn update_capture(&mut self, updated_capture: storage::Capture) {
        let capture_id = &updated_capture.capture_id;
        if self.all_captures.contains_key(capture_id) {
            self.all_captures
                .entry(capture_id.into())
                .and_modify(|capture| *capture = updated_capture);
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
        // hash_map
        //     .entry(self.latest_capture.status)
        //     .and_modify(|count| *count += 1);
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
            storage_enums::AttemptStatus::PartialCharged
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
}
