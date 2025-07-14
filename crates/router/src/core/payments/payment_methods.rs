//! Contains functions of payment methods that are used in payments
//! one of such functions is `list_payment_methods`

use std::collections::{BTreeMap, HashSet};

use std::{collections::HashSet, str::FromStr};

use common_utils::{
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use error_stack::ResultExt;
use euclid::dssa::graph::{AnalysisContext, CgraphExt};
use hyperswitch_constraint_graph as cgraph;
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;
use kgraph_utils::transformers::IntoDirValue;

use super::errors;
use crate::{
    configs::settings,
    core::{payment_methods, payments::helpers},
    db::errors::StorageErrorExt,
    logger,
    routes,
    types::{self, api, domain, storage},
    settings,
};

#[cfg(feature = "v2")]
pub async fn list_payment_methods(
    state: routes::SessionState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    payment_id: id_type::GlobalPaymentId,
    req: api_models::payments::PaymentMethodsListRequest,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
) -> errors::RouterResponse<api_models::payments::PaymentMethodListResponseForPayments> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let payment_intent = db
        .find_payment_intent_by_id(
            key_manager_state,
            &payment_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    validate_payment_status_for_payment_method_list(payment_intent.status)?;

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            key_manager_state,
            profile.get_id(),
            merchant_context.get_merchant_key_store(),
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let customer_payment_methods = match &payment_intent.customer_id {
        Some(customer_id) => Some(
            payment_methods::list_customer_payment_methods_core(
                &state,
                &merchant_context,
                customer_id,
            )
            .await?,
        ),
        None => None,
    };

    let response =
        hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts.clone())
            .perform_filtering(
                &state,
                &merchant_context,
                profile.get_id(),
                &req,
                &payment_intent,
            ).await?
            .merge_and_transform()
            .get_required_fields(RequiredFieldsInput::new())
            .perform_surcharge_calculation()
            .populate_pm_subtype_specific_data(&state.conf.bank_config)
            .generate_response(customer_payment_methods);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

/// Container for the inputs required for the required fields
struct RequiredFieldsInput {}

impl RequiredFieldsInput {
    fn new() -> Self {
        Self {}
    }
}

struct FlattenedPaymentMethodsEnabled(
    hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled,
);

impl FlattenedPaymentMethodsEnabled {
    fn perform_filtering(self) -> FilteredPaymentMethodsEnabled {
        FilteredPaymentMethodsEnabled(self.0.payment_methods_enabled)
    }
}

/// Container for the filtered payment methods
struct FilteredPaymentMethodsEnabled(
    Vec<hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector>,
);

impl FilteredPaymentMethodsEnabled {
    fn merge_and_transform(self) -> MergedEnabledPaymentMethodTypes {
        let values = self
            .0
            .into_iter()
            // BTreeMap used to ensure soretd response, otherwise the response is arbitrarily ordered
            .fold(BTreeMap::new(), |mut acc, item| {
                let key = (
                    item.payment_method,
                    item.payment_methods_enabled.payment_method_subtype,
                );
                let (experiences, connectors) = acc
                    .entry(key)
                    // HashSet used to ensure payment_experience does not have duplicates, due to multiple connectors for a pm_subtype
                    .or_insert_with(|| (HashSet::new(), Vec::new()));

                if let Some(experience) = item.payment_methods_enabled.payment_experience {
                    experiences.insert(experience);
                }
                connectors.push(item.connector);

                acc
            })
            .into_iter()
            .map(
                |(
                    (payment_method_type, payment_method_subtype),
                    (payment_experience, connectors),
                )| {
                    MergedEnabledPaymentMethod {
                        payment_method_type,
                        payment_method_subtype,
                        payment_experience: if payment_experience.is_empty() {
                            None
                        } else {
                            Some(payment_experience.into_iter().collect())
                        },
                        connectors,
                    }
                },
            )
            .collect();
        MergedEnabledPaymentMethodTypes(values)
    }
}

/// Element container to hold the filtered payment methods with payment_experience and connectors merged for a pm_subtype
struct MergedEnabledPaymentMethod {
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
    payment_experience: Option<Vec<common_enums::PaymentExperience>>,
    connectors: Vec<api_models::enums::Connector>,
}

/// Container to hold the filtered payment methods with payment_experience and connectors merged for a pm_subtype
struct MergedEnabledPaymentMethodTypes(Vec<MergedEnabledPaymentMethod>);

impl MergedEnabledPaymentMethodTypes {
    fn get_required_fields(
        self,
        _input: RequiredFieldsInput,
    ) -> RequiredFieldsForEnabledPaymentMethodTypes {
        let required_fields_info = self
            .0
            .into_iter()
            .map(
                |payment_methods_enabled| RequiredFieldsForEnabledPaymentMethod {
                    required_field: None,
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    connectors: payment_methods_enabled.connectors,
                },
            )
            .collect();

        RequiredFieldsForEnabledPaymentMethodTypes(required_fields_info)
    }
}

/// Element container to hold the filtered payment methods with required fields
struct RequiredFieldsForEnabledPaymentMethod {
    required_field: Option<Vec<api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
    payment_experience: Option<Vec<common_enums::PaymentExperience>>,
    connectors: Vec<api_models::enums::Connector>,
}

/// Container to hold the filtered payment methods enabled with required fields
struct RequiredFieldsForEnabledPaymentMethodTypes(Vec<RequiredFieldsForEnabledPaymentMethod>);

impl RequiredFieldsForEnabledPaymentMethodTypes {
    fn perform_surcharge_calculation(
        self,
    ) -> RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes {
        // TODO: Perform surcharge calculation
        let details_with_surcharge = self
            .0
            .into_iter()
            .map(
                |payment_methods_enabled| RequiredFieldsAndSurchargeForEnabledPaymentMethodType {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    required_field: payment_methods_enabled.required_field,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    surcharge: None,
                    connectors: payment_methods_enabled.connectors,
                },
            )
            .collect();

        RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes(details_with_surcharge)
    }
}

/// Element Container to hold the filtered payment methods enabled with required fields and surcharge
struct RequiredFieldsAndSurchargeForEnabledPaymentMethodType {
    required_field: Option<Vec<api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
    payment_experience: Option<Vec<common_enums::PaymentExperience>>,
    connectors: Vec<api_models::enums::Connector>,
    surcharge: Option<api_models::payment_methods::SurchargeDetailsResponse>,
}

/// Container to hold the filtered payment methods enabled with required fields and surcharge
struct RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes(
    Vec<RequiredFieldsAndSurchargeForEnabledPaymentMethodType>,
);

fn get_pm_subtype_specific_data(
    bank_config: &settings::BankRedirectConfig,
    payment_method_type: common_enums::enums::PaymentMethod,
    payment_method_subtype: common_enums::enums::PaymentMethodType,
    connectors: &[api_models::enums::Connector],
) -> Option<api_models::payment_methods::PaymentMethodSubtypeSpecificData> {
    match payment_method_type {
        // TODO: Return card_networks
        common_enums::PaymentMethod::Card | common_enums::PaymentMethod::CardRedirect => None,

        common_enums::PaymentMethod::BankRedirect
        | common_enums::PaymentMethod::BankTransfer
        | common_enums::PaymentMethod::BankDebit
        | common_enums::PaymentMethod::OpenBanking => {
            if let Some(connector_bank_names) = bank_config.0.get(&payment_method_subtype) {
                let bank_names = connectors
                    .iter()
                    .filter_map(|connector| {
                        connector_bank_names.0.get(&connector.to_string())
                            .map(|connector_hash_set| {
                                connector_hash_set.banks.clone()
                            })
                            .or_else(|| {
                                logger::debug!("Could not find any configured connectors for payment_method -> {payment_method_subtype} for connector -> {connector}");
                                None
                            })
                    })
                    .flatten()
                    .collect();
                Some(
                    api_models::payment_methods::PaymentMethodSubtypeSpecificData::Bank {
                        bank_names,
                    },
                )
            } else {
                logger::debug!("Could not find any configured banks for payment_method -> {payment_method_subtype}");
                None
            }
        }

        common_enums::PaymentMethod::PayLater
        | common_enums::PaymentMethod::Wallet
        | common_enums::PaymentMethod::Crypto
        | common_enums::PaymentMethod::Reward
        | common_enums::PaymentMethod::RealTimePayment
        | common_enums::PaymentMethod::Upi
        | common_enums::PaymentMethod::Voucher
        | common_enums::PaymentMethod::GiftCard
        | common_enums::PaymentMethod::MobilePayment => None,
    }
}

impl RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes {
    fn populate_pm_subtype_specific_data(
        self,
        bank_config: &settings::BankRedirectConfig,
    ) -> RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodTypes {
        let response_payment_methods = self
            .0
            .into_iter()
            .map(|payment_methods_enabled| {
                RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodType {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    required_field: payment_methods_enabled.required_field,
                    surcharge: payment_methods_enabled.surcharge,
                    pm_subtype_specific_data: get_pm_subtype_specific_data(
                        bank_config,
                        payment_methods_enabled.payment_method_type,
                        payment_methods_enabled.payment_method_subtype,
                        &payment_methods_enabled.connectors,
                    ),
                }
            })
            .collect();

        RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodTypes(
            response_payment_methods,
        )
    }
}

/// Element Container to hold the filtered payment methods enabled with required fields, surcharge and subtype specific data
struct RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodType {
    required_field: Option<Vec<api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
    payment_experience: Option<Vec<common_enums::PaymentExperience>>,
    surcharge: Option<api_models::payment_methods::SurchargeDetailsResponse>,
    pm_subtype_specific_data: Option<api_models::payment_methods::PaymentMethodSubtypeSpecificData>,
}

/// Container to hold the filtered payment methods enabled with required fields, surcharge and subtype specific data
struct RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodTypes(
    Vec<RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodType>,
);

impl RequiredFieldsAndSurchargeWithExtraInfoForEnabledPaymentMethodTypes {
    fn generate_response(
        self,
        customer_payment_methods: Option<
            Vec<api_models::payment_methods::CustomerPaymentMethodResponseItem>,
        >,
    ) -> api_models::payments::PaymentMethodListResponseForPayments {
        let response_payment_methods = self
            .0
            .into_iter()
            .map(|payment_methods_enabled| {
                api_models::payments::ResponsePaymentMethodTypesForPayments {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    required_fields: payment_methods_enabled.required_field,
                    surcharge_details: payment_methods_enabled.surcharge,
                    extra_information: payment_methods_enabled.pm_subtype_specific_data,
                }
            })
            .collect();

        api_models::payments::PaymentMethodListResponseForPayments {
            payment_methods_enabled: response_payment_methods,
            customer_payment_methods,
        }
    }
}

impl RequiredFieldsForEnabledPaymentMethodTypes {
    fn perform_surcharge_calculation(
        self,
    ) -> RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes {
        let details_with_surcharge = self
            .0
            .into_iter()
            .map(
                |payment_methods_enabled| RequiredFieldsAndSurchargeForEnabledPaymentMethodType {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    required_field: payment_methods_enabled.required_field,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    surcharge: None,
                },
            )
            .collect();

        RequiredFieldsAndSurchargeForEnabledPaymentMethodTypes(details_with_surcharge)
    }
}

trait PerformFilteringOnPaymentMethodsEnabled {
    async fn perform_filtering(
        self,
        state: &routes::SessionState,
        merchant_context: &domain::MerchantContext,
        profile_id: &id_type::ProfileId,
        req: &api_models::payments::PaymentMethodsListRequest,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    ) -> errors::RouterResult<FilteredPaymentMethodsEnabled>;
}

impl PerformFilteringOnPaymentMethodsEnabled
    for hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled
{
    async fn perform_filtering(
        self,
        state: &routes::SessionState,
        merchant_context: &domain::MerchantContext,
        profile_id: &id_type::ProfileId,
        req: &api_models::payments::PaymentMethodsListRequest,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    ) -> errors::RouterResult<FilteredPaymentMethodsEnabled> {
        let billing_address = payment_intent
            .billing_address
            .clone()
            .and_then(|address| address.into_inner().address);

        let mut response: Vec<hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector> = vec![];
        // Key creation for storing PM_FILTER_CGRAPH
        let key = {
            format!(
                "pm_filters_cgraph_{}_{}",
                merchant_context
                    .get_merchant_account()
                    .get_id()
                    .get_string_repr(),
                profile_id.get_string_repr()
            )
        };

        if let Some(graph) = payment_methods::utils::get_merchant_pm_filter_graph(state, &key).await
        {
            // Derivation of PM_FILTER_CGRAPH from MokaCache successful
            for payment_method_enabled_details in self.payment_methods_enabled {
                filter_payment_methods(
                    &graph,
                    payment_method_enabled_details,
                    req,
                    &mut response,
                    Some(payment_intent),
                    billing_address.as_ref(),
                    &state.conf,
                )
                .await?;
            }
        } else {
            // No PM_FILTER_CGRAPH Cache present in MokaCache
            let mut builder = cgraph::ConstraintGraphBuilder::new();

            for payment_method_enabled_details in &self.payment_methods_enabled {
                let domain_id = builder.make_domain(
                    payment_method_enabled_details
                        .merchant_connector_id
                        .get_string_repr()
                        .to_string(),
                    &payment_method_enabled_details.connector.to_string(),
                );

                let Ok(domain_id) = domain_id else {
                    router_env::logger::error!(
                        "Failed to construct domain for list payment methods"
                    );
                    return Err(errors::ApiErrorResponse::InternalServerError.into());
                };

                if let Err(e) = payment_methods::utils::make_pm_graph(
                    &mut builder,
                    domain_id,
                    payment_method_enabled_details.payment_method,
                    payment_method_enabled_details
                        .payment_methods_enabled
                        .clone(),
                    payment_method_enabled_details.connector.to_string(),
                    &state.conf.pm_filters,
                    &state.conf.mandates.supported_payment_methods,
                    &state.conf.mandates.update_mandate_supported,
                ) {
                    router_env::logger::error!(
                        "Failed to construct constraint graph for list payment methods {e:?}"
                    );
                }
            }

            // Refreshing our CGraph cache
            let graph =
                payment_methods::utils::refresh_pm_filters_cache(state, &key, builder.build())
                    .await;

            for payment_method_enabled_details in self.payment_methods_enabled {
                filter_payment_methods(
                    &graph,
                    payment_method_enabled_details,
                    req,
                    &mut response,
                    Some(payment_intent),
                    billing_address.as_ref(),
                    &state.conf,
                )
                .await?;
            }
        }

        Ok(FilteredPaymentMethodsEnabled(response))
    }
}

// v2 type for PaymentMethodListRequest will not have the installment_payment_enabled field,
#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    graph: &cgraph::ConstraintGraph<euclid::frontend::dir::DirValue>,
    payment_method_type_details: hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    req: &api_models::payments::PaymentMethodsListRequest,
    resp: &mut Vec<
        hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    >,
    payment_intent: Option<&storage::PaymentIntent>,
    address: Option<&hyperswitch_domain_models::address::AddressDetails>,
    configs: &settings::Settings<RawSecret>,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    let allowed_payment_method_types =
        payment_intent.and_then(|intent| intent.allowed_payment_method_types.clone());

    if filter_recurring_based(
        &payment_method_type_details.payment_methods_enabled,
        req.recurring_enabled,
    ) && filter_amount_based(
        &payment_method_type_details.payment_methods_enabled,
        req.amount,
    ) {
        let payment_method_object = payment_method_type_details.payment_methods_enabled.clone();

        let pm_dir_value: euclid::frontend::dir::DirValue = (
            payment_method_type_details
                .payment_methods_enabled
                .payment_method_subtype,
            payment_method_type_details.payment_method,
        )
            .into_dir_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("pm_value_node not created")?;

        let mut context_values: Vec<euclid::frontend::dir::DirValue> = Vec::new();
        context_values.push(pm_dir_value.clone());

        payment_intent.map(|intent| {
            context_values.push(euclid::frontend::dir::DirValue::PaymentCurrency(
                intent.amount_details.currency,
            ))
        });
        address.map(|address| {
            address.country.map(|country| {
                context_values.push(euclid::frontend::dir::DirValue::BillingCountry(
                    common_enums::Country::from_alpha2(country),
                ))
            })
        });

        // Addition of Connector to context
        if let Ok(connector) = api_models::enums::RoutableConnectors::from_str(
            payment_method_type_details.connector.to_string().as_str(),
        ) {
            context_values.push(euclid::frontend::dir::DirValue::Connector(Box::new(
                api_models::routing::ast::ConnectorChoice { connector },
            )));
        };

        let filter_pm_based_on_allowed_types = filter_pm_based_on_allowed_types(
            allowed_payment_method_types.as_ref(),
            payment_method_object.payment_method_subtype,
        );

        // Filter logic for payment method types based on the below conditions
        // Case 1: If the payment method type support Zero Mandate flow, filter only payment method type that support it
        // Case 2: Whether the payment method type support Mandates or not, list all the payment method types
        if payment_intent
            .map(|intent| intent.setup_future_usage == common_enums::FutureUsage::OffSession)
            .unwrap_or(false)
        {
            payment_intent
                .map(|intent| intent.amount_details.calculate_net_amount())
                .map(|amount| {
                    if amount == types::MinorUnit::zero() {
                        if configs
                            .zero_mandates
                            .supported_payment_methods
                            .0
                            .get(&payment_method_type_details.payment_method)
                            .and_then(|supported_pm_for_mandates| {
                                supported_pm_for_mandates
                                    .0
                                    .get(
                                        &payment_method_type_details
                                            .payment_methods_enabled
                                            .payment_method_subtype,
                                    )
                                    .map(|supported_connector_for_mandates| {
                                        supported_connector_for_mandates
                                            .connector_list
                                            .contains(&payment_method_type_details.connector)
                                    })
                            })
                            .unwrap_or(false)
                        {
                            context_values.push(euclid::frontend::dir::DirValue::PaymentType(
                                euclid::enums::PaymentType::SetupMandate,
                            ));
                        }
                    } else if configs
                        .mandates
                        .supported_payment_methods
                        .0
                        .get(&payment_method_type_details.payment_method)
                        .and_then(|supported_pm_for_mandates| {
                            supported_pm_for_mandates
                                .0
                                .get(
                                    &payment_method_type_details
                                        .payment_methods_enabled
                                        .payment_method_subtype,
                                )
                                .map(|supported_connector_for_mandates| {
                                    supported_connector_for_mandates
                                        .connector_list
                                        .contains(&payment_method_type_details.connector)
                                })
                        })
                        .unwrap_or(false)
                    {
                        context_values.push(euclid::frontend::dir::DirValue::PaymentType(
                            euclid::enums::PaymentType::NewMandate,
                        ));
                    } else {
                        context_values.push(euclid::frontend::dir::DirValue::PaymentType(
                            euclid::enums::PaymentType::NonMandate,
                        ));
                    }
                });
        } else {
            context_values.push(euclid::frontend::dir::DirValue::PaymentType(
                euclid::enums::PaymentType::NonMandate,
            ));
        }

        payment_intent.map(|inner| {
            context_values.push(euclid::frontend::dir::DirValue::CaptureMethod(
                inner.capture_method,
            ));
        });

        let filter_pm_card_network_based = filter_pm_card_network_based(
            payment_method_object.card_networks.as_ref(),
            req.card_networks.as_ref(),
            payment_method_object.payment_method_subtype,
        );

        let saved_payment_methods_filter = req
            .client_secret
            .as_ref()
            .map(|cs| {
                if cs.starts_with("cs_") {
                    configs
                        .saved_payment_methods
                        .sdk_eligible_payment_methods
                        .contains(
                            payment_method_type_details
                                .payment_method
                                .to_string()
                                .as_str(),
                        )
                } else {
                    true
                }
            })
            .unwrap_or(true);

        let context = AnalysisContext::from_dir_values(context_values.clone());
        router_env::logger::info!("Context created for List Payment method is {:?}", context);

        let domain_ident: &[String] = &[payment_method_type_details
            .merchant_connector_id
            .get_string_repr()
            .to_string()];
        let result = graph.key_value_analysis(
            pm_dir_value.clone(),
            &context,
            &mut cgraph::Memoization::new(),
            &mut cgraph::CycleCheck::new(),
            Some(domain_ident),
        );
        if let Err(ref e) = result {
            router_env::logger::error!(
                "Error while performing Constraint graph's key value analysis
                for list payment methods {:?}",
                e
            );
        } else if filter_pm_based_on_allowed_types
            && filter_pm_card_network_based
            && saved_payment_methods_filter
            && matches!(result, Ok(()))
        {
            resp.push(payment_method_type_details);
        } else {
            router_env::logger::error!("Filtering Payment Methods Failed");
        }
    }
    Ok(())
}

