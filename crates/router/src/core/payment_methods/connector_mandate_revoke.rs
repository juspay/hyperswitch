//! Payment-method-scoped connector mandate revoke (v1).
//!
//! `POST /payment_methods/{payment_method_id}/connector_tokens/revoke`
//!
//! Revokes connector-level mandates stored in
//! `payment_methods.connector_mandate_details.payments[<merchant_connector_account_id>]` by calling
//! each connector's `MandateRevoke` flow. Eligible entries are first moved to
//! `MandateRevocationInitiated`, then become `Inactive` after connector confirmation. Known
//! connector failures return to `Active`; failures with an unknown connector outcome remain
//! `MandateRevocationInitiated`. The `mandates` table is intentionally untouched (use
//! `POST /mandates/revoke/{id}` for table-backed mandates).
#![cfg(feature = "v1")]

use std::collections::{HashMap, HashSet};

use api_models::{
    enums as api_enums,
    payment_methods::{
        ConnectorMandateRevokeRequest, ConnectorMandateRevokeResponse, ConnectorMandateRevokeResult,
    },
};
use common_utils::{errors::ErrorSwitch, id_type};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::mandates::CommonMandateReference;
use router_env::{instrument, logger};

use super::cards;
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        mandate::utils as mandate_utils,
        payments::{self, access_token, gateway::context as gateway_context, helpers},
    },
    routes::SessionState,
    services::{self, ConnectorSpecifications},
    types::{
        self,
        api::{ConnectorData, GetToken},
        domain,
    },
};

