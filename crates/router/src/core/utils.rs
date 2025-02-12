use std::{collections::HashSet, marker::PhantomData, str::FromStr};

use api_models::enums::{DisputeStage, DisputeStatus};
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutVendorAccountDetails;
use common_enums::{IntentStatus, RequestIncrementalAuthorization};
#[cfg(feature = "payouts")]
use common_utils::{crypto::Encryptable, pii::Email};
use common_utils::{
    errors::CustomResult,
    ext_traits::AsyncExt,
    types::{keymanager::KeyManagerState, ConnectorTransactionIdTrait, MinorUnit},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    merchant_connector_account::MerchantConnectorAccount, payment_address::PaymentAddress,
    router_data::ErrorResponse, router_request_types, types::OrderDetailsWithAmount,
};
#[cfg(feature = "payouts")]
use masking::{ExposeInterface, PeekInterface};
use maud::{html, PreEscaped};
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::payments::helpers;
#[cfg(feature = "payouts")]
use super::payouts::{helpers as payout_helpers, PayoutData};
#[cfg(feature = "payouts")]
use crate::core::payments;
use crate::{
    configs::Settings,
    consts,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::PaymentData,
    },
    db::StorageInterface,
    routes::SessionState,
    types::{
        self, api, domain,
        storage::{self, enums},
        PollConfig,
    },
    utils::{generate_id, generate_uuid, OptionExt, ValueExt},
};

pub const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_dispute_flow";
#[cfg(feature = "payouts")]
pub const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_PAYOUTS_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_payouts_flow";
const IRRELEVANT_ATTEMPT_ID_IN_DISPUTE_FLOW: &str = "irrelevant_attempt_id_in_dispute_flow";

#[cfg(all(feature = "payouts", feature = "v2", feature = "customer_v2"))]
#[instrument(skip_all)]
pub async fn construct_payout_router_data<'a, F>(
    _state: &SessionState,
    _connector_data: &api::ConnectorData,
    _merchant_account: &domain::MerchantAccount,
    _payout_data: &mut PayoutData,
) -> RouterResult<types::PayoutsRouterData<F>> {
    todo!()
}

#[cfg(all(
    feature = "payouts",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
