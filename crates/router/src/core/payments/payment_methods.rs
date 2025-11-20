//! Contains functions of payment methods that are used in payments
//! one of such functions is `list_payment_methods`

use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};

use common_utils::{
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;

use super::errors;
use crate::{
    configs::settings,
    core::{payment_methods, payments::helpers},
    db::errors::StorageErrorExt,
    logger, routes,
    types::{self, api, domain, storage},
};

#[cfg(feature = "v2")]
pub async fn list_payment_methods(
    state: routes::SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    payment_id: id_type::GlobalPaymentId,
    req: api_models::payments::ListMethodsForPaymentsRequest,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
) -> errors::RouterResponse<api_models::payments::PaymentMethodListResponseForPayments> {
    let db = &*state.store;

    let payment_intent = db
        .find_payment_intent_by_id(
            &payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    validate_payment_status_for_payment_method_list(payment_intent.status)?;

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            profile.get_id(),
            platform.get_processor().get_key_store(),
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let customer_payment_methods = match &payment_intent.customer_id {
        Some(customer_id) => Some(
            payment_methods::list_customer_payment_methods_core(&state, &platform, customer_id)
                .await?,
        ),
        None => None,
    };

    let response =
        FlattenedPaymentMethodsEnabled(hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts))
            .perform_filtering(
                &state,
                &platform,
                profile.get_id(),
                &req,
                &payment_intent,
            ).await?
            .store_gift_card_mca_in_redis(&payment_id, db, &profile).await
            .merge_and_transform()
            .get_required_fields(RequiredFieldsInput::new(state.conf.required_fields.clone(), payment_intent.setup_future_usage))
            .perform_surcharge_calculation()
            .populate_pm_subtype_specific_data(&state.conf.bank_config)
            .generate_response(customer_payment_methods);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

/// Container for the inputs required for the required fields
struct RequiredFieldsInput {
    required_fields_config: settings::RequiredFields,
    setup_future_usage: common_enums::FutureUsage,
}

impl RequiredFieldsInput {
    fn new(
        required_fields_config: settings::RequiredFields,
        setup_future_usage: common_enums::FutureUsage,
    ) -> Self {
        Self {
            required_fields_config,
            setup_future_usage,
        }
    }
}

trait GetRequiredFields {
    fn get_required_fields(
        &self,
        payment_method_enabled: &MergedEnabledPaymentMethod,
    ) -> Option<&settings::RequiredFieldFinal>;
}

impl GetRequiredFields for settings::RequiredFields {
    fn get_required_fields(
        &self,
        payment_method_enabled: &MergedEnabledPaymentMethod,
    ) -> Option<&settings::RequiredFieldFinal> {
        self.0
            .get(&payment_method_enabled.payment_method_type)
            .and_then(|required_fields_for_payment_method| {
                required_fields_for_payment_method
                    .0
                    .get(&payment_method_enabled.payment_method_subtype)
            })
            .map(|connector_fields| &connector_fields.fields)
            .and_then(|connector_hashmap| {
                payment_method_enabled
                    .connectors
                    .first()
                    .and_then(|connector| connector_hashmap.get(connector))
            })
    }
}

struct FlattenedPaymentMethodsEnabled(
    hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled,
);

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
    async fn store_gift_card_mca_in_redis(
        self,
        payment_id: &id_type::GlobalPaymentId,
        db: &dyn crate::db::StorageInterface,
        profile: &domain::Profile,
    ) -> Self {
        let gift_card_connector_id = self
            .0
            .iter()
            .find(|item| item.payment_method == common_enums::PaymentMethod::GiftCard)
            .map(|item| &item.merchant_connector_id);

        if let Some(gift_card_mca) = gift_card_connector_id {
            let gc_key = payment_id.get_gift_card_connector_key();
            let redis_expiry = profile
                .get_order_fulfillment_time()
                .unwrap_or(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME);

            let redis_conn = db
                .get_redis_conn()
                .map_err(|redis_error| logger::error!(?redis_error))
                .ok();

            if let Some(rc) = redis_conn {
                rc.set_key_with_expiry(
                    &gc_key.as_str().into(),
                    gift_card_mca.get_string_repr().to_string(),
                    redis_expiry,
                )
                .await
                .attach_printable("Failed to store gift card mca_id in redis")
                .unwrap_or_else(|error| {
                    logger::error!(?error);
                })
            };
        } else {
            logger::error!(
                "Could not find any configured MCA supporting gift card for payment_id -> {}",
                payment_id.get_string_repr()
            );
        }

        self
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
        input: RequiredFieldsInput,
    ) -> RequiredFieldsForEnabledPaymentMethodTypes {
        let required_fields_config = input.required_fields_config;
        let is_cit_transaction = input.setup_future_usage == common_enums::FutureUsage::OffSession;

        let required_fields_info = self
            .0
            .into_iter()
            .map(|payment_methods_enabled| {
                let required_fields =
                    required_fields_config.get_required_fields(&payment_methods_enabled);

                let required_fields = required_fields
                    .map(|required_fields| {
                        let common_required_fields = required_fields
                            .common
                            .iter()
                            .flatten()
                            .map(ToOwned::to_owned);

                        // Collect mandate required fields because this is for zero auth mandates only
                        let mandate_required_fields = required_fields
                            .mandate
                            .iter()
                            .flatten()
                            .map(ToOwned::to_owned);

                        // Collect non-mandate required fields because this is for zero auth mandates only
                        let non_mandate_required_fields = required_fields
                            .non_mandate
                            .iter()
                            .flatten()
                            .map(ToOwned::to_owned);

                        // Combine mandate and non-mandate required fields based on setup_future_usage
                        if is_cit_transaction {
                            common_required_fields
                                .chain(non_mandate_required_fields)
                                .collect::<Vec<_>>()
                        } else {
                            common_required_fields
                                .chain(mandate_required_fields)
                                .collect::<Vec<_>>()
                        }
                    })
                    .unwrap_or_default();

                RequiredFieldsForEnabledPaymentMethod {
                    required_fields,
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    payment_experience: payment_methods_enabled.payment_experience,
                    connectors: payment_methods_enabled.connectors,
                }
            })
            .collect();

        RequiredFieldsForEnabledPaymentMethodTypes(required_fields_info)
    }
}

/// Element container to hold the filtered payment methods with required fields
struct RequiredFieldsForEnabledPaymentMethod {
    required_fields: Vec<api_models::payment_methods::RequiredFieldInfo>,
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
                    required_fields: payment_methods_enabled.required_fields,
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
    required_fields: Vec<api_models::payment_methods::RequiredFieldInfo>,
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
                    required_fields: payment_methods_enabled.required_fields,
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
    required_fields: Vec<api_models::payment_methods::RequiredFieldInfo>,
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
                    required_fields: payment_methods_enabled.required_fields,
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

impl FlattenedPaymentMethodsEnabled {
    async fn perform_filtering(
        self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        profile_id: &id_type::ProfileId,
        req: &api_models::payments::ListMethodsForPaymentsRequest,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    ) -> errors::RouterResult<FilteredPaymentMethodsEnabled> {
        let billing_address = payment_intent
            .billing_address
            .clone()
            .and_then(|address| address.into_inner().address);

        let mut response: Vec<hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector> = vec![];

        for payment_method_enabled_details in self.0.payment_methods_enabled {
            filter_payment_methods(
                payment_method_enabled_details,
                req,
                &mut response,
                Some(payment_intent),
                billing_address.as_ref(),
                &state.conf,
            )
            .await?;
        }

        Ok(FilteredPaymentMethodsEnabled(response))
    }
}

// note: v2 type for ListMethodsForPaymentMethodsRequest will not have the installment_payment_enabled field,
#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    payment_method_type_details: hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    req: &api_models::payments::ListMethodsForPaymentsRequest,
    resp: &mut Vec<
        hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    >,
    payment_intent: Option<&storage::PaymentIntent>,
    address: Option<&hyperswitch_domain_models::address::AddressDetails>,
    configs: &settings::Settings<RawSecret>,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    let payment_method = payment_method_type_details.payment_method;
    let mut payment_method_object = payment_method_type_details.payment_methods_enabled.clone();

    // filter based on request parameters
    let request_based_filter =
        filter_recurring_based(&payment_method_object, req.recurring_enabled)
            && filter_amount_based(&payment_method_object, req.amount)
            && filter_card_network_based(
                payment_method_object.card_networks.as_ref(),
                req.card_networks.as_ref(),
                payment_method_object.payment_method_subtype,
            );

    // filter based on payment intent
    let intent_based_filter = if let Some(payment_intent) = payment_intent {
        filter_country_based(address, &payment_method_object)
            && filter_currency_based(
                payment_intent.amount_details.currency,
                &payment_method_object,
            )
            && filter_amount_based(
                &payment_method_object,
                Some(payment_intent.amount_details.calculate_net_amount()),
            )
            && filter_zero_mandate_based(configs, payment_intent, &payment_method_type_details)
            && filter_allowed_payment_method_types_based(
                payment_intent.allowed_payment_method_types.as_ref(),
                payment_method_object.payment_method_subtype,
            )
    } else {
        true
    };

    // filter based on payment method type configuration
    let config_based_filter = filter_config_based(
        configs,
        &payment_method_type_details.connector.to_string(),
        payment_method_object.payment_method_subtype,
        payment_intent,
        &mut payment_method_object.card_networks,
        address.and_then(|inner| inner.country),
        payment_intent.map(|value| value.amount_details.currency),
    );

    // if all filters pass, add the payment method type details to the response
    if request_based_filter && intent_based_filter && config_based_filter {
        resp.push(payment_method_type_details);
    }

    Ok(())
}

// filter based on country supported by payment method type
// return true if the intent's country is null or if the country is in the accepted countries list
fn filter_country_based(
    address: Option<&hyperswitch_domain_models::address::AddressDetails>,
    pm: &common_types::payment_methods::RequestPaymentMethodTypes,
) -> bool {
    address.is_none_or(|address| {
        address.country.as_ref().is_none_or(|country| {
            pm.accepted_countries.as_ref().is_none_or(|ac| match ac {
                common_types::payment_methods::AcceptedCountries::EnableOnly(acc) => {
                    acc.contains(country)
                }
                common_types::payment_methods::AcceptedCountries::DisableOnly(den) => {
                    !den.contains(country)
                }
                common_types::payment_methods::AcceptedCountries::AllAccepted => true,
            })
        })
    })
}

// filter based on currency supported by payment method type
// return true if the intent's currency is null or if the currency is in the accepted currencies list
fn filter_currency_based(
    currency: common_enums::Currency,
    pm: &common_types::payment_methods::RequestPaymentMethodTypes,
) -> bool {
    pm.accepted_currencies.as_ref().is_none_or(|ac| match ac {
        common_types::payment_methods::AcceptedCurrencies::EnableOnly(acc) => {
            acc.contains(&currency)
        }
        common_types::payment_methods::AcceptedCurrencies::DisableOnly(den) => {
            !den.contains(&currency)
        }
        common_types::payment_methods::AcceptedCurrencies::AllAccepted => true,
    })
}

// filter based on payment method type configuration
// return true if the payment method type is in the configuration for the connector
// return true if the configuration is not available for the connector
fn filter_config_based<'a>(
    config: &'a settings::Settings<RawSecret>,
    connector: &'a str,
    payment_method_type: common_enums::PaymentMethodType,
    payment_intent: Option<&storage::PaymentIntent>,
    card_network: &mut Option<Vec<common_enums::CardNetwork>>,
    country: Option<common_enums::CountryAlpha2>,
    currency: Option<common_enums::Currency>,
) -> bool {
    config
        .pm_filters
        .0
        .get(connector)
        .or_else(|| config.pm_filters.0.get("default"))
        .and_then(|inner| match payment_method_type {
            common_enums::PaymentMethodType::Credit | common_enums::PaymentMethodType::Debit => {
                inner
                    .0
                    .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                        payment_method_type,
                    ))
                    .map(|value| filter_config_country_currency_based(value, country, currency))
            }
            payment_method_type => inner
                .0
                .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                    payment_method_type,
                ))
                .map(|value| filter_config_country_currency_based(value, country, currency)),
        })
        .unwrap_or(true)
}