/// Request-level validations and the per-item revoke pipeline
/// Always returns HTTP 200 when the request itself is valid; per-merchant-connector outcomes are
/// reported in `results`. Any failure after the per-item loop starts is converted to an `error`
/// item, never propagated with `?`.
#[instrument(skip_all, fields(flow = "payment_methods_connector_mandate_revoke"))]
pub async fn revoke_connector_mandates(
    state: SessionState,
    platform: domain::Platform,
    profile_id: id_type::ProfileId,
    payment_method_id: String,
    req: ConnectorMandateRevokeRequest,
) -> RouterResponse<ConnectorMandateRevokeResponse> {
    let db = state.store.as_ref();
    let key_store = platform.get_processor().get_key_store();
    let merchant_id = platform.get_processor().get_account().get_id();
    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    if req.merchant_connector_ids.is_empty() {
        return Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "merchant_connector_ids".into(),
        }
        .into());
    }

    // The merchant-scoped profile fetch is the authorization check for `X-Profile-Id`
    let profile = state
        .accounts_store
        .find_business_profile_by_merchant_id_profile_id(key_store, merchant_id, &profile_id)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)
        .attach_printable("X-Profile-Id does not name a profile belonging to this merchant")?;

    let payment_method = db
        .find_payment_method(key_store, &payment_method_id, storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let merchant_connector_accounts = state
        .accounts_store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to fetch merchant connector accounts")?
        .into_iter()
        .map(|merchant_connector_account| {
            (
                merchant_connector_account.get_id(),
                merchant_connector_account,
            )
        })
        .collect::<HashMap<_, _>>();

    if payment_method.merchant_id != *merchant_id {
        return Err(errors::ApiErrorResponse::PaymentMethodNotFound.into());
    }

    if payment_method.status == storage_enums::PaymentMethodStatus::Redacted {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "payment method is deleted".to_string(),
        }
        .into());
    }

    let mandate_reference = payment_method
        .get_common_mandate_reference()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to parse connector mandate details")?;

    // Dedupe merchant_connector_ids, keeping the first occurrence (results follow request order)
    let mut seen = HashSet::new();
    let merchant_connector_ids = req
        .merchant_connector_ids
        .into_iter()
        .filter(|mca_id| seen.insert(mca_id.clone()))
        .collect::<Vec<_>>();

    // Prepare every requested mandate first, then stage every eligible mandate before issuing a
    // connector request. This makes an MIT observe `MandateRevocationInitiated` for every
    // eligible item in the batch, rather than only for the item currently being revoked.
    let mut results = Vec::with_capacity(merchant_connector_ids.len());
    let mut prepared_mandates = Vec::with_capacity(merchant_connector_ids.len());
    for (index, mca_id) in merchant_connector_ids.into_iter().enumerate() {
        match prepare_connector_mandate_revoke(
            &state,
            &profile,
            &payment_method,
            &mandate_reference,
            &merchant_connector_accounts,
            mca_id.clone(),
        ) {
            Ok(prepared_mandate) => prepared_mandates.push((index, prepared_mandate)),
            Err(error) => {
                results.push((index, connector_mandate_revoke_error_item(&mca_id, error)))
            }
        }
    }

    let mut staged_mandate_reference = mandate_reference.clone();
    let stage_status_updates = prepared_mandates
        .iter()
        .map(|(_, prepared_mandate)| {
            (
                prepared_mandate.mca_id.clone(),
                api_enums::ConnectorMandateStatus::MandateRevocationInitiated,
            )
        })
        .collect::<Vec<_>>();

    // Stage every eligible entry with one payment-method update before calling any connector.
    let staged_mandates = if stage_status_updates.is_empty() {
        Vec::new()
    } else {
        match persist_connector_mandate_statuses(
            &state,
            &platform,
            &payment_method,
            &mut staged_mandate_reference,
            &stage_status_updates,
        )
        .await
        {
            Ok(()) => prepared_mandates,
            Err(error) => {
                logger::error!(?error, "failed to stage connector mandate revocations");
                for (index, prepared_mandate) in prepared_mandates {
                    results.push((
                        index,
                        connector_mandate_revoke_error_item(
                            &prepared_mandate.mca_id,
                            error_stack::report!(errors::ApiErrorResponse::MandateUpdateFailed),
                        ),
                    ));
                }
                Vec::new()
            }
        }
    };

    let mut final_status_updates = Vec::with_capacity(staged_mandates.len());
    let mut completed_mca_ids = Vec::new();
    for (index, prepared_mandate) in staged_mandates {
        let mca_id = prepared_mandate.mca_id.clone();
        let result = match revoke_connector_mandate_for_mca(
            &state,
            &platform,
            &profile,
            &payment_method,
            prepared_mandate,
        )
        .await
        {
            Ok(item) => {
                completed_mca_ids.push((index, mca_id.clone()));
                final_status_updates.push((mca_id, api_enums::ConnectorMandateStatus::Inactive));
                item
            }
            Err(error) => {
                if should_restore_connector_mandate_after_revoke_error(&error) {
                    final_status_updates
                        .push((mca_id.clone(), api_enums::ConnectorMandateStatus::Active));
                } else {
                    // TODO: add a mandate sync flow to reconcile connector revoke 5xx outcomes.
                }
                connector_mandate_revoke_error_item(&mca_id, error)
            }
        };
        results.push((index, result));
    }

    if !final_status_updates.is_empty() {
        if let Err(error) = persist_connector_mandate_statuses(
            &state,
            &platform,
            &payment_method,
            &mut staged_mandate_reference,
            &final_status_updates,
        )
        .await
        {
            logger::error!(
                ?error,
                "failed to persist confirmed connector mandate revocations"
            );
            for (index, mca_id) in completed_mca_ids {
                if let Some((_, result)) = results
                    .iter_mut()
                    .find(|(result_index, _)| *result_index == index)
                {
                    *result = connector_mandate_revoke_error_item(
                        &mca_id,
                        error_stack::report!(errors::ApiErrorResponse::MandateUpdateFailed),
                    );
                }
            }
        }
    }

    results.sort_unstable_by_key(|(index, _)| *index);
    let results = results.into_iter().map(|(_, result)| result).collect();

    Ok(services::ApplicationResponse::Json(
        ConnectorMandateRevokeResponse {
            payment_method_id,
            profile_id,
            results,
        },
    ))
}