#[instrument(skip_all)]
pub async fn construct_payout_router_data<'a, F>(
    state: &SessionState,
    connector_data: &api::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    payout_data: &mut PayoutData,
) -> RouterResult<types::PayoutsRouterData<F>> {
    let merchant_connector_account = payout_data
        .merchant_connector_account
        .clone()
        .get_required_value("merchant_connector_account")?;
    let connector_name = connector_data.connector_name;
    let connector_auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let billing = payout_data.billing_address.to_owned();

    let billing_address = billing.map(|a| {
        let phone_details = api_models::payments::PhoneDetails {
            number: a.phone_number.clone().map(Encryptable::into_inner),
            country_code: a.country_code.to_owned(),
        };
        let address_details = api_models::payments::AddressDetails {
            city: a.city.to_owned(),
            country: a.country.to_owned(),
            line1: a.line1.clone().map(Encryptable::into_inner),
            line2: a.line2.clone().map(Encryptable::into_inner),
            line3: a.line3.clone().map(Encryptable::into_inner),
            zip: a.zip.clone().map(Encryptable::into_inner),
            first_name: a.first_name.clone().map(Encryptable::into_inner),
            last_name: a.last_name.clone().map(Encryptable::into_inner),
            state: a.state.map(Encryptable::into_inner),
        };

        api_models::payments::Address {
            phone: Some(phone_details),
            address: Some(address_details),
            email: a.email.to_owned().map(Email::from),
        }
    });

    let address = PaymentAddress::new(None, billing_address.map(From::from), None, None);

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let payouts = &payout_data.payouts;
    let payout_attempt = &payout_data.payout_attempt;
    let customer_details = &payout_data.customer_details;
    let connector_label = format!(
        "{}_{}",
        payout_data.profile_id.get_string_repr(),
        connector_name
    );
    let connector_customer_id = customer_details
        .as_ref()
        .and_then(|c| c.connector_customer.as_ref())
        .and_then(|connector_customer_value| {
            connector_customer_value
                .clone()
                .expose()
                .get(connector_label)
                .cloned()
        })
        .and_then(|id| serde_json::from_value::<String>(id).ok());

    let vendor_details: Option<PayoutVendorAccountDetails> =
        match api_models::enums::PayoutConnectors::try_from(connector_name.to_owned()).map_err(
            |err| report!(errors::ApiErrorResponse::InternalServerError).attach_printable(err),
        )? {
            api_models::enums::PayoutConnectors::Stripe => {
                payout_data.payouts.metadata.to_owned().and_then(|meta| {
                    let val = meta
                        .peek()
                        .to_owned()
                        .parse_value("PayoutVendorAccountDetails")
                        .ok();
                    val
                })
            }
            _ => None,
        };

    let connector_transfer_method_id =
        payout_helpers::should_create_connector_transfer_method(&*payout_data, connector_data)?;

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().to_owned(),
        customer_id: customer_details.to_owned().map(|c| c.customer_id),
        tenant_id: state.tenant.tenant_id.clone(),
        connector_customer: connector_customer_id,
        connector: connector_name.to_string(),
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("payout")
            .get_string_repr()
            .to_owned(),
        attempt_id: "".to_string(),
        status: enums::AttemptStatus::Failure,
        payment_method: enums::PaymentMethod::default(),
        connector_auth_type,
        description: None,
        address,
        auth_type: enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        payment_method_status: None,
        request: types::PayoutsData {
            payout_id: payouts.payout_id.to_owned(),
            amount: payouts.amount.get_amount_as_i64(),
            minor_amount: payouts.amount,
            connector_payout_id: payout_attempt.connector_payout_id.clone(),
            destination_currency: payouts.destination_currency,
            source_currency: payouts.source_currency,
            entity_type: payouts.entity_type.to_owned(),
            payout_type: payouts.payout_type,
            vendor_details,
            priority: payouts.priority,
            customer_details: customer_details
                .to_owned()
                .map(|c| payments::CustomerDetails {
                    customer_id: Some(c.customer_id),
                    name: c.name.map(Encryptable::into_inner),
                    email: c.email.map(Email::from),
                    phone: c.phone.map(Encryptable::into_inner),
                    phone_country_code: c.phone_country_code,
                }),
            connector_transfer_method_id,
        },
        response: Ok(types::PayoutsResponseData::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: payout_attempt.payout_attempt_id.clone(),
        payout_method_data: payout_data.payout_method_data.to_owned(),
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_refund_router_data<'a, F>(
    _state: &'a SessionState,
    _connector_id: &str,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    _money: (MinorUnit, enums::Currency),
    _payment_intent: &'a storage::PaymentIntent,
    _payment_attempt: &storage::PaymentAttempt,
    _refund: &'a storage::Refund,
    _creds_identifier: Option<String>,
    _split_refunds: Option<router_request_types::SplitRefundsRequest>,
) -> RouterResult<types::RefundsRouterData<F>> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_refund_router_data<'a, F>(
    state: &'a SessionState,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    money: (MinorUnit, enums::Currency),
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    refund: &'a storage::Refund,
    creds_identifier: Option<String>,
    split_refunds: Option<router_request_types::SplitRefundsRequest>,
) -> RouterResult<types::RefundsRouterData<F>> {
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        creds_identifier.as_deref(),
        key_store,
        profile_id,
        connector_id,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let status = payment_attempt.status;

    let (payment_amount, currency) = money;

    let payment_method_type = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let merchant_connector_account_id_or_connector_name = payment_attempt
        .merchant_connector_id
        .as_ref()
        .map(|mca_id| mca_id.get_string_repr())
        .unwrap_or(connector_id);

    let webhook_url = Some(helpers::create_webhook_url(
        &state.base_url.clone(),
        merchant_account.get_id(),
        merchant_connector_account_id_or_connector_name,
    ));
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;
    let connector_enum = api_models::enums::Connector::from_str(connector_id)
        .change_context(errors::ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_id:?}"))?;

    let connector_api_version = if supported_connector.contains(&connector_enum) {
        state
            .store
            .find_config_by_key(&format!("connector_api_version_{connector_id}"))
            .await
            .map(|value| value.config)
            .ok()
    } else {
        None
    };

    let browser_info: Option<types::BrowserInformation> = payment_attempt
        .browser_info
        .clone()
        .map(|b| b.parse_value("BrowserInformation"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "browser_info",
        })?;

    let connector_refund_id = refund.get_optional_connector_refund_id().cloned();

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        customer_id: payment_intent.customer_id.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status,
        payment_method: payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        // Does refund need shipping/billing address ?
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        payment_method_status: None,
        minor_amount_captured: payment_intent.amount_captured,
        request: types::RefundsData {
            refund_id: refund.refund_id.clone(),
            connector_transaction_id: refund.get_connector_transaction_id().clone(),
            refund_amount: refund.refund_amount.get_amount_as_i64(),
            minor_refund_amount: refund.refund_amount,
            currency,
            payment_amount: payment_amount.get_amount_as_i64(),
            minor_payment_amount: payment_amount,
            webhook_url,
            connector_metadata: payment_attempt.connector_metadata.clone(),
            reason: refund.refund_reason.clone(),
            connector_refund_id: connector_refund_id.clone(),
            browser_info,
            split_refunds,
            integrity_object: None,
            refund_status: refund.refund_status,
        },

        response: Ok(types::RefundsResponseData {
            connector_refund_id: connector_refund_id.unwrap_or_default(),
            refund_status: refund.refund_status,
        }),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: refund.refund_id.clone(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: Some(refund.refund_id.clone()),
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

pub fn get_or_generate_id(
    key: &str,
    provided_id: &Option<String>,
    prefix: &str,
) -> Result<String, errors::ApiErrorResponse> {
    let validate_id = |id| validate_id(id, key);
    provided_id
        .clone()
        .map_or(Ok(generate_id(consts::ID_LENGTH, prefix)), validate_id)
}

pub fn get_or_generate_uuid(
    key: &str,
    provided_id: Option<&String>,
) -> Result<String, errors::ApiErrorResponse> {
    let validate_id = |id: String| validate_uuid(id, key);
    provided_id
        .cloned()
        .map_or(Ok(generate_uuid()), validate_id)
}

fn invalid_id_format_error(key: &str) -> errors::ApiErrorResponse {
    errors::ApiErrorResponse::InvalidDataFormat {
        field_name: key.to_string(),
        expected_format: format!(
            "length should be less than {} characters",
            consts::MAX_ID_LENGTH
        ),
    }
}

pub fn validate_id(id: String, key: &str) -> Result<String, errors::ApiErrorResponse> {
    if id.len() > consts::MAX_ID_LENGTH {
        Err(invalid_id_format_error(key))
    } else {
        Ok(id)
    }
}

pub fn validate_uuid(uuid: String, key: &str) -> Result<String, errors::ApiErrorResponse> {
    match (Uuid::parse_str(&uuid), uuid.len() > consts::MAX_ID_LENGTH) {
        (Ok(_), false) => Ok(uuid),
        (_, _) => Err(invalid_id_format_error(key)),
    }
}

#[cfg(feature = "v1")]
pub fn get_split_refunds(
    split_refund_input: super::refunds::transformers::SplitRefundInput,
) -> RouterResult<Option<router_request_types::SplitRefundsRequest>> {
    match split_refund_input.split_payment_request.as_ref() {
        Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(stripe_payment)) => {
            let (charge_id_option, charge_type_option) = match (
                &split_refund_input.payment_charges,
                &split_refund_input.split_payment_request,
            ) {
                (
                    Some(common_types::payments::ConnectorChargeResponseData::StripeSplitPayment(
                        stripe_split_payment_response,
                    )),
                    _,
                ) => (
                    stripe_split_payment_response.charge_id.clone(),
                    Some(stripe_split_payment_response.charge_type.clone()),
                ),
                (
                    _,
                    Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
                        stripe_split_payment_request,
                    )),
                ) => (
                    split_refund_input.charge_id,
                    Some(stripe_split_payment_request.charge_type.clone()),
                ),
                (_, _) => (None, None),
            };

            if let Some(charge_id) = charge_id_option {
                let options = super::refunds::validator::validate_stripe_charge_refund(
                    charge_type_option,
                    &split_refund_input.refund_request,
                )?;

                Ok(Some(
                    router_request_types::SplitRefundsRequest::StripeSplitRefund(
                        router_request_types::StripeSplitRefund {
                            charge_id,
                            charge_type: stripe_payment.charge_type.clone(),
                            transfer_account_id: stripe_payment.transfer_account_id.clone(),
                            options,
                        },
                    ),
                ))
            } else {
                Ok(None)
            }
        }
        Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(_)) => {
            match &split_refund_input.payment_charges {
                Some(common_types::payments::ConnectorChargeResponseData::AdyenSplitPayment(
                    adyen_split_payment_response,
                )) => {
                    if let Some(common_types::refunds::SplitRefund::AdyenSplitRefund(
                        split_refund_request,
                    )) = split_refund_input.refund_request.clone()
                    {
                        super::refunds::validator::validate_adyen_charge_refund(
                            adyen_split_payment_response,
                            &split_refund_request,
                        )?;

                        Ok(Some(
                            router_request_types::SplitRefundsRequest::AdyenSplitRefund(
                                split_refund_request,
                            ),
                        ))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn validate_id_length_constraint() {
        let payment_id =
            "abcdefghijlkmnopqrstuvwzyzabcdefghijknlmnopsjkdnfjsknfkjsdnfspoig".to_string(); //length = 65

        let result = validate_id(payment_id, "payment_id");
        assert!(result.is_err());
    }

    #[test]
    fn validate_id_proper_response() {
        let payment_id = "abcdefghijlkmnopqrstjhbjhjhkhbhgcxdfxvmhb".to_string();

        let result = validate_id(payment_id.clone(), "payment_id");
        assert!(result.is_ok());
        let result = result.unwrap_or_default();
        assert_eq!(result, payment_id);
    }

    #[test]
    fn test_generate_id() {
        let generated_id = generate_id(consts::ID_LENGTH, "ref");
        assert_eq!(generated_id.len(), consts::ID_LENGTH + 4)
    }

    #[test]
    fn test_filter_objects_based_on_profile_id_list() {
        #[derive(PartialEq, Debug, Clone)]
        struct Object {
            profile_id: Option<common_utils::id_type::ProfileId>,
        }

        impl Object {
            pub fn new(profile_id: &'static str) -> Self {
                Self {
                    profile_id: Some(
                        common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from(
                            profile_id,
                        ))
                        .expect("invalid profile ID"),
                    ),
                }
            }
        }

        impl GetProfileId for Object {
            fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
                self.profile_id.as_ref()
            }
        }

        fn new_profile_id(profile_id: &'static str) -> common_utils::id_type::ProfileId {
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from(profile_id))
                .expect("invalid profile ID")
        }

        // non empty object_list and profile_id_list
        let object_list = vec![
            Object::new("p1"),
            Object::new("p2"),
            Object::new("p2"),
            Object::new("p4"),
            Object::new("p5"),
        ];
        let profile_id_list = vec![
            new_profile_id("p1"),
            new_profile_id("p2"),
            new_profile_id("p3"),
        ];
        let filtered_list =
            filter_objects_based_on_profile_id_list(Some(profile_id_list), object_list.clone());
        let expected_result = vec![Object::new("p1"), Object::new("p2"), Object::new("p2")];
        assert_eq!(filtered_list, expected_result);

        // non empty object_list and empty profile_id_list
        let empty_profile_id_list = vec![];
        let filtered_list = filter_objects_based_on_profile_id_list(
            Some(empty_profile_id_list),
            object_list.clone(),
        );
        let expected_result = vec![];
        assert_eq!(filtered_list, expected_result);

        // non empty object_list and None profile_id_list
        let profile_id_list_as_none = None;
        let filtered_list =
            filter_objects_based_on_profile_id_list(profile_id_list_as_none, object_list);
        let expected_result = vec![
            Object::new("p1"),
            Object::new("p2"),
            Object::new("p2"),
            Object::new("p4"),
            Object::new("p5"),
        ];
        assert_eq!(filtered_list, expected_result);
    }
}

