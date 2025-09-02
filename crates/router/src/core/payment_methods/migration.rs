use actix_multipart::form::{self, bytes, text};
use api_models::payment_methods as pm_api;
use common_utils::{errors::CustomResult, id_type};
use csv::Reader;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, errors::api_error_response as errors, merchant_context,
    payment_methods::PaymentMethodUpdate,
};
use masking::PeekInterface;
use rdkafka::message::ToBytes;
use router_env::logger;

use crate::{core::errors::StorageErrorExt, routes::SessionState};

type PmMigrationResult<T> = CustomResult<ApplicationResponse<T>, errors::ApiErrorResponse>;

#[cfg(feature = "v1")]
pub async fn update_payment_methods(
    state: &SessionState,
    payment_methods: Vec<pm_api::UpdatePaymentMethodRecord>,
    merchant_id: &id_type::MerchantId,
    merchant_context: &merchant_context::MerchantContext,
) -> PmMigrationResult<Vec<pm_api::PaymentMethodUpdateResponse>> {
    let mut result = Vec::with_capacity(payment_methods.len());

    for record in payment_methods {
        let update_res =
            update_payment_method_record(state, record.clone(), merchant_id, merchant_context)
                .await;
        let res = match update_res {
            Ok(ApplicationResponse::Json(response)) => Ok(response),
            Err(e) => Err(e.to_string()),
            _ => Err("Failed to update payment method".to_string()),
        };

        result.push(pm_api::PaymentMethodUpdateResponse::from((res, record)));
    }
    Ok(ApplicationResponse::Json(result))
}

#[cfg(feature = "v1")]
pub async fn update_payment_method_record(
    state: &SessionState,
    req: pm_api::UpdatePaymentMethodRecord,
    merchant_id: &id_type::MerchantId,
    merchant_context: &merchant_context::MerchantContext,
) -> CustomResult<
    ApplicationResponse<pm_api::PaymentMethodRecordUpdateResponse>,
    errors::ApiErrorResponse,
