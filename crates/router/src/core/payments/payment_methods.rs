//! Contains functions of payment methods that are used in payments
//! one of such functions is `list_payment_methods`

use std::collections::{BTreeMap, HashSet};

use common_utils::{ext_traits::OptionExt, id_type};
use error_stack::ResultExt;

use super::errors;
use crate::{
    core::payment_methods, db::errors::StorageErrorExt, logger, routes, settings, types::domain,
};

#[cfg(feature = "v2")]
pub async fn list_payment_methods(
    state: routes::SessionState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    payment_id: id_type::GlobalPaymentId,
    _req: api_models::payments::PaymentMethodsListRequest,
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
    FlattenedPaymentMethodsEnabled(hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts))
            .perform_filtering()
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