// filter country and currency based on config for payment method type
// return true if the country and currency are in the accepted countries and currencies list
fn filter_config_country_currency_based(
    item: &settings::CurrencyCountryFlowFilter,
    country: Option<common_enums::CountryAlpha2>,
    currency: Option<common_enums::Currency>,
) -> bool {
    let country_condition = item
        .country
        .as_ref()
        .zip(country.as_ref())
        .map(|(lhs, rhs)| lhs.contains(rhs));
    let currency_condition = item
        .currency
        .as_ref()
        .zip(currency)
        .map(|(lhs, rhs)| lhs.contains(&rhs));
    country_condition.unwrap_or(true) && currency_condition.unwrap_or(true)
}

// filter based on recurring enabled parameter of request
// return true if recurring_enabled is null or if it matches the payment method's recurring_enabled
fn filter_recurring_based(
    payment_method: &common_types::payment_methods::RequestPaymentMethodTypes,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.is_none_or(|enabled| payment_method.recurring_enabled == Some(enabled))
}

// filter based on valid amount range of payment method type
// return true if the amount is within the payment method's minimum and maximum amount range
// return true if the amount is null or zero
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

// return true if the intent is a zero mandate intent and the payment method is supported for zero mandates
// return false if the intent is a zero mandate intent and the payment method is not supported for zero mandates
// return true if the intent is not a zero mandate intent
fn filter_zero_mandate_based(
    configs: &settings::Settings<RawSecret>,
    payment_intent: &storage::PaymentIntent,
    payment_method_type_details: &hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
) -> bool {
    if payment_intent.setup_future_usage == common_enums::FutureUsage::OffSession
        && payment_intent.amount_details.calculate_net_amount() == types::MinorUnit::zero()
    {
        configs
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
    } else {
        true
    }
}

