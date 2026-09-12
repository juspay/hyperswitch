use diesel_models::enums::FraudCheckLastStep;
use uuid::Uuid;

use crate::{
    core::{
        errors::{RouterResult, StorageErrorExt},
        fraud_check::{types::PayoutFrmData, ConnectorDetailsCore},
        payouts::PayoutData,
    },
    errors,
    types::{
        fraud_check::{FraudCheckResponseData, FrmPayoutRouterData},
        storage::{
            enums::{FraudCheckStatus, FraudCheckType},
            fraud_check::{FraudCheckNew, FraudCheckUpdate},
        },
    },
    SessionState,
};

#[derive(Debug, Clone, Copy)]
pub struct FraudCheckPrePayout;

impl FraudCheckPrePayout {
    pub async fn get_trackers(
        &self,
        state: &SessionState,
        payout_data: &PayoutData,
        connector_details: ConnectorDetailsCore,
    ) -> RouterResult<Option<PayoutFrmData>> {
        let db = &*state.store;

        let payout_id = payout_data.payouts.payout_id.clone();
        let fraud_check_value = db
            .insert_fraud_check_response(FraudCheckNew {
                frm_id: Uuid::new_v4().simple().to_string(),
                payment_id: None,
                payout_id: Some(payout_id.clone()),
                merchant_id: payout_data.payouts.merchant_id.clone(),
                processor_merchant_id: payout_data.payouts.processor_merchant_id.clone(),
                attempt_id: payout_data.payout_attempt.payout_attempt_id.clone(),
                created_at: common_utils::date_time::now(),
                frm_name: connector_details.connector_name.clone(),
                frm_transaction_id: None,
                frm_transaction_type: FraudCheckType::PreFrm,
                frm_status: FraudCheckStatus::Pending,
                frm_score: None,
                frm_reason: None,
                frm_error: None,
                payment_details: None,
                metadata: None,
                modified_at: common_utils::date_time::now(),
                last_step: FraudCheckLastStep::Processing,
                payment_capture_method: None,
                created_by: None,
            })
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout { payout_id })?;

        Ok(Some(PayoutFrmData {
            fraud_check: fraud_check_value,
            amount: payout_data.payouts.amount,
            currency: payout_data.payouts.destination_currency,
            payout_attempt: payout_data.payout_attempt.clone(),
            customer_details: payout_data.customer_details.clone(),
            payout_method_data: payout_data.payout_method_data.clone(),
            billing_address: payout_data.billing_address.clone(),
        }))
    }

    pub async fn update_tracker(
        &self,
        state: &SessionState,
        mut frm_data: PayoutFrmData,
        router_data: FrmPayoutRouterData,
    ) -> RouterResult<PayoutFrmData> {
        let db = &*state.store;

        let fraud_check_update = match router_data.response {
            Ok(FraudCheckResponseData::TransactionResponse {
                resource_id,
                status,
                connector_metadata,
                reason,
                score,
            }) => FraudCheckUpdate::ResponseUpdate {
                frm_status: status,
                frm_transaction_id: resource_id.get_optional_response_id(),
                frm_reason: reason,
                frm_score: score,
                metadata: connector_metadata,
                modified_at: common_utils::date_time::now(),
                last_step: frm_data.fraud_check.last_step,
                payment_capture_method: None,
            },
            Err(error) => FraudCheckUpdate::ErrorUpdate {
                status: FraudCheckStatus::TransactionFailure,
                error_message: Some(Some(error.message)),
            },
            Ok(_) => FraudCheckUpdate::ErrorUpdate {
                status: FraudCheckStatus::TransactionFailure,
                error_message: Some(Some(
                    "Unexpected response type for payout fraud check".to_string(),
                )),
            },
        };

        frm_data.fraud_check = db
            .update_fraud_check_response_with_frm_id(
                frm_data.fraud_check.clone(),
                fraud_check_update,
            )
            .await
            .map_err(|error| error.change_context(errors::ApiErrorResponse::InternalServerError))?;

        Ok(frm_data)
    }
}