/// Builds a per-item `error` result from any failure in the per-item pipeline.
fn connector_mandate_revoke_error_item(
    mca_id: &id_type::MerchantConnectorAccountId,
    error: error_stack::Report<errors::ApiErrorResponse>,
) -> ConnectorMandateRevokeResult {
    logger::error!(
        merchant_connector_id = mca_id.get_string_repr(),
        ?error,
        "connector mandate revoke failed for merchant_connector_id"
    );
    let api_error: api_models::errors::types::ApiErrorResponse = error.current_context().switch();
    ConnectorMandateRevokeResult {
        merchant_connector_id: mca_id.clone(),
        connector: None,
        connector_mandate_id: None,
        connector_mandate_status: None,
        error: Some(api_models::errors::types::ErrorResponse::from(&api_error)),
    }
}

fn should_restore_connector_mandate_after_revoke_error(
    error: &error_stack::Report<errors::ApiErrorResponse>,
) -> bool {
    let has_unknown_connector_outcome = matches!(
        error.current_context(),
        errors::ApiErrorResponse::ExternalConnectorError {
            status_code: 500..=599,
            ..
        }
    ) || matches!(
        error.downcast_ref::<errors::ConnectorError>(),
        Some(errors::ConnectorError::ResponseDeserializationFailed)
    );

    !has_unknown_connector_outcome
}

struct PreparedConnectorMandateRevoke {
    mca_id: id_type::MerchantConnectorAccountId,
    mandate_record: hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord,
    merchant_connector_account: domain::MerchantConnectorAccount,
    connector_data: ConnectorData,
}

/// Validate an entry against records fetched by the handler while its stored mandate status is still `Active`.
fn prepare_connector_mandate_revoke(
    state: &SessionState,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    mandate_reference: &CommonMandateReference,
    merchant_connector_accounts: &HashMap<
        id_type::MerchantConnectorAccountId,
        domain::MerchantConnectorAccount,
    >,
    mca_id: id_type::MerchantConnectorAccountId,
) -> RouterResult<PreparedConnectorMandateRevoke> {
    let merchant_connector_account = merchant_connector_accounts
        .get(&mca_id)
        .cloned()
        .ok_or_else(
            || errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: mca_id.get_string_repr().to_string(),
            },
        )?;

    // Leak-free: a cross-profile MCA is indistinguishable from a non-existent one
    if merchant_connector_account.profile_id != *profile.get_id() {
        return Err(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: mca_id.get_string_repr().to_string(),
        }
        .into());
    }

    if merchant_connector_account.disabled.unwrap_or(false) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "merchant connector account is disabled".to_string(),
        }
        .into());
    }

    let mandate_record = mandate_reference
        .payments
        .as_ref()
        .and_then(|payments| payments.get(&mca_id))
        .filter(|record| !record.connector_mandate_id.is_empty())
        .ok_or(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "no connector mandate found for merchant_connector_id {} on the payment method",
                mca_id.get_string_repr()
            ),
        })?;

    // An already-inactive entry is an error item (not an idempotent success)
    if mandate_record.connector_mandate_status != Some(api_enums::ConnectorMandateStatus::Active) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message:
                "connector mandate is already inactive or in an unknown status; nothing to revoke"
                    .to_string(),
        }
        .into());
    }

    let connector_data = ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &merchant_connector_account.connector_name,
        GetToken::Connector,
        Some(mca_id.clone()),
    )?;

    if !connector_data
        .connector
        .is_connector_mandate_revoke_supported(
            hyperswitch_interfaces::api::CurrentFlowInfo::ConnectorMandateRevoke {
                payment_method_type: mandate_record.payment_method_type,
                payment_method_id: Some(payment_method.payment_method_id.clone()),
            },
        )
    {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "connector mandate revoke is not supported for this connector or payment method type".to_string(),
        }
        .into());
    }

    Ok(PreparedConnectorMandateRevoke {
        mca_id,
        mandate_record: mandate_record.clone(),
        merchant_connector_account,
        connector_data,
    })
}