// Dispute Stage can move linearly from PreDispute -> Dispute -> PreArbitration
pub fn validate_dispute_stage(
    prev_dispute_stage: DisputeStage,
    dispute_stage: DisputeStage,
) -> bool {
    match prev_dispute_stage {
        DisputeStage::PreDispute => true,
        DisputeStage::Dispute => !matches!(dispute_stage, DisputeStage::PreDispute),
        DisputeStage::PreArbitration => matches!(dispute_stage, DisputeStage::PreArbitration),
    }
}

//Dispute status can go from Opened -> (Expired | Accepted | Cancelled | Challenged -> (Won | Lost))
pub fn validate_dispute_status(
    prev_dispute_status: DisputeStatus,
    dispute_status: DisputeStatus,
) -> bool {
    match prev_dispute_status {
        DisputeStatus::DisputeOpened => true,
        DisputeStatus::DisputeExpired => {
            matches!(dispute_status, DisputeStatus::DisputeExpired)
        }
        DisputeStatus::DisputeAccepted => {
            matches!(dispute_status, DisputeStatus::DisputeAccepted)
        }
        DisputeStatus::DisputeCancelled => {
            matches!(dispute_status, DisputeStatus::DisputeCancelled)
        }
        DisputeStatus::DisputeChallenged => matches!(
            dispute_status,
            DisputeStatus::DisputeChallenged
                | DisputeStatus::DisputeWon
                | DisputeStatus::DisputeLost
        ),
        DisputeStatus::DisputeWon => matches!(dispute_status, DisputeStatus::DisputeWon),
        DisputeStatus::DisputeLost => matches!(dispute_status, DisputeStatus::DisputeLost),
    }
}

