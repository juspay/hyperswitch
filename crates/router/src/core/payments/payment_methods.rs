//! Contains functions of payment methods that are used in payments
//! one of such functions is `list_payment_methods`

use common_utils::{ext_traits::OptionExt, id_type};
use error_stack::ResultExt;

use super::errors;
use crate::{db::errors::StorageErrorExt, routes, types::domain};

#[cfg(all(
    feature = "v2",
    feature = "customer_v2",
    feature = "payment_methods_v2"
))]
pub async fn list_payment_methods(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
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
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    validate_payment_status(payment_intent.status)?;

    let client_secret = header_payload
        .client_secret
        .as_ref()
        .get_required_value("client_secret header")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret header",
        })?;

    payment_intent.validate_client_secret(client_secret)?;

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            key_manager_state,
            profile.get_id(),
            &key_store,
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let response =
        hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts)
            .perform_filtering()
            .get_required_fields(RequiredFieldsInput::new())
            .perform_surcharge_calculation()
            .generate_response();

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

/// Container for the filtered payment methods
struct FilteredPaymentMethodsEnabled {
    payment_methods_enabled: Vec<
        hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    >,
}

impl FilteredPaymentMethodsEnabled {
    fn get_required_fields(
        self,
        _input: RequiredFieldsInput,
    ) -> PaymentMethodTypesEnabledWithRequiredFieldsContainer {
        let required_fields_info = self
            .payment_methods_enabled
            .into_iter()
            .map(
                |payment_methods_enabled| PaymentMethodTypesEnabledWithRequiredFieldsElement {
                    required_field: None,
                    payment_method_type: payment_methods_enabled.payment_method,
                    payment_method_subtype: payment_methods_enabled
                        .payment_methods_enabled
                        .payment_method_subtype,
                },
            )
            .collect();

        PaymentMethodTypesEnabledWithRequiredFieldsContainer {
            payment_methods_enabled: required_fields_info,
        }
    }
}

/// Element container to hold the filtered payment methods with required fields
struct PaymentMethodTypesEnabledWithRequiredFieldsElement {
    required_field:
        Option<std::collections::HashMap<String, api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
}

/// Container to hold the filtered payment methods enabled with required fields
struct PaymentMethodTypesEnabledWithRequiredFieldsContainer {
    payment_methods_enabled: Vec<PaymentMethodTypesEnabledWithRequiredFieldsElement>,
}

/// Element Container to hold the filtered payment methods enabled with required fields and surcharge
struct PaymentMethodTypessEnabledWithRequiredFieldsAndSurcharge {
    required_field:
        Option<std::collections::HashMap<String, api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
    surcharge: Option<api_models::payment_methods::SurchargeDetailsResponse>,
}

/// Container to hold the filtered payment methods enabled with required fields and surcharge
struct PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
    payment_methods_enabled: Vec<PaymentMethodTypessEnabledWithRequiredFieldsAndSurcharge>,
}

impl PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
    fn generate_response(self) -> api_models::payments::PaymentMethodListResponseForPayments {
        let response_payment_methods = self
            .payment_methods_enabled
            .into_iter()
            .map(|payment_methods_enabled| {
                api_models::payment_methods::ResponsePaymentMethodTypes {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    required_fields: payment_methods_enabled.required_field,
                    surcharge_details: payment_methods_enabled.surcharge,
                    card_networks: None,
                    bank_names: None,
                }
            })
            .collect();

        api_models::payments::PaymentMethodListResponseForPayments {
            payment_methods_enabled: response_payment_methods,
            customer_payment_methods: Vec::new(),
        }
    }
}

impl PaymentMethodTypesEnabledWithRequiredFieldsContainer {
    fn perform_surcharge_calculation(
        self,
    ) -> PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
        let details_with_surcharge = self
            .payment_methods_enabled
            .into_iter()
            .map(|payment_methods_enabled| {
                PaymentMethodTypessEnabledWithRequiredFieldsAndSurcharge {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    required_field: payment_methods_enabled.required_field,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    surcharge: None,
                }
            })
            .collect();

        PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
            payment_methods_enabled: details_with_surcharge,
        }
    }
}

trait PerformFilteringOnPaymentMethodsEnabled {
    fn perform_filtering(self) -> FilteredPaymentMethodsEnabled;
}

impl PerformFilteringOnPaymentMethodsEnabled
    for hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled
{
    fn perform_filtering(self) -> FilteredPaymentMethodsEnabled {
        FilteredPaymentMethodsEnabled {
            payment_methods_enabled: self.payment_methods_enabled,
        }
    }
}

/// Validate if payment methods list can be performed on the current status of payment intent
fn validate_payment_status(
    intent_status: common_enums::IntentStatus,
) -> Result<(), errors::ApiErrorResponse> {
    match intent_status {
        common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
        common_enums::IntentStatus::Succeeded
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
