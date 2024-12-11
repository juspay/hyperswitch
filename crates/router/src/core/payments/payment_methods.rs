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

    let response = PaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts)
        .perform_filtering()
        .get_required_fields(RequiredFieldsInput::new())
        .perform_surcharge_calculation()
        .generate_response();

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

struct PaymentMethodsEnabledForConnector {
    payment_methods_enabled: common_utils::types::RequestPaymentMethodTypes,
    payment_method: common_enums::PaymentMethod,
    connector: String,
}

struct PaymentMethodsEnabled {
    payment_methods_enabled: Vec<PaymentMethodsEnabledForConnector>,
}

struct FilteredPaymentMethodsEnabled {
    payment_methods_enabled: Vec<PaymentMethodsEnabledForConnector>,
}

struct RequiredFieldsInput {}

impl RequiredFieldsInput {
    fn new() -> Self {
        Self {}
    }
}

impl FilteredPaymentMethodsEnabled {
    fn get_required_fields(
        self,
        input: RequiredFieldsInput,
    ) -> PaymentMethodsEnabledWithRequiredFieldsContainer {
        let required_fields_info = self
            .payment_methods_enabled
            .into_iter()
            .map(
                |payment_methods_enabled| PaymentMethodsEnabledWithRequiredFields {
                    required_field: None,
                    payment_method_subtype: payment_methods_enabled
                        .payment_methods_enabled
                        .payment_method_type,
                },
            )
            .collect();

        PaymentMethodsEnabledWithRequiredFieldsContainer {
            payment_methods_enabled: required_fields_info,
        }
    }
}

struct PaymentMethodsEnabledWithRequiredFields {
    required_field:
        Option<std::collections::HashMap<String, api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
}

struct PaymentMethodsEnabledWithRequiredFieldsContainer {
    payment_methods_enabled: Vec<PaymentMethodsEnabledWithRequiredFields>,
}

struct PaymentMethodsEnabledWithRequiredFieldsAndSurcharge {
    required_field:
        Option<std::collections::HashMap<String, api_models::payment_methods::RequiredFieldInfo>>,
    payment_method_subtype: common_enums::PaymentMethodType,
    surcharge: Option<api_models::payment_methods::SurchargeDetailsResponse>,
}

struct PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
    payment_methods_enabled: Vec<PaymentMethodsEnabledWithRequiredFieldsAndSurcharge>,
}

impl PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
    fn generate_response(self) -> api_models::payments::PaymentMethodListResponse {
        let response_payment_methods = self
            .payment_methods_enabled
            .into_iter()
            .map(|payment_methods_enabled| {
                api_models::payment_methods::ResponsePaymentMethodTypes {
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    required_fields: payment_methods_enabled.required_field,
                    surcharge_details: payment_methods_enabled.surcharge,
                    card_networks: None,
                    bank_names: None,
                }
            })
            .collect();

        api_models::payments::PaymentMethodListResponse {
            payment_methods: response_payment_methods,
            customer_payment_methods: Vec::new(),
        }
    }
}

impl PaymentMethodsEnabledWithRequiredFieldsContainer {
    fn perform_surcharge_calculation(
        self,
    ) -> PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
        let details_with_surcharge = self
            .payment_methods_enabled
            .into_iter()
            .map(
                |payment_methods_enabled| PaymentMethodsEnabledWithRequiredFieldsAndSurcharge {
                    required_field: payment_methods_enabled.required_field,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    surcharge: None,
                },
            )
            .collect();

        PaymentMethodsEnabledWithRequiredFieldsAndSurchargeContainer {
            payment_methods_enabled: details_with_surcharge,
        }
    }
}

impl PaymentMethodsEnabled {
    /// This functions flattens the payment methods enabled from the connector accounts
    /// Retains the connector name and payment method in every flattened element
    fn from_payment_connectors_list(
        payment_connectors: Vec<domain::MerchantConnectorAccount>,
    ) -> Self {
        let payment_methods_enabled_flattened_with_connector = payment_connectors
            .into_iter()
            .map(|connector| {
                (
                    connector.payment_methods_enabled.unwrap_or_default(),
                    connector.connector_name,
                )
            })
            .flat_map(|(payment_method_enabled, connector_name)| {
                payment_method_enabled
                    .into_iter()
                    .flat_map(move |payment_method| {
                        let request_payment_methods_enabled =
                            payment_method.payment_method_types.unwrap_or_default();
                        let length = request_payment_methods_enabled.len();
                        request_payment_methods_enabled.into_iter().zip(
                            std::iter::repeat((
                                connector_name.clone(),
                                payment_method.payment_method,
                            ))
                            .take(length),
                        )
                    })
            })
            .map(
                |(request_payment_methods, (connector_name, payment_method))| {
                    PaymentMethodsEnabledForConnector {
                        payment_methods_enabled: request_payment_methods,
                        connector: connector_name.clone(),
                        payment_method: payment_method,
                    }
                },
            )
            .collect();

        Self {
            payment_methods_enabled: payment_methods_enabled_flattened_with_connector,
        }
    }

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