fn filter_recurring_based(
    payment_method: &common_types::payment_methods::RequestPaymentMethodTypes,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| {
        payment_method.recurring_enabled == Some(enabled)
    })
}

fn filter_amount_based(
    payment_method: &common_types::payment_methods::RequestPaymentMethodTypes,
    amount: Option<types::MinorUnit>,
) -> bool {
    let min_check = amount
        .and_then(|amt| payment_method.minimum_amount.map(|min_amt| amt >= min_amt))
        .unwrap_or(true);
    let max_check = amount
        .and_then(|amt| payment_method.maximum_amount.map(|max_amt| amt <= max_amt))
        .unwrap_or(true);
    (min_check && max_check) || amount == Some(types::MinorUnit::zero())
}

fn filter_pm_based_on_allowed_types(
    allowed_types: Option<&Vec<api_models::enums::PaymentMethodType>>,
    payment_method_type: api_models::enums::PaymentMethodType,
) -> bool {
    allowed_types.map_or(true, |pm| pm.contains(&payment_method_type))
}

fn filter_pm_card_network_based(
    pm_card_networks: Option<&Vec<api_models::enums::CardNetwork>>,
    request_card_networks: Option<&Vec<api_models::enums::CardNetwork>>,
    pm_type: api_models::enums::PaymentMethodType,
) -> bool {
    match pm_type {
        api_models::enums::PaymentMethodType::Credit
        | api_models::enums::PaymentMethodType::Debit => {
            match (pm_card_networks, request_card_networks) {
                (Some(pm_card_networks), Some(request_card_networks)) => request_card_networks
                    .iter()
                    .all(|card_network| pm_card_networks.contains(card_network)),
                (None, Some(_)) => false,
                _ => true,
            }
        }
        _ => true,
    }
}

/// Validate if payment methods list can be performed on the current status of payment intent
fn validate_payment_status_for_payment_method_list(
    intent_status: common_enums::IntentStatus,
) -> Result<(), errors::ApiErrorResponse> {
    match intent_status {
        common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
        common_enums::IntentStatus::Succeeded
        | common_enums::IntentStatus::Conflicted
        | common_enums::IntentStatus::Failed
        | common_enums::IntentStatus::Cancelled
        | common_enums::IntentStatus::Processing
        | common_enums::IntentStatus::RequiresCustomerAction
        | common_enums::IntentStatus::RequiresMerchantAction
        | common_enums::IntentStatus::RequiresCapture
        | common_enums::IntentStatus::PartiallyCaptured
        | common_enums::IntentStatus::RequiresConfirmation
        | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
            Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow: "list_payment_methods".to_string(),
                field_name: "status".to_string(),
                current_value: intent_status.to_string(),
                states: ["requires_payment_method".to_string()].join(", "),
            })
        }
    }
}