pub fn validate_dispute_stage_and_dispute_status(
    prev_dispute_stage: DisputeStage,
    prev_dispute_status: DisputeStatus,
    dispute_stage: DisputeStage,
    dispute_status: DisputeStatus,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let dispute_stage_validation = validate_dispute_stage(prev_dispute_stage, dispute_stage);
    let dispute_status_validation = if dispute_stage == prev_dispute_stage {
        validate_dispute_status(prev_dispute_status, dispute_status)
    } else {
        true
    };
    common_utils::fp_utils::when(
        !(dispute_stage_validation && dispute_status_validation),
        || {
            super::metrics::INCOMING_DISPUTE_WEBHOOK_VALIDATION_FAILURE_METRIC.add(1, &[]);
            Err(errors::WebhooksFlowError::DisputeWebhookValidationFailed)?
        },
    )
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_accept_dispute_router_data<'a>(
    state: &'a SessionState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
) -> RouterResult<types::AcceptDisputeRouterData> {
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        None,
        key_store,
        &profile_id,
        &dispute.connector,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let payment_method = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: dispute.connector.to_string(),
        tenant_id: state.tenant.tenant_id.clone(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_intent.amount_captured,
        payment_method_status: None,
        request: types::AcceptDisputeRequestData {
            dispute_id: dispute.dispute_id.clone(),
            connector_dispute_id: dispute.connector_dispute_id.clone(),
        },
        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            payment_attempt,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: Some(dispute.dispute_id.clone()),
        refund_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_submit_evidence_router_data<'a>(
    state: &'a SessionState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
    submit_evidence_request_data: types::SubmitEvidenceRequestData,
) -> RouterResult<types::SubmitEvidenceRouterData> {
    let connector_id = &dispute.connector;
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        None,
        key_store,
        &profile_id,
        connector_id,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let payment_method = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_intent.amount_captured,
        request: submit_evidence_request_data,
        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        payment_method_status: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            payment_attempt,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: Some(dispute.dispute_id.clone()),
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_upload_file_router_data<'a>(
    state: &'a SessionState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    create_file_request: &api::CreateFileRequest,
    connector_id: &str,
    file_key: String,
) -> RouterResult<types::UploadFileRouterData> {
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        None,
        key_store,
        &profile_id,
        connector_id,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let payment_method = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_intent.amount_captured,
        payment_method_status: None,
        request: types::UploadFileRequestData {
            file_key,
            file: create_file_request.file.clone(),
            file_type: create_file_request.file_type.clone(),
            file_size: create_file_request.file_size,
        },
        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            payment_attempt,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v2")]
pub async fn construct_payments_dynamic_tax_calculation_router_data<F: Clone>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    merchant_connector_account: &MerchantConnectorAccount,
) -> RouterResult<types::PaymentsTaxCalculationRouterData> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn construct_payments_dynamic_tax_calculation_router_data<F: Clone>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    merchant_connector_account: &MerchantConnectorAccount,
) -> RouterResult<types::PaymentsTaxCalculationRouterData> {
    let payment_intent = &payment_data.payment_intent.clone();
    let payment_attempt = &payment_data.payment_attempt.clone();

    #[cfg(feature = "v1")]
    let test_mode: Option<bool> = merchant_connector_account.test_mode;

    #[cfg(feature = "v2")]
    let test_mode = None;

    let connector_auth_type: types::ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let shipping_address = payment_data
        .tax_data
        .clone()
        .map(|tax_data| tax_data.shipping_details)
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Missing shipping_details")?;

    let order_details: Option<Vec<OrderDetailsWithAmount>> = payment_intent
        .order_details
        .clone()
        .map(|order_details| {
            order_details
                .iter()
                .map(|data| {
                    data.to_owned()
                        .parse_value("OrderDetailsWithAmount")
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "OrderDetailsWithAmount",
                        })
                        .attach_printable("Unable to parse OrderDetailsWithAmount")
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().to_owned(),
        customer_id: None,
        connector_customer: None,
        connector: merchant_connector_account.connector_name.clone(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        attempt_id: payment_attempt.attempt_id.clone(),
        tenant_id: state.tenant.tenant_id.clone(),
        status: payment_attempt.status,
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type,
        description: None,
        address: payment_data.address.clone(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: types::PaymentsTaxCalculationData {
            amount: payment_intent.amount,
            shipping_cost: payment_intent.shipping_cost,
            order_details,
            currency: payment_data.currency,
            shipping_address,
        },
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            payment_attempt,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        payment_method_status: None,
        minor_amount_captured: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_defend_dispute_router_data<'a>(
    state: &'a SessionState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
) -> RouterResult<types::DefendDisputeRouterData> {
    let _db = &*state.store;
    let connector_id = &dispute.connector;
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        None,
        key_store,
        &profile_id,
        connector_id,
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let payment_method = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_intent.amount_captured,
        payment_method_status: None,
        request: types::DefendDisputeRequestData {
            dispute_id: dispute.dispute_id.clone(),
            connector_dispute_id: dispute.connector_dispute_id.clone(),
        },
        response: Err(ErrorResponse::get_not_implemented()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        customer_id: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            payment_attempt,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: Some(dispute.dispute_id.clone()),
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[instrument(skip_all)]
pub async fn construct_retrieve_file_router_data<'a>(
    state: &'a SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    file_metadata: &diesel_models::file::FileMetadata,
    connector_id: &str,
) -> RouterResult<types::RetrieveFileRouterData> {
    let profile_id = file_metadata
        .profile_id
        .as_ref()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in file_metadata")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.get_id(),
        None,
        key_store,
        profile_id,
        connector_id,
        file_metadata.merchant_connector_id.as_ref(),
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: connector_id.to_string(),
        tenant_id: state.tenant.tenant_id.clone(),
        customer_id: None,
        connector_customer: None,
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("dispute")
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_DISPUTE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        payment_method_status: None,
        request: types::RetrieveFileRequestData {
            provider_file_id: file_metadata
                .provider_file_id
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing provider file id")?,
        },
        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_request_reference_id: IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW
            .to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

pub fn is_merchant_enabled_for_payment_id_as_connector_request_id(
    conf: &Settings,
    merchant_id: &common_utils::id_type::MerchantId,
) -> bool {
    let config_map = &conf
        .connector_request_reference_id_config
        .merchant_ids_send_payment_id_as_connector_request_id;
    config_map.contains(merchant_id)
}

#[cfg(feature = "v1")]
pub fn get_connector_request_reference_id(
    conf: &Settings,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
) -> String {
    let is_config_enabled_for_merchant =
        is_merchant_enabled_for_payment_id_as_connector_request_id(conf, merchant_id);
    // Send payment_id if config is enabled for a merchant, else send attempt_id
    if is_config_enabled_for_merchant {
        payment_attempt.payment_id.get_string_repr().to_owned()
    } else {
        payment_attempt.attempt_id.to_owned()
    }
}

// TODO: Based on the connector configuration, the connector_request_reference_id should be generated
#[cfg(feature = "v2")]
pub fn get_connector_request_reference_id(
    conf: &Settings,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
) -> String {
    todo!()
}

/// Validate whether the profile_id exists and is associated with the merchant_id
pub async fn validate_and_get_business_profile(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    profile_id: Option<&common_utils::id_type::ProfileId>,
    merchant_id: &common_utils::id_type::MerchantId,
) -> RouterResult<Option<domain::Profile>> {
    profile_id
        .async_map(|profile_id| async {
            db.find_business_profile_by_profile_id(
                key_manager_state,
                merchant_key_store,
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })
        })
        .await
        .transpose()?
        .map(|business_profile| {
            // Check if the merchant_id of business profile is same as the current merchant_id
            if business_profile.merchant_id.ne(merchant_id) {
                Err(errors::ApiErrorResponse::AccessForbidden {
                    resource: business_profile.get_id().get_string_repr().to_owned(),
                }
                .into())
            } else {
                Ok(business_profile)
            }
        })
        .transpose()
}

fn connector_needs_business_sub_label(connector_name: &str) -> bool {
    let connectors_list = [api_models::enums::Connector::Cybersource];
    connectors_list
        .map(|connector| connector.to_string())
        .contains(&connector_name.to_string())
}

/// Create the connector label
/// {connector_name}_{country}_{business_label}
pub fn get_connector_label(
    business_country: Option<api_models::enums::CountryAlpha2>,
    business_label: Option<&String>,
    business_sub_label: Option<&String>,
    connector_name: &str,
) -> Option<String> {
    business_country
        .zip(business_label)
        .map(|(business_country, business_label)| {
            let mut connector_label =
                format!("{connector_name}_{business_country}_{business_label}");

            // Business sub label is currently being used only for cybersource
            // To ensure backwards compatibality, cybersource mca's created before this change
            // will have the business_sub_label value as default.
            //
            // Even when creating the connector account, if no sub label is provided, default will be used
            if connector_needs_business_sub_label(connector_name) {
                if let Some(sub_label) = business_sub_label {
                    connector_label.push_str(&format!("_{sub_label}"));
                } else {
                    connector_label.push_str("_default"); // For backwards compatibality
                }
            }

            connector_label
        })
}

#[cfg(feature = "v1")]
/// If profile_id is not passed, use default profile if available, or
/// If business_details (business_country and business_label) are passed, get the business_profile
/// or return a `MissingRequiredField` error
#[allow(clippy::too_many_arguments)]
pub async fn get_profile_id_from_business_details(
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    business_country: Option<api_models::enums::CountryAlpha2>,
    business_label: Option<&String>,
    merchant_account: &domain::MerchantAccount,
    request_profile_id: Option<&common_utils::id_type::ProfileId>,
    db: &dyn StorageInterface,
    should_validate: bool,
) -> RouterResult<common_utils::id_type::ProfileId> {
    match request_profile_id.or(merchant_account.default_profile.as_ref()) {
        Some(profile_id) => {
            // Check whether this business profile belongs to the merchant
            if should_validate {
                let _ = validate_and_get_business_profile(
                    db,
                    key_manager_state,
                    merchant_key_store,
                    Some(profile_id),
                    merchant_account.get_id(),
                )
                .await?;
            }
            Ok(profile_id.clone())
        }
        None => match business_country.zip(business_label) {
            Some((business_country, business_label)) => {
                let profile_name = format!("{business_country}_{business_label}");
                let business_profile = db
                    .find_business_profile_by_profile_name_merchant_id(
                        key_manager_state,
                        merchant_key_store,
                        &profile_name,
                        merchant_account.get_id(),
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                        id: profile_name,
                    })?;

                Ok(business_profile.get_id().to_owned())
            }
            _ => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "profile_id or business_country, business_label"
            })),
        },
    }
}

pub fn get_poll_id(merchant_id: &common_utils::id_type::MerchantId, unique_id: String) -> String {
    merchant_id.get_poll_id(&unique_id)
}

pub fn get_external_authentication_request_poll_id(
    payment_id: &common_utils::id_type::PaymentId,
) -> String {
    payment_id.get_external_authentication_request_poll_id()
}

#[cfg(feature = "v1")]
pub fn get_html_redirect_response_for_external_authentication(
    return_url_with_query_params: String,
    payment_response: &api_models::payments::PaymentsResponse,
    payment_id: common_utils::id_type::PaymentId,
    poll_config: &PollConfig,
) -> RouterResult<String> {
    // if intent_status is requires_customer_action then set poll_id, fetch poll config and do a poll_status post message, else do open_url post message to redirect to return_url
    let html = match payment_response.status {
            IntentStatus::RequiresCustomerAction => {
                // Request poll id sent to client for retrieve_poll_status api
                let req_poll_id = get_external_authentication_request_poll_id(&payment_id);
                let poll_frequency = poll_config.frequency;
                let poll_delay_in_secs = poll_config.delay_in_secs;
                html! {
                    head {
                        title { "Redirect Form" }
                        (PreEscaped(format!(r#"
                                <script>
                                    let return_url = "{return_url_with_query_params}";
                                    let poll_status_data = {{
                                        'poll_id': '{req_poll_id}',
                                        'frequency': '{poll_frequency}',
                                        'delay_in_secs': '{poll_delay_in_secs}',
                                        'return_url_with_query_params': return_url
                                    }};
                                    try {{
                                        // if inside iframe, send post message to parent for redirection
                                        if (window.self !== window.parent) {{
                                            window.parent.postMessage({{poll_status: poll_status_data}}, '*')
                                        // if parent, redirect self to return_url
                                        }} else {{
                                            window.location.href = return_url
                                        }}
                                    }}
                                    catch(err) {{
                                        // if error occurs, send post message to parent and wait for 10 secs to redirect. if doesn't redirect, redirect self to return_url
                                        window.parent.postMessage({{poll_status: poll_status_data}}, '*')
                                        setTimeout(function() {{
                                            window.location.href = return_url
                                        }}, 10000);
                                        console.log(err.message)
                                    }}
                                </script>
                                "#)))
                    }
                }
                .into_string()
            },
            _ => {
                html! {
                    head {
                        title { "Redirect Form" }
                        (PreEscaped(format!(r#"
                                <script>
                                    let return_url = "{return_url_with_query_params}";
                                    try {{
                                        // if inside iframe, send post message to parent for redirection
                                        if (window.self !== window.parent) {{
                                            window.parent.postMessage({{openurl_if_required: return_url}}, '*')
                                        // if parent, redirect self to return_url
                                        }} else {{
                                            window.location.href = return_url
                                        }}
                                    }}
                                    catch(err) {{
                                        // if error occurs, send post message to parent and wait for 10 secs to redirect. if doesn't redirect, redirect self to return_url
                                        window.parent.postMessage({{openurl_if_required: return_url}}, '*')
                                        setTimeout(function() {{
                                            window.location.href = return_url
                                        }}, 10000);
                                        console.log(err.message)
                                    }}
                                </script>
                                "#)))
                    }
                }
                .into_string()
            },
        };
    Ok(html)
}

#[inline]
pub fn get_flow_name<F>() -> RouterResult<String> {
    Ok(std::any::type_name::<F>()
        .to_string()
        .rsplit("::")
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Flow stringify failed")?
        .to_string())
}

pub fn get_request_incremental_authorization_value(
    request_incremental_authorization: Option<bool>,
    capture_method: Option<common_enums::CaptureMethod>,
) -> RouterResult<Option<RequestIncrementalAuthorization>> {
    Some(request_incremental_authorization
        .map(|request_incremental_authorization| {
            if request_incremental_authorization {
                if matches!(
                    capture_method,
                    Some(common_enums::CaptureMethod::Automatic) | Some(common_enums::CaptureMethod::SequentialAutomatic)
                ) {
                    Err(errors::ApiErrorResponse::NotSupported { message: "incremental authorization is not supported when capture_method is automatic".to_owned() })?
                }
                Ok(RequestIncrementalAuthorization::True)
            } else {
                Ok(RequestIncrementalAuthorization::False)
            }
        })
        .unwrap_or(Ok(RequestIncrementalAuthorization::default()))).transpose()
}

pub fn get_incremental_authorization_allowed_value(
    incremental_authorization_allowed: Option<bool>,
    request_incremental_authorization: Option<RequestIncrementalAuthorization>,
) -> Option<bool> {
    if request_incremental_authorization == Some(RequestIncrementalAuthorization::False) {
        Some(false)
    } else {
        incremental_authorization_allowed
    }
}

pub(crate) trait GetProfileId {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId>;
}

impl GetProfileId for MerchantConnectorAccount {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(&self.profile_id)
    }
}

impl GetProfileId for storage::PaymentIntent {
    #[cfg(feature = "v1")]
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.profile_id.as_ref()
    }

    // TODO: handle this in a better way for v2
    #[cfg(feature = "v2")]
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(&self.profile_id)
    }
}

impl<A> GetProfileId for (storage::PaymentIntent, A) {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.0.get_profile_id()
    }
}

impl GetProfileId for diesel_models::Dispute {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.profile_id.as_ref()
    }
}

impl GetProfileId for diesel_models::Refund {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.profile_id.as_ref()
    }
}

#[cfg(feature = "v1")]
impl GetProfileId for api_models::routing::RoutingConfigRequest {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.profile_id.as_ref()
    }
}