// filter based on allowed payment method types
// return true if the allowed types are null or if the payment method type is in the allowed types list
fn filter_allowed_payment_method_types_based(
    allowed_types: Option<&Vec<api_models::enums::PaymentMethodType>>,
    payment_method_type: api_models::enums::PaymentMethodType,
) -> bool {
    allowed_types.is_none_or(|pm| pm.contains(&payment_method_type))
}

// filter based on card networks
// return true if the payment method type's card networks are a subset of the request's card networks
// return true if the card networks are not specified in the request
fn filter_card_network_based(
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
        | common_enums::IntentStatus::CancelledPostCapture
        | common_enums::IntentStatus::Processing
        | common_enums::IntentStatus::RequiresCustomerAction
        | common_enums::IntentStatus::RequiresMerchantAction
        | common_enums::IntentStatus::RequiresCapture
        | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
        | common_enums::IntentStatus::PartiallyCaptured
        | common_enums::IntentStatus::RequiresConfirmation
        | common_enums::IntentStatus::PartiallyCapturedAndCapturable
        | common_enums::IntentStatus::Expired => {
            Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow: "list_payment_methods".to_string(),
                field_name: "status".to_string(),
                current_value: intent_status.to_string(),
                states: ["requires_payment_method".to_string()].join(", "),
            })
        }
    }
}