/// Execute a revoke after the matching local mandate has been staged as
/// `MandateRevocationInitiated`.
async fn revoke_connector_mandate_for_mca(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    prepared_mandate: PreparedConnectorMandateRevoke,
) -> RouterResult<ConnectorMandateRevokeResult> {
    let PreparedConnectorMandateRevoke {
        mca_id,
        mandate_record,
        merchant_connector_account,
        connector_data,
    } = prepared_mandate;
    let merchant_id = platform.get_processor().get_account().get_id();

    let mca_type =
        helpers::MerchantConnectorAccountType::DbVal(Box::new(merchant_connector_account.clone()));

    let mut router_data = mandate_utils::construct_pm_connector_mandate_revoke_router_data(
        state,
        mca_type.clone(),
        platform,
        payment_method,
        &mandate_record,
        merchant_connector_account.connector_name.clone(),
    )
    .await?;

    let gateway_context = gateway_context::RouterGatewayContext::direct(
        platform.get_processor().clone(),
        mca_type,
        merchant_id.clone(),
        profile.get_id().clone(),
        None,
    );

    // OAuth access-token pre-step (e.g. Santander Pix Automático Leg2); a no-op for
    // connectors that do not use access tokens.
    let add_access_token_result = Box::pin(access_token::add_access_token(
        state,
        &connector_data,
        &router_data,
        None,
        &gateway_context,
        None,
    ))
    .await?;

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    if add_access_token_result.connector_supports_access_token && router_data.access_token.is_none()
    {
        return Err(
            error_stack::report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "access token required for connector mandate revoke but not available",
            ),
        );
    }

    let connector_integration: services::BoxedMandateRevokeConnectorIntegrationInterface<
        types::api::MandateRevoke,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
    > = connector_data.connector.get_connector_integration();

    let connector_response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match connector_response.response {
        Ok(types::MandateRevokeResponseData {
            mandate_status: common_enums::MandateStatus::Revoked,
        }) => Ok(ConnectorMandateRevokeResult {
            merchant_connector_id: mca_id,
            connector: Some(connector_data.connector_name),
            connector_mandate_id: Some(mandate_record.connector_mandate_id.clone()),
            connector_mandate_status: Some(api_enums::ConnectorMandateStatus::Inactive),
            error: None,
        }),
        Ok(types::MandateRevokeResponseData { mandate_status }) => {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "connector reported mandate status {mandate_status} instead of revoked after revoke"
                ),
            }
            .into())
        }
        // The batch owner restores this entry to `Active` when this call fails.
        Err(connector_error) => Err(errors::ApiErrorResponse::ExternalConnectorError {
            code: connector_error.code,
            message: connector_error.message,
            connector: merchant_connector_account.connector_name.clone(),
            status_code: connector_error.status_code,
            reason: connector_error.reason,
        }
        .into()),
    }
}

async fn persist_connector_mandate_statuses(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
    mandate_reference: &mut CommonMandateReference,
    status_updates: &[(
        id_type::MerchantConnectorAccountId,
        api_enums::ConnectorMandateStatus,
    )],
) -> RouterResult<()> {
    let db = state.store.as_ref();
    let key_store = platform.get_processor().get_key_store();
    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let payment_mandate_reference = mandate_reference
        .payments
        .as_mut()
        .ok_or(errors::ApiErrorResponse::MandateNotFound)?;

    for (merchant_connector_id, status) in status_updates {
        let mandate_record = payment_mandate_reference
            .get_mut(merchant_connector_id)
            .ok_or(errors::ApiErrorResponse::MandateNotFound)?;
        mandate_record.connector_mandate_status = Some(*status);
    }

    cards::update_payment_method_connector_mandate_details(
        key_store,
        db,
        payment_method.clone(),
        Some(mandate_reference.clone()),
        storage_scheme,
        platform.get_initiator(),
        None,
    )
    .await
    .map_err(|error| {
        logger::error!(
            payment_method_id = payment_method.payment_method_id.as_str(),
            transition_count = status_updates.len(),
            ?error,
            "failed to persist connector mandate revoke states"
        );
        error_stack::report!(errors::ApiErrorResponse::MandateUpdateFailed)
            .attach_printable("failed to persist connector mandate revoke states")
    })
}
