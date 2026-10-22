use std::collections::{HashMap, HashSet};

use api_models::{admin::MerchantConnectorInfo, enums};
use common_utils::id_type;
#[cfg(feature = "v1")]
use error_stack::{report, ResultExt};
use strum::IntoEnumIterator;

#[cfg(feature = "v1")]
use crate::types::storage::enums as storage_enums;
use crate::{
    core::{
        admin,
        errors::{self, RouterResponse, StorageErrorExt},
    },
    routes::SessionState,
    services,
    types::{api, domain},
};

/// Available filter values for a platform, aggregated across all of its connected merchants.
///
/// Connectors and payment methods are derived from the *configured* merchant connector
/// accounts of every connected merchant under the platform's organization (not from payment
/// data), mirroring [`get_payment_filters`]. The remaining filters (currency, status,
/// authentication type, card network, card discovery) are the full set of supported values.
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_platform_payment_filters(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
) -> RouterResponse<api::PlatformPaymentListFilters> {
    // This endpoint is exclusively for platform merchants - not connected, not standard.
    common_utils::fp_utils::when(
        !platform.get_provider().get_account().is_platform_account(),
        || {
            Err(report!(errors::ApiErrorResponse::Unauthorized)).attach_printable(
                "Platform payment filters are only accessible to platform merchants",
            )
        },
    )?;

    let db = state.store.as_ref();

    // Every connected merchant lives under the platform's organization.
    let merchant_accounts = db
        .list_merchant_accounts_by_organization_id(
            platform.get_provider().get_account().get_org_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mut connector_map: HashMap<String, Vec<MerchantConnectorInfo>> = HashMap::new();
    let mut payment_method_types_map: HashMap<
        enums::PaymentMethod,
        HashSet<enums::PaymentMethodType>,
    > = HashMap::new();

    for connected_account in merchant_accounts.into_iter().filter(|account| {
        account.merchant_account_type == common_enums::MerchantAccountType::Connected
    }) {
        let merchant_id = connected_account.get_id().clone();
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        // `list_payment_connectors` operates on a processor identity; build one for this
        // connected merchant (provider == processor since we are not acting on its behalf).
        let processor = domain::Platform::new(
            connected_account.clone(),
            key_store.clone(),
            connected_account,
            key_store,
            None,
        )
        .get_processor()
        .clone();

        let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
            admin::list_payment_connectors(state.clone(), processor, profile_id_list.clone())
                .await?
        {
            data
        } else {
            return Err(errors::ApiErrorResponse::InternalServerError.into());
        };

        // populate connector map
        merchant_connector_accounts
            .iter()
            .filter_map(|merchant_connector_account| {
                merchant_connector_account
                    .connector_label
                    .as_ref()
                    .map(|label| {
                        let info = merchant_connector_account.to_merchant_connector_info(label);
                        (merchant_connector_account.get_connector_name(), info)
                    })
            })
            .for_each(|(connector_name, info)| {
                connector_map
                    .entry(connector_name.to_string())
                    .or_default()
                    .push(info);
            });

        // populate payment method type map
        merchant_connector_accounts
            .iter()
            .flat_map(|merchant_connector_account| {
                merchant_connector_account.payment_methods_enabled.as_ref()
            })
            .map(|payment_methods_enabled| {
                payment_methods_enabled
                    .iter()
                    .filter_map(|payment_method_enabled| {
                        payment_method_enabled
                            .get_payment_method_type()
                            .map(|types_vec| {
                                (
                                    payment_method_enabled.get_payment_method(),
                                    types_vec.clone(),
                                )
                            })
                    })
            })
            .for_each(|payment_methods_enabled| {
                payment_methods_enabled.for_each(
                    |(payment_method_option, payment_method_types_vec)| {
                        if let Some(payment_method) = payment_method_option {
                            payment_method_types_map
                                .entry(payment_method)
                                .or_default()
                                .extend(payment_method_types_vec.iter().filter_map(
                                    |req_payment_method_types| {
                                        req_payment_method_types.get_payment_method_type()
                                    },
                                ));
                        }
                    },
                );
            });
    }

    Ok(services::ApplicationResponse::Json(
        api::PlatformPaymentListFilters {
            connector: connector_map,
            currency: enums::Currency::iter().collect(),
            status: enums::IntentStatus::iter().collect(),
            payment_method: payment_method_types_map,
            authentication_type: enums::AuthenticationType::iter().collect(),
            card_network: enums::CardNetwork::iter().collect(),
            card_discovery: enums::CardDiscovery::iter().collect(),
        },
    ))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_filters_for_payments(
    state: SessionState,
    platform: domain::Platform,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentListFilters> {
    let db = state.store.as_ref();
    let pi = db
        .filter_payment_intents_by_time_range_constraints(
            platform.get_processor().get_account().get_id(),
            &time_range,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payments(
            pi.as_slice(),
            platform.get_processor().get_account().get_id(),
            // since OLAP doesn't have KV. Force to get the data from PSQL.
            storage_enums::MerchantStorageScheme::PostgresOnly,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    Ok(services::ApplicationResponse::Json(
        api::PaymentListFilters {
            connector: filters.connector,
            currency: filters.currency,
            status: filters.status,
            payment_method: filters.payment_method,
            payment_method_type: filters.payment_method_type,
            authentication_type: filters.authentication_type,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_payment_filters(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
) -> RouterResponse<api::PaymentListFiltersV2> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        admin::list_payment_connectors(state, platform.get_processor().clone(), profile_id_list)
            .await?
    {
        data
    } else {
        return Err(errors::ApiErrorResponse::InternalServerError.into());
    };

    let mut connector_map: HashMap<String, Vec<MerchantConnectorInfo>> = HashMap::new();
    let mut payment_method_types_map: HashMap<
        enums::PaymentMethod,
        HashSet<enums::PaymentMethodType>,
    > = HashMap::new();

    // populate connector map
    merchant_connector_accounts
        .iter()
        .filter_map(|merchant_connector_account| {
            merchant_connector_account
                .connector_label
                .as_ref()
                .map(|label| {
                    let info = merchant_connector_account.to_merchant_connector_info(label);
                    (merchant_connector_account.get_connector_name(), info)
                })
        })
        .for_each(|(connector_name, info)| {
            connector_map
                .entry(connector_name.to_string())
                .or_default()
                .push(info);
        });

    // populate payment method type map
    merchant_connector_accounts
        .iter()
        .flat_map(|merchant_connector_account| {
            merchant_connector_account.payment_methods_enabled.as_ref()
        })
        .map(|payment_methods_enabled| {
            payment_methods_enabled
                .iter()
                .filter_map(|payment_method_enabled| {
                    payment_method_enabled
                        .get_payment_method_type()
                        .map(|types_vec| {
                            (
                                payment_method_enabled.get_payment_method(),
                                types_vec.clone(),
                            )
                        })
                })
        })
        .for_each(|payment_methods_enabled| {
            payment_methods_enabled.for_each(
                |(payment_method_option, payment_method_types_vec)| {
                    if let Some(payment_method) = payment_method_option {
                        payment_method_types_map
                            .entry(payment_method)
                            .or_default()
                            .extend(payment_method_types_vec.iter().filter_map(
                                |req_payment_method_types| {
                                    req_payment_method_types.get_payment_method_type()
                                },
                            ));
                    }
                },
            );
        });

    Ok(services::ApplicationResponse::Json(
        api::PaymentListFiltersV2 {
            connector: connector_map,
            currency: enums::Currency::iter().collect(),
            status: enums::IntentStatus::iter().collect(),
            payment_method: payment_method_types_map,
            authentication_type: enums::AuthenticationType::iter().collect(),
            card_network: enums::CardNetwork::iter().collect(),
            card_discovery: enums::CardDiscovery::iter().collect(),
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_aggregates_for_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentsAggregateResponse> {
    let db = state.store.as_ref();
    let intent_status_with_count = db
        .get_intent_status_with_count(
            platform.get_processor().get_account().get_id(),
            profile_id_list,
            &time_range,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let mut status_map: HashMap<enums::IntentStatus, i64> =
        intent_status_with_count.into_iter().collect();
    for status in enums::IntentStatus::iter() {
        status_map.entry(status).or_default();
    }

    Ok(services::ApplicationResponse::Json(
        api::PaymentsAggregateResponse {
            status_with_count: status_map,
        },
    ))
}
