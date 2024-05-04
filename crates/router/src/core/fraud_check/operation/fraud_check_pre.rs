use async_trait::async_trait;
use common_enums::FrmSuggestion;
use common_utils::ext_traits::Encode;
use diesel_models::enums::FraudCheckLastStep;
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{Domain, FraudCheckOperation, GetTracker, UpdateTracker};
use crate::{
    core::{
        errors::RouterResult,
        fraud_check::{
            self as frm_core,
            types::{FrmData, PaymentDetails, PaymentToFrmData},
            ConnectorDetailsCore,
        },
        payments,
    },
    db::StorageInterface,
    errors,
    types::{
        api::fraud_check as frm_api,
        domain,
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckResponseData, FraudCheckTransactionData, FrmRequest,
            FrmResponse, FrmRouterData,
        },
        storage::{
            enums::{FraudCheckStatus, FraudCheckType},
            fraud_check::{FraudCheckNew, FraudCheckUpdate},
        },
        ResponseId,
    },
    AppState,
};

#[derive(Debug, Clone, Copy)]
pub struct FraudCheckPre;

impl<F: Clone + Send> FraudCheckOperation<F> for &FraudCheckPre {
    fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<PaymentToFrmData> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F>)> {
        Ok(*self)
    }
    fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<FrmData, F> + Send + Sync)> {
        Ok(*self)
    }
}

impl<F: Clone + Send> FraudCheckOperation<F> for FraudCheckPre {
    fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<PaymentToFrmData> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F>)> {
        Ok(self)
    }
    fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<FrmData, F> + Send + Sync)> {
        Ok(self)
    }
}

