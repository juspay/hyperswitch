use api_models::payment_method_billing_address_migration;
use common_utils::{crypto::Encryptable, errors::CustomResult, ext_traits::AsyncExt};
use diesel_models::PaymentMethodUpdate;
use error_stack::ResultExt;
use hyperswitch_domain_models::address::Address;
use masking::Secret;

use super::{errors, payment_methods::cards::create_encrypted_data};
use crate::{routes::SessionState, services, types::domain};

pub async fn payment_method_billing_address_migration(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_method_id: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<
    services::ApplicationResponse<
        payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse,
    >,
    errors::ApiErrorResponse,
> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let payment_method = db
        .find_payment_method(
            key_manager_state,
            key_store,
            payment_method_id,
            merchant_account.storage_scheme,
        )
        .await;

    let (existing_payment_method,last_payment_attempt, payment_intent) = match payment_method {
        Ok(res) => {
            let last_payment_attempt = db.find_last_successful_attempt_by_payment_method_id_merchant_id_where_billing_address_is_present(
                &res.payment_method_id,
                merchant_id,
                merchant_account.storage_scheme,
            ).await;

            match last_payment_attempt {
                Ok(last_payment_attempt) => {
                    let payment_intent = db.find_payment_intent_by_payment_id_merchant_id(
                        key_manager_state,
                        &last_payment_attempt.payment_id,
                        merchant_id,
                        key_store,
                        merchant_account.storage_scheme,
                    ).await;
                    match payment_intent {
                        Ok(payment_intent) => {
                            (res, last_payment_attempt, payment_intent)
                        }
                        Err(_) => {
                            return Ok(services::api::ApplicationResponse::Json(
                                payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse {
                                    payment_method_id: payment_method_id.to_string(),
                                    merchant_id: merchant_id.clone(),
                                    migration_successful: payment_method_billing_address_migration::MigrationStatus::Failed,
                                    failure_reason: Some("No payment intent found for corresponding payment attempt".to_string()),
                                },
                            ))
                        }
                    }
                }
                Err(_) => {
                    return Ok(services::api::ApplicationResponse::Json(
                        payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse {
                            payment_method_id: payment_method_id.to_string(),
                            merchant_id: merchant_id.clone(),
                            migration_successful: payment_method_billing_address_migration::MigrationStatus::Failed,
                            failure_reason: Some("No successful payment attempt found where billing addresss is present".to_string()),
                        },
                    ))
                }

            }
        }
        Err(_) => {
            return Ok(services::api::ApplicationResponse::Json(
                payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse {
                    payment_method_id: payment_method_id.to_string(),
                    merchant_id: merchant_id.clone(),
                    migration_successful: payment_method_billing_address_migration::MigrationStatus::Failed,
                    failure_reason: Some("Payment method not found".to_string()),
                },
            ))
        }
    };

    let payment_attempt_billing_address = last_payment_attempt
        .payment_method_billing_address_id
        .as_ref()
        .async_map(|address_id| {
            db.find_address_by_address_id(key_manager_state, address_id, key_store)
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to find addresss by address_id")?;

    let payment_intent_billing_address = payment_intent
        .billing_address_id
        .as_ref()
        .async_map(|address_id| {
            db.find_address_by_address_id(key_manager_state, address_id, key_store)
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to find addresss by address_id")?;

    let payment_attempt_domain_billing_address: Option<Address> =
        payment_attempt_billing_address.as_ref().map(From::from);
    let payment_intent_domain_billing_address: Option<Address> =
        payment_intent_billing_address.as_ref().map(From::from);

    let unified_billing_address = payment_attempt_domain_billing_address.map(|billing_address| {
        billing_address.unify_address(payment_intent_domain_billing_address.as_ref())
    });

    let encrypted_unified_billing_address: Option<Encryptable<Secret<serde_json::Value>>> =
        unified_billing_address
            .async_map(|address| {
                create_encrypted_data(key_manager_state, key_store, address.clone())
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt payment method billing address")?;

    let payment_method_update = PaymentMethodUpdate::PaymentMethodBillingAddressUpdate {
        payment_method_billing_address: encrypted_unified_billing_address
            .map(|address| address.into()),
    };

    let update_billing_address = db
        .update_payment_method(
            key_manager_state,
            key_store,
            existing_payment_method,
            payment_method_update,
            merchant_account.storage_scheme,
        )
        .await;

    match update_billing_address {
        Ok(billing_address) => {
            Ok(services::api::ApplicationResponse::Json(
                payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse {
                    payment_method_id: billing_address.payment_method_id,
                    merchant_id: merchant_id.clone(),
                    migration_successful: payment_method_billing_address_migration::MigrationStatus::Success,
                    failure_reason: None,
                },
            ))
        }
        Err(_) => {
            Ok(services::api::ApplicationResponse::Json(
                payment_method_billing_address_migration::PaymentMethodBillingAddressMigrationResponse {
                    payment_method_id: payment_method_id.to_string(),
                    merchant_id: merchant_id.clone(),
                    migration_successful: payment_method_billing_address_migration::MigrationStatus::Failed,
                    failure_reason: Some("Unable to Update Payment Method Table".to_string()),
                },
            ))
        }
    }
}
