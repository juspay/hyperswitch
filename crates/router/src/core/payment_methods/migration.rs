use actix_multipart::form::{self, bytes, text};
use api_models::payment_methods as pm_api;
use common_utils::{errors::CustomResult, id_type};
use csv::Reader;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, errors::api_error_response as errors,
    payment_methods::PaymentMethodUpdate, platform,
};
use masking::{ExposeInterface, PeekInterface};
use payment_methods::core::migration::MerchantConnectorValidator;
use rdkafka::message::ToBytes;
use router_env::logger;

use crate::{
    core::{errors::StorageErrorExt, payment_methods::cards::create_encrypted_data},
    routes::SessionState,
};
type PmMigrationResult<T> = CustomResult<ApplicationResponse<T>, errors::ApiErrorResponse>;

#[cfg(feature = "v1")]
pub async fn update_payment_methods(
    state: &SessionState,
    payment_methods: Vec<pm_api::UpdatePaymentMethodRecord>,
    merchant_id: &id_type::MerchantId,
    platform: &platform::Platform,
) -> PmMigrationResult<Vec<pm_api::PaymentMethodUpdateResponse>> {
    let mut result = Vec::with_capacity(payment_methods.len());

    for record in payment_methods {
        let update_res =
            update_payment_method_record(state, record.clone(), merchant_id, platform).await;
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
    platform: &platform::Platform,
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
    let key_manager_state = state.into();
    let mut updated_card_expiry = false;

    let payment_method = db
        .find_payment_method(
            platform.get_processor().get_key_store(),
            &payment_method_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if payment_method.merchant_id != *merchant_id {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Merchant ID in the request does not match the Merchant ID in the payment method record.".to_string(),
                }.into());
    }

    let updated_payment_method_data = match payment_method.payment_method_data.as_ref() {
        Some(data) => {
            match serde_json::from_value::<pm_api::PaymentMethodsData>(
                data.clone().into_inner().expose(),
            ) {
                Ok(pm_api::PaymentMethodsData::Card(mut card_data)) => {
                    if let Some(new_month) = &req.card_expiry_month {
                        card_data.expiry_month = Some(new_month.clone());
                        updated_card_expiry = true;
                    }

                    if let Some(new_year) = &req.card_expiry_year {
                        card_data.expiry_year = Some(new_year.clone());
                        updated_card_expiry = true;
                    }

                    if updated_card_expiry {
                        Some(
                            create_encrypted_data(
                                &key_manager_state,
                                platform.get_processor().get_key_store(),
                                pm_api::PaymentMethodsData::Card(card_data),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt payment method data")
                            .map(Into::into),
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        None => None,
    }
    .transpose()?;

    let mca_data_cache = if let Some(merchant_connector_ids) = &req.merchant_connector_ids {
        let parsed_mca_ids =
            MerchantConnectorValidator::parse_comma_separated_ids(merchant_connector_ids)?;
        let mut cache = HashMap::new();

        for merchant_connector_id in parsed_mca_ids {
            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    platform.get_processor().get_account().get_id(),
                    &merchant_connector_id,
                    platform.get_processor().get_key_store(),
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            cache.insert(merchant_connector_id, mca);
        }
        Some(cache)
    } else {
        None
    };

    let pm_update = match (
        &req.payment_instrument_id,
        &req.connector_customer_id,
        &mca_data_cache,
    ) {
        // Case: payment_instrument_id (with or without connector_customer_id)
        (Some(payment_instrument_id), _, Some(cache)) => {
            let mandate_details = payment_method
                .get_common_mandate_reference()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;

            let mut existing_payments_mandate = mandate_details
                .payments
                .clone()
                .unwrap_or(PaymentsMandateReference(HashMap::new()));
            let mut existing_payouts_mandate = mandate_details
                .payouts
                .clone()
                .unwrap_or(PayoutsMandateReference(HashMap::new()));

            for (merchant_connector_id, mca) in cache.iter() {
                match mca.connector_type {
                    enums::ConnectorType::PayoutProcessor => {
                        let new_payout_record = PayoutsMandateReferenceRecord {
                            transfer_method_id: Some(payment_instrument_id.peek().to_string()),
                            connector_customer_id: req.connector_customer_id.clone(),
                        };

                        if let Some(existing_record) =
                            existing_payouts_mandate.0.get_mut(merchant_connector_id)
                        {
                            if let Some(transfer_method_id) = &new_payout_record.transfer_method_id
                            {
                                existing_record.transfer_method_id =
                                    Some(transfer_method_id.clone());
                            }
                            // Update connector_customer_id if provided
                            if req.connector_customer_id.is_some() {
                                existing_record.connector_customer_id =
                                    req.connector_customer_id.clone();
                            }
                        } else {
                            existing_payouts_mandate
                                .0
                                .insert(merchant_connector_id.clone(), new_payout_record);
                        }
                    }
                    _ => {
                        if let Some(existing_record) =
                            existing_payments_mandate.0.get_mut(merchant_connector_id)
                        {
                            existing_record.connector_mandate_id =
                                payment_instrument_id.peek().to_string();
                            // Update connector_customer_id if provided
                            if req.connector_customer_id.is_some() {
                                existing_record.connector_customer_id =
                                    req.connector_customer_id.clone();
                            }
                        } else {
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
                                    connector_customer_id: req.connector_customer_id.clone(),
                                },
                            );
                        }
                    }
                }
            }

            let updated_connector_mandate_details = CommonMandateReference {
                payments: if !existing_payments_mandate.0.is_empty() {
                    Some(existing_payments_mandate)
                } else {
                    mandate_details.payments
                },
                payouts: if !existing_payouts_mandate.0.is_empty() {
                    Some(existing_payouts_mandate)
                } else {
                    mandate_details.payouts
                },
            };

            let connector_mandate_details_value = updated_connector_mandate_details
                .get_mandate_details_value()
                .map_err(|err| {
                    logger::error!("Failed to get get_mandate_details_value : {:?}", err);
                    errors::ApiErrorResponse::MandateUpdateFailed
                })?;

            PaymentMethodUpdate::PaymentMethodBatchUpdate {
                connector_mandate_details: Some(pii::SecretSerdeValue::new(
                    connector_mandate_details_value,
                )),
                network_transaction_id,
                status,
                payment_method_data: updated_payment_method_data.clone(),
                last_modified_by: None,
            }
        }
        // Case: Only connector_customer_id provided
        (None, Some(connector_customer_id), Some(cache)) => {
            let mandate_details = payment_method
                .get_common_mandate_reference()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;

            // Error if no mandate details exist
            if mandate_details.payments.is_none() && mandate_details.payouts.is_none() {
                return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "connector_mandate_details does not exist".to_string(),
                }
                .into());
            }

            let mut existing_payments_mandate = mandate_details
                .payments
                .clone()
                .unwrap_or(PaymentsMandateReference(HashMap::new()));
            let mut existing_payouts_mandate = mandate_details
                .payouts
                .clone()
                .unwrap_or(PayoutsMandateReference(HashMap::new()));

            // For each merchant_connector_id, check appropriate reference based on connector_type
            for (merchant_connector_id, mca) in cache.iter() {
                match mca.connector_type {
                    enums::ConnectorType::PayoutProcessor => {
                        // Check if exists in payouts mandate
                        if let Some(existing_record) =
                            existing_payouts_mandate.0.get_mut(merchant_connector_id)
                        {
                            existing_record.connector_customer_id =
                                Some(connector_customer_id.clone());
                        } else {
                            return Err(errors::ApiErrorResponse::InvalidRequestData {
                                message: format!("merchant_connector_id {} does not exist in payouts connector_mandate_details", merchant_connector_id.get_string_repr()),
                            }.into());
                        }
                    }
                    _ => {
                        // Check if exists in payments mandate
                        if let Some(existing_record) =
                            existing_payments_mandate.0.get_mut(merchant_connector_id)
                        {
                            existing_record.connector_customer_id =
                                Some(connector_customer_id.clone());
                        } else {
                            return Err(errors::ApiErrorResponse::InvalidRequestData {
                                message: format!("merchant_connector_id {} does not exist in payments connector_mandate_details", merchant_connector_id.get_string_repr()),
                            }.into());
                        }
                    }
                }
            }

            let updated_connector_mandate_details = CommonMandateReference {
                payments: if !existing_payments_mandate.0.is_empty() {
                    Some(existing_payments_mandate)
                } else {
                    mandate_details.payments
                },
                payouts: if !existing_payouts_mandate.0.is_empty() {
                    Some(existing_payouts_mandate)
                } else {
                    mandate_details.payouts
                },
            };

            let connector_mandate_details_value = updated_connector_mandate_details
                .get_mandate_details_value()
                .map_err(|err| {
                    logger::error!("Failed to get get_mandate_details_value : {:?}", err);
                    errors::ApiErrorResponse::MandateUpdateFailed
                })?;

            PaymentMethodUpdate::PaymentMethodBatchUpdate {
                connector_mandate_details: Some(pii::SecretSerdeValue::new(
                    connector_mandate_details_value,
                )),
                network_transaction_id,
                status,
                payment_method_data: updated_payment_method_data.clone(),
                last_modified_by: None,
            }
        }
        _ => {
            if updated_payment_method_data.is_some() {
                PaymentMethodUpdate::PaymentMethodDataUpdate {
                    payment_method_data: updated_payment_method_data,
                    last_modified_by: None,
                }
            } else {
                PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                    network_transaction_id,
                    status,
                    last_modified_by: None,
                }
            }
        }
    };

    let response = db
        .update_payment_method(
            platform.get_processor().get_key_store(),
            payment_method,
            pm_update,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to update payment method for existing pm_id: {payment_method_id:?} in db",
        ))?;

    Ok(ApplicationResponse::Json(
        pm_api::PaymentMethodRecordUpdateResponse {
            payment_method_id: response.payment_method_id,
            status: response.status,
            network_transaction_id: response.network_transaction_id,
            connector_mandate_details: response
                .connector_mandate_details
                .map(pii::SecretSerdeValue::new),
            updated_payment_method_data: Some(updated_card_expiry),
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
