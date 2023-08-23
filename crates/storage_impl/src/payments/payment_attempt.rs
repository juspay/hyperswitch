use data_models::{
    mandates::{MandateAmountData, MandateDataType},
    payments::payment_attempt::PaymentAttempt,
};
use diesel_models::{
    enums::{MandateAmountData as DieselMandateAmountData, MandateDataType as DieselMandateType},
    payment_attempt::PaymentAttempt as DieselPaymentAttempt,
};

use crate::DataModelExt;

impl DataModelExt for MandateAmountData {
    type StorageModel = DieselMandateAmountData;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselMandateAmountData {
            amount: self.amount,
            currency: self.currency,
            start_date: self.start_date,
            end_date: self.end_date,
            metadata: self.metadata,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            amount: storage_model.amount,
            currency: storage_model.currency,
            start_date: storage_model.start_date,
            end_date: storage_model.end_date,
            metadata: storage_model.metadata,
        }
    }
}

impl DataModelExt for MandateDataType {
    type StorageModel = DieselMandateType;

    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::SingleUse(data) => DieselMandateType::SingleUse(data.to_storage_model()),
            Self::MultiUse(None) => DieselMandateType::MultiUse(None),
            Self::MultiUse(Some(data)) => {
                DieselMandateType::MultiUse(Some(data.to_storage_model()))
            }
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselMandateType::SingleUse(data) => {
                Self::SingleUse(MandateAmountData::from_storage_model(data))
            }
            DieselMandateType::MultiUse(Some(data)) => {
                Self::MultiUse(Some(MandateAmountData::from_storage_model(data)))
            }
            DieselMandateType::MultiUse(None) => Self::MultiUse(None),
        }
    }
}

impl DataModelExt for PaymentAttempt {
    type StorageModel = DieselPaymentAttempt;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttempt {
            id: self.id,
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id: self.connector_transaction_id,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            error_code: self.error_code,
            payment_token: self.payment_token,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(|md| md.to_storage_model()),
            error_reason: self.error_reason,
            multiple_capture_count: self.multiple_capture_count,
            connector_response_reference_id: self.connector_response_reference_id,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            attempt_id: storage_model.attempt_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            save_to_locker: storage_model.save_to_locker,
            connector: storage_model.connector,
            error_message: storage_model.error_message,
            offer_amount: storage_model.offer_amount,
            surcharge_amount: storage_model.surcharge_amount,
            tax_amount: storage_model.tax_amount,
            payment_method_id: storage_model.payment_method_id,
            payment_method: storage_model.payment_method,
            connector_transaction_id: storage_model.connector_transaction_id,
            capture_method: storage_model.capture_method,
            capture_on: storage_model.capture_on,
            confirm: storage_model.confirm,
            authentication_type: storage_model.authentication_type,
            created_at: storage_model.created_at,
            modified_at: storage_model.modified_at,
            last_synced: storage_model.last_synced,
            cancellation_reason: storage_model.cancellation_reason,
            amount_to_capture: storage_model.amount_to_capture,
            mandate_id: storage_model.mandate_id,
            browser_info: storage_model.browser_info,
            error_code: storage_model.error_code,
            payment_token: storage_model.payment_token,
            connector_metadata: storage_model.connector_metadata,
            payment_experience: storage_model.payment_experience,
            payment_method_type: storage_model.payment_method_type,
            payment_method_data: storage_model.payment_method_data,
            business_sub_label: storage_model.business_sub_label,
            straight_through_algorithm: storage_model.straight_through_algorithm,
            preprocessing_step_id: storage_model.preprocessing_step_id,
            mandate_details: storage_model
                .mandate_details
                .map(MandateDataType::from_storage_model),
            error_reason: storage_model.error_reason,
            multiple_capture_count: storage_model.multiple_capture_count,
            connector_response_reference_id: storage_model.connector_response_reference_id,
        }
    }
}