#[async_trait]
impl GetTracker<PaymentToFrmData> for FraudCheckPre {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: PaymentToFrmData,
        frm_connector_details: ConnectorDetailsCore,
    ) -> RouterResult<Option<FrmData>> {
        let db = &*state.store;

        let payment_details: Option<serde_json::Value> = PaymentDetails::from(payment_data.clone())
            .encode_to_value()
            .ok();

        let existing_fraud_check = db
            .find_fraud_check_by_payment_id_if_present(
                payment_data.payment_intent.payment_id.clone(),
                payment_data.merchant_account.merchant_id.clone(),
            )
            .await
            .ok();

        let fraud_check = match existing_fraud_check {
            Some(Some(fraud_check)) => Ok(fraud_check),
            _ => {
                db.insert_fraud_check_response(FraudCheckNew {
                    frm_id: Uuid::new_v4().simple().to_string(),
                    payment_id: payment_data.payment_intent.payment_id.clone(),
                    merchant_id: payment_data.merchant_account.merchant_id.clone(),
                    attempt_id: payment_data.payment_attempt.attempt_id.clone(),
                    created_at: common_utils::date_time::now(),
                    frm_name: frm_connector_details.connector_name,
                    frm_transaction_id: None,
                    frm_transaction_type: FraudCheckType::PreFrm,
                    frm_status: FraudCheckStatus::Pending,
                    frm_score: None,
                    frm_reason: None,
                    frm_error: None,
                    payment_details,
                    metadata: None,
                    modified_at: common_utils::date_time::now(),
                    last_step: FraudCheckLastStep::Processing,
                })
                .await
            }
        };

        match fraud_check {
            Ok(fraud_check_value) => {
                let frm_data = FrmData {
                    payment_intent: payment_data.payment_intent,
                    payment_attempt: payment_data.payment_attempt,
                    merchant_account: payment_data.merchant_account,
                    address: payment_data.address,
                    fraud_check: fraud_check_value,
                    connector_details: payment_data.connector_details,
                    order_details: payment_data.order_details,
                    refund: None,
                    frm_metadata: payment_data.frm_metadata,
                };
                Ok(Some(frm_data))
            }
            Err(error) => {
                router_env::logger::error!("inserting into fraud_check table failed {error:?}");
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl<F: Send + Clone> Domain<F> for FraudCheckPre {
    #[instrument(skip_all)]
    async fn post_payment_frm<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut payments::PaymentData<F>,
        frm_data: &mut FrmData,
        merchant_account: &domain::MerchantAccount,
        customer: &Option<domain::Customer>,
        key_store: domain::MerchantKeyStore,
    ) -> RouterResult<Option<FrmRouterData>> {
        let router_data = frm_core::call_frm_service::<F, frm_api::Transaction, _>(
            state,
            payment_data,
            &mut frm_data.to_owned(),
            merchant_account,
            &key_store,
            customer,
        )
        .await?;
        frm_data.fraud_check.last_step = FraudCheckLastStep::TransactionOrRecordRefund;
        Ok(Some(FrmRouterData {
            merchant_id: router_data.merchant_id,
            connector: router_data.connector,
            payment_id: router_data.payment_id,
            attempt_id: router_data.attempt_id,
            request: FrmRequest::Transaction(FraudCheckTransactionData {
                amount: router_data.request.amount,
                order_details: router_data.request.order_details,
                currency: router_data.request.currency,
                payment_method: Some(router_data.payment_method),
                error_code: router_data.request.error_code,
                error_message: router_data.request.error_message,
                connector_transaction_id: router_data.request.connector_transaction_id,
            }),
            response: FrmResponse::Transaction(router_data.response),
        }))
    }

    async fn pre_payment_frm<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut payments::PaymentData<F>,
        frm_data: &mut FrmData,
        merchant_account: &domain::MerchantAccount,
        customer: &Option<domain::Customer>,
        key_store: domain::MerchantKeyStore,
    ) -> RouterResult<FrmRouterData> {
        let router_data = frm_core::call_frm_service::<F, frm_api::Checkout, _>(
            state,
            payment_data,
            &mut frm_data.to_owned(),
            merchant_account,
            &key_store,
            customer,
        )
        .await?;
        frm_data.fraud_check.last_step = FraudCheckLastStep::CheckoutOrSale;
        Ok(FrmRouterData {
            merchant_id: router_data.merchant_id,
            connector: router_data.connector,
            payment_id: router_data.payment_id,
            attempt_id: router_data.attempt_id,
            request: FrmRequest::Checkout(FraudCheckCheckoutData {
                amount: router_data.request.amount,
                order_details: router_data.request.order_details,
                currency: router_data.request.currency,
                browser_info: router_data.request.browser_info,
                payment_method_data: router_data.request.payment_method_data,
                email: router_data.request.email,
                gateway: router_data.request.gateway,
            }),
            response: FrmResponse::Checkout(router_data.response),
        })
    }
}

#[async_trait]
impl<F: Clone + Send> UpdateTracker<FrmData, F> for FraudCheckPre {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut frm_data: FrmData,
        payment_data: &mut payments::PaymentData<F>,
        _frm_suggestion: Option<FrmSuggestion>,
        frm_router_data: FrmRouterData,
    ) -> RouterResult<FrmData> {
        let frm_check_update = match frm_router_data.response {
            FrmResponse::Checkout(response) => match response {
                Err(err) => Some(FraudCheckUpdate::ErrorUpdate {
                    status: FraudCheckStatus::TransactionFailure,
                    error_message: Some(Some(err.message)),
                }),
                Ok(payments_response) => match payments_response {
                    FraudCheckResponseData::TransactionResponse {
                        resource_id,
                        connector_metadata,
                        status,
                        reason,
                        score,
                    } => {
                        let connector_transaction_id = match resource_id {
                            ResponseId::NoResponseId => None,
                            ResponseId::ConnectorTransactionId(id) => Some(id),
                            ResponseId::EncodedData(id) => Some(id),
                        };

                        let fraud_check_update = FraudCheckUpdate::ResponseUpdate {
                            frm_status: status,
                            frm_transaction_id: connector_transaction_id,
                            frm_reason: reason,
                            frm_score: score,
                            metadata: connector_metadata,
                            modified_at: common_utils::date_time::now(),
                            last_step: frm_data.fraud_check.last_step,
                        };
                        Some(fraud_check_update)
                    }
                    FraudCheckResponseData::FulfillmentResponse {
                        order_id: _,
                        shipment_ids: _,
                    } => None,
                    FraudCheckResponseData::RecordReturnResponse {
                        resource_id: _,
                        connector_metadata: _,
                        return_id: _,
                    } => Some(FraudCheckUpdate::ErrorUpdate {
                        status: FraudCheckStatus::TransactionFailure,
                        error_message: Some(Some(
                            "Error: Got Record Return Response response in current Checkout flow"
                                .to_string(),
                        )),
                    }),
                },
            },
            FrmResponse::Transaction(response) => match response {
                Err(err) => Some(FraudCheckUpdate::ErrorUpdate {
                    status: FraudCheckStatus::TransactionFailure,
                    error_message: Some(Some(err.message)),
                }),
                Ok(payments_response) => match payments_response {
                    FraudCheckResponseData::TransactionResponse {
                        resource_id,
                        connector_metadata,
                        status,
                        reason,
                        score,
                    } => {
                        let connector_transaction_id = match resource_id {
                            ResponseId::NoResponseId => None,
                            ResponseId::ConnectorTransactionId(id) => Some(id),
                            ResponseId::EncodedData(id) => Some(id),
                        };

                        let frm_status = payment_data
                            .frm_message
                            .as_ref()
                            .map_or(status, |frm_data| frm_data.frm_status);

                        let fraud_check_update = FraudCheckUpdate::ResponseUpdate {
                            frm_status,
                            frm_transaction_id: connector_transaction_id,
                            frm_reason: reason,
                            frm_score: score,
                            metadata: connector_metadata,
                            modified_at: common_utils::date_time::now(),
                            last_step: frm_data.fraud_check.last_step,
                        };
                        Some(fraud_check_update)
                    }
                    FraudCheckResponseData::FulfillmentResponse {
                        order_id: _,
                        shipment_ids: _,
                    } => None,
                    FraudCheckResponseData::RecordReturnResponse {
                        resource_id: _,
                        connector_metadata: _,
                        return_id: _,
                    } => Some(FraudCheckUpdate::ErrorUpdate {
                        status: FraudCheckStatus::TransactionFailure,
                        error_message: Some(Some(
                            "Error: Got Record Return Response response in current Checkout flow"
                                .to_string(),
                        )),
                    }),
                },
            },
            FrmResponse::Sale(_response)
            | FrmResponse::Fulfillment(_response)
            | FrmResponse::RecordReturn(_response) => Some(FraudCheckUpdate::ErrorUpdate {
                status: FraudCheckStatus::TransactionFailure,
                error_message: Some(Some(
                    "Error: Got Pre(Sale) flow response in current post flow".to_string(),
                )),
            }),
        };

        frm_data.fraud_check = match frm_check_update {
            Some(fraud_check_update) => db
                .update_fraud_check_response_with_attempt_id(
                    frm_data.clone().fraud_check,
                    fraud_check_update,
                )
                .await
                .map_err(|error| error.change_context(errors::ApiErrorResponse::PaymentNotFound))?,
            None => frm_data.clone().fraud_check,
        };

        Ok(frm_data)
    }
}