> {
    use std::collections::HashMap;

    use common_enums::enums;
    use common_utils::pii;
    use hyperswitch_domain_models::mandates::{
        CommonMandateReference, PaymentsMandateReference, PaymentsMandateReferenceRecord,
        PayoutsMandateReference, PayoutsMandateReferenceRecord,
    };

    let db = &*state.store;
    let payment_method_id = req.payment_method_id.clone();
    let network_transaction_id = req.network_transaction_id.clone();
    let status = req.status;

    let payment_method = db
        .find_payment_method(
            &state.into(),
            merchant_context.get_merchant_key_store(),
            &payment_method_id,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if payment_method.merchant_id != *merchant_id {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Merchant ID in the request does not match the Merchant ID in the payment method record.".to_string(),
                }.into());
    }

    // Process mandate details when both payment_instrument_id and merchant_connector_id are present
    let pm_update = match (&req.payment_instrument_id, &req.merchant_connector_id) {
        (Some(payment_instrument_id), Some(merchant_connector_id)) => {
            let mandate_details = payment_method
                .get_common_mandate_reference()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;

            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    &state.into(),
                    merchant_context.get_merchant_account().get_id(),
                    merchant_connector_id,
                    merchant_context.get_merchant_key_store(),
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            let updated_connector_mandate_details = match mca.connector_type {
                enums::ConnectorType::PayoutProcessor => {
                    // Handle PayoutsMandateReference
                    let mut existing_payouts_mandate = mandate_details
                        .payouts
                        .unwrap_or_else(|| PayoutsMandateReference(HashMap::new()));

                    // Create new payout mandate record
                    let new_payout_record = PayoutsMandateReferenceRecord {
                        transfer_method_id: Some(payment_instrument_id.peek().to_string()),
                    };

                    // Check if record exists for this merchant_connector_id
                    if let Some(existing_record) =
                        existing_payouts_mandate.0.get_mut(merchant_connector_id)
                    {
                        if let Some(transfer_method_id) = &new_payout_record.transfer_method_id {
                            existing_record.transfer_method_id = Some(transfer_method_id.clone());
                        }
                    } else {
                        // Insert new record in connector_mandate_details
                        existing_payouts_mandate
                            .0
                            .insert(merchant_connector_id.clone(), new_payout_record);
                    }

                    // Create updated CommonMandateReference preserving payments section
                    CommonMandateReference {
                        payments: mandate_details.payments,
                        payouts: Some(existing_payouts_mandate),
                    }
                }
                _ => {
                    // Handle PaymentsMandateReference
                    let mut existing_payments_mandate = mandate_details
                        .payments
                        .unwrap_or_else(|| PaymentsMandateReference(HashMap::new()));

                    // Check if record exists for this merchant_connector_id
                    if let Some(existing_record) =
                        existing_payments_mandate.0.get_mut(merchant_connector_id)
                    {
                        existing_record.connector_mandate_id =
                            payment_instrument_id.peek().to_string();
                    } else {
                        // Insert new record in connector_mandate_details
                        existing_payments_mandate.0.insert(
                            merchant_connector_id.clone(),
                            PaymentsMandateReferenceRecord {
                                connector_mandate_id: payment_instrument_id.peek().to_string(),
                                payment_method_type: None,
                                original_payment_authorized_amount: None,
                                original_payment_authorized_currency: None,
                                mandate_metadata: None,
                                connector_mandate_status: None,
                                connector_mandate_request_reference_id: None,
                            },
                        );
                    }

                    // Create updated CommonMandateReference preserving payouts section
                    CommonMandateReference {
                        payments: Some(existing_payments_mandate),
                        payouts: mandate_details.payouts,
                    }
                }
            };

            let connector_mandate_details_value = updated_connector_mandate_details
                .get_mandate_details_value()
                .map_err(|err| {
                    logger::error!("Failed to get get_mandate_details_value : {:?}", err);
                    errors::ApiErrorResponse::MandateUpdateFailed
                })?;

            PaymentMethodUpdate::ConnectorNetworkTransactionIdStatusAndMandateDetailsUpdate {
                connector_mandate_details: Some(pii::SecretSerdeValue::new(
                    connector_mandate_details_value,
                )),
                network_transaction_id,
                status,
            }
        }
        _ => {
            // Update only network_transaction_id and status
            PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status,
            }
        }
    };

    let response = db
        .update_payment_method(
            &state.into(),
            merchant_context.get_merchant_key_store(),
            payment_method,
            pm_update,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to update payment method for existing pm_id: {payment_method_id:?} in db",
        ))?;

    logger::debug!("Payment method updated in db");

    Ok(ApplicationResponse::Json(
        pm_api::PaymentMethodRecordUpdateResponse {
            payment_method_id: response.payment_method_id,
            status: response.status,
            network_transaction_id: response.network_transaction_id,
            connector_mandate_details: response
                .connector_mandate_details
                .map(pii::SecretSerdeValue::new),
        },
    ))
}

#[derive(Debug, form::MultipartForm)]
pub struct PaymentMethodsUpdateForm {
    #[multipart(limit = "1MB")]
    pub file: bytes::Bytes,

    pub merchant_id: text::Text<id_type::MerchantId>,
}

fn parse_update_csv(data: &[u8]) -> csv::Result<Vec<pm_api::UpdatePaymentMethodRecord>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for result in csv_reader.deserialize() {
        let mut record: pm_api::UpdatePaymentMethodRecord = result?;
        id_counter += 1;
        record.line_number = Some(id_counter);
        records.push(record);
    }
    Ok(records)
}

type UpdateValidationResult =
    Result<(id_type::MerchantId, Vec<pm_api::UpdatePaymentMethodRecord>), errors::ApiErrorResponse>;

impl PaymentMethodsUpdateForm {
    pub fn validate_and_get_payment_method_records(self) -> UpdateValidationResult {
        let records = parse_update_csv(self.file.data.to_bytes()).map_err(|e| {
            errors::ApiErrorResponse::PreconditionFailed {
                message: e.to_string(),
            }
        })?;
        Ok((self.merchant_id.clone(), records))
    }
}