#[cfg(feature = "v2")]
impl GetProfileId for api_models::routing::RoutingConfigRequest {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(&self.profile_id)
    }
}

impl GetProfileId for api_models::routing::RoutingRetrieveLinkQuery {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.profile_id.as_ref()
    }
}

impl GetProfileId for diesel_models::routing_algorithm::RoutingProfileMetadata {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(&self.profile_id)
    }
}

impl GetProfileId for domain::Profile {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(self.get_id())
    }
}

#[cfg(feature = "payouts")]
impl GetProfileId for storage::Payouts {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        Some(&self.profile_id)
    }
}
#[cfg(feature = "payouts")]
impl<T, F, R> GetProfileId for (storage::Payouts, T, F, R) {
    fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
        self.0.get_profile_id()
    }
}

/// Filter Objects based on profile ids
pub(super) fn filter_objects_based_on_profile_id_list<T: GetProfileId>(
    profile_id_list_auth_layer: Option<Vec<common_utils::id_type::ProfileId>>,
    object_list: Vec<T>,
) -> Vec<T> {
    if let Some(profile_id_list) = profile_id_list_auth_layer {
        let profile_ids_to_filter: HashSet<_> = profile_id_list.iter().collect();
        object_list
            .into_iter()
            .filter_map(|item| {
                if item
                    .get_profile_id()
                    .is_some_and(|profile_id| profile_ids_to_filter.contains(profile_id))
                {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    } else {
        object_list
    }
}

pub(crate) fn validate_profile_id_from_auth_layer<T: GetProfileId + std::fmt::Debug>(
    profile_id_auth_layer: Option<common_utils::id_type::ProfileId>,
    object: &T,
) -> RouterResult<()> {
    match (profile_id_auth_layer, object.get_profile_id()) {
        (Some(auth_profile_id), Some(object_profile_id)) => {
            auth_profile_id.eq(object_profile_id).then_some(()).ok_or(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Profile id authentication failed. Please use the correct JWT token"
                        .to_string(),
                }
                .into(),
            )
        }
        (Some(_auth_profile_id), None) => RouterResult::Err(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Couldn't find profile_id in record for authentication".to_string(),
            }
            .into(),
        )
        .attach_printable(format!("Couldn't find profile_id in entity {:?}", object)),
        (None, None) | (None, Some(_)) => Ok(()),
    }
}
