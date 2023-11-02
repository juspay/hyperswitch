use std::{marker::PhantomData, str::FromStr};

use api_models::enums::{DisputeStage, DisputeStatus};
#[cfg(feature = "payouts")]
use common_utils::{crypto::Encryptable, pii::Email};
use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::payments::{helpers, PaymentAddress};
#[cfg(feature = "payouts")]
use super::payouts::PayoutData;
#[cfg(feature = "payouts")]
use crate::core::payments;
use crate::{
    configs::settings,
    consts,
    core::errors::{self, RouterResult, StorageErrorExt},
    db::StorageInterface,
    routes::AppState,
    types::{
        self, domain,
        storage::{self, enums},
        ErrorResponse,
    },
    utils::{generate_id, generate_uuid, OptionExt, ValueExt},
};

pub const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_dispute_flow";
const IRRELEVANT_PAYMENT_ID_IN_DISPUTE_FLOW: &str = "irrelevant_payment_id_in_dispute_flow";
const IRRELEVANT_ATTEMPT_ID_IN_DISPUTE_FLOW: &str = "irrelevant_attempt_id_in_dispute_flow";

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn get_mca_for_payout<'a>(
    state: &'a AppState,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payout_data: &PayoutData,
) -> RouterResult<(helpers::MerchantConnectorAccountType, String)> {
    let payout_attempt = &payout_data.payout_attempt;
    let profile_id = get_profile_id_from_business_details(
        payout_attempt.business_country,
        payout_attempt.business_label.as_ref(),
        merchant_account,
        payout_attempt.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payout_attempt")?;
    match payout_data.merchant_connector_account.to_owned() {
        Some(mca) => Ok((mca, profile_id)),
        None => {
            let merchant_connector_account = helpers::get_merchant_connector_account(
                state,
                merchant_account.merchant_id.as_str(),
                None,
                key_store,
                &profile_id,
                connector_id,
                payout_attempt.merchant_connector_id.as_ref(),
            )
            .await?;
            Ok((merchant_connector_account, profile_id))
        }
    }
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
pub async fn construct_payout_router_data<'a, F>(
    state: &'a AppState,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    _request: &api_models::payouts::PayoutRequest,
    payout_data: &mut PayoutData,
) -> RouterResult<types::PayoutsRouterData<F>> {
    let (merchant_connector_account, profile_id) = get_mca_for_payout(
        state,
        connector_id,
        merchant_account,
        key_store,
        payout_data,
    )
    .await?;
    payout_data.merchant_connector_account = Some(merchant_connector_account.clone());
    let connector_auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let billing = payout_data.billing_address.to_owned();

    let address = PaymentAddress {
        shipping: None,
        billing: billing.map(|a| {
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
            }
        }),
    };

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let payouts = &payout_data.payouts;
    let payout_attempt = &payout_data.payout_attempt;
    let customer_details = &payout_data.customer_details;
    let connector_label = format!("{profile_id}_{}", payout_attempt.connector);
    let connector_customer_id = customer_details
        .as_ref()
        .and_then(|c| c.connector_customer.as_ref())
        .and_then(|cc| cc.get(connector_label))
        .and_then(|id| serde_json::from_value::<String>(id.to_owned()).ok());
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.to_owned(),
        customer_id: None,
        connector_customer: connector_customer_id,
        connector: connector_id.to_string(),
        payment_id: "".to_string(),
        attempt_id: "".to_string(),
        status: enums::AttemptStatus::Failure,
        payment_method: enums::PaymentMethod::default(),
        connector_auth_type,
        description: None,
        return_url: payouts.return_url.to_owned(),
        payment_method_id: None,
        address,
        auth_type: enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: None,
        request: types::PayoutsData {
            payout_id: payouts.payout_id.to_owned(),
            amount: payouts.amount,
            connector_payout_id: Some(payout_attempt.connector_payout_id.to_owned()),
            destination_currency: payouts.destination_currency,
            source_currency: payouts.source_currency,
            entity_type: payouts.entity_type.to_owned(),
            payout_type: payouts.payout_type,
            customer_details: customer_details
                .to_owned()
                .map(|c| payments::CustomerDetails {
                    customer_id: Some(c.customer_id),
                    name: c.name.map(Encryptable::into_inner),
                    email: c.email.map(Email::from),
                    phone: c.phone.map(Encryptable::into_inner),
                    phone_country_code: c.phone_country_code,
                }),
        },
        response: Ok(types::PayoutsResponseData::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW
            .to_string(),
        payout_method_data: payout_data.payout_method_data.to_owned(),
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
    };

    Ok(router_data)
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_refund_router_data<'a, F>(
    state: &'a AppState,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    money: (i64, enums::Currency),
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    refund: &'a storage::Refund,
    creds_identifier: Option<String>,
) -> RouterResult<types::RefundsRouterData<F>> {
    let profile_id = get_profile_id_from_business_details(
        payment_intent.business_country,
        payment_intent.business_label.as_ref(),
        merchant_account,
        payment_intent.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        creds_identifier,
        key_store,
        &profile_id,
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

    let webhook_url = Some(helpers::create_webhook_url(
        &state.conf.server.base_url.clone(),
        &merchant_account.merchant_id,
        &connector_id.to_string(),
    ));
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;
    let connector_enum = api_models::enums::Connector::from_str(connector_id)
        .into_report()
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

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        customer_id: payment_intent.customer_id.to_owned(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status,
        payment_method: payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        payment_method_id: payment_attempt.payment_method_id.clone(),
        // Does refund need shipping/billing address ?
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: payment_intent.amount_captured,
        request: types::RefundsData {
            refund_id: refund.refund_id.clone(),
            connector_transaction_id: refund.connector_transaction_id.clone(),
            refund_amount: refund.refund_amount,
            currency,
            payment_amount,
            webhook_url,
            connector_metadata: payment_attempt.connector_metadata.clone(),
            reason: refund.refund_reason.clone(),
            connector_refund_id: refund.connector_refund_id.clone(),
            browser_info,
        },

        response: Ok(types::RefundsResponseData {
            connector_refund_id: refund.connector_refund_id.clone().unwrap_or_default(),
            refund_status: refund.refund_status,
        }),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
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

#[cfg(test)]
mod tests {
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
}

// Dispute Stage can move linearly from PreDispute -> Dispute -> PreArbitration
pub fn validate_dispute_stage(
    prev_dispute_stage: &DisputeStage,
    dispute_stage: &DisputeStage,
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
    let dispute_stage_validation = validate_dispute_stage(&prev_dispute_stage, &dispute_stage);
    let dispute_status_validation = if dispute_stage == prev_dispute_stage {
        validate_dispute_status(prev_dispute_status, dispute_status)
    } else {
        true
    };
    common_utils::fp_utils::when(
        !(dispute_stage_validation && dispute_status_validation),
        || {
            super::metrics::INCOMING_DISPUTE_WEBHOOK_VALIDATION_FAILURE_METRIC.add(
                &super::metrics::CONTEXT,
                1,
                &[],
            );
            Err(errors::WebhooksFlowError::DisputeWebhookValidationFailed)?
        },
    )
}

#[instrument(skip_all)]
pub async fn construct_accept_dispute_router_data<'a>(
    state: &'a AppState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
) -> RouterResult<types::AcceptDisputeRouterData> {
    let profile_id = get_profile_id_from_business_details(
        payment_intent.business_country,
        payment_intent.business_label.as_ref(),
        merchant_account,
        payment_intent.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
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
        merchant_id: merchant_account.merchant_id.clone(),
        connector: dispute.connector.to_string(),
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        payment_method_id: payment_attempt.payment_method_id.clone(),
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: payment_intent.amount_captured,
        request: types::AcceptDisputeRequestData {
            dispute_id: dispute.dispute_id.clone(),
            connector_dispute_id: dispute.connector_dispute_id.clone(),
        },
        response: Err(types::ErrorResponse::default()),
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
            &merchant_account.merchant_id,
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
    };
    Ok(router_data)
}

#[instrument(skip_all)]
pub async fn construct_submit_evidence_router_data<'a>(
    state: &'a AppState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
    submit_evidence_request_data: types::SubmitEvidenceRequestData,
) -> RouterResult<types::SubmitEvidenceRouterData> {
    let connector_id = &dispute.connector;
    let profile_id = get_profile_id_from_business_details(
        payment_intent.business_country,
        payment_intent.business_label.as_ref(),
        merchant_account,
        payment_intent.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
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
        merchant_id: merchant_account.merchant_id.clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        payment_method_id: payment_attempt.payment_method_id.clone(),
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: payment_intent.amount_captured,
        request: submit_evidence_request_data,
        response: Err(types::ErrorResponse::default()),
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
            &merchant_account.merchant_id,
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
    };
    Ok(router_data)
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_upload_file_router_data<'a>(
    state: &'a AppState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    create_file_request: &types::api::CreateFileRequest,
    connector_id: &str,
    file_key: String,
) -> RouterResult<types::UploadFileRouterData> {
    let profile_id = get_profile_id_from_business_details(
        payment_intent.business_country,
        payment_intent.business_label.as_ref(),
        merchant_account,
        payment_intent.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
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
        merchant_id: merchant_account.merchant_id.clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        payment_method_id: payment_attempt.payment_method_id.clone(),
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: payment_intent.amount_captured,
        request: types::UploadFileRequestData {
            file_key,
            file: create_file_request.file.clone(),
            file_type: create_file_request.file_type.clone(),
            file_size: create_file_request.file_size,
        },
        response: Err(types::ErrorResponse::default()),
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
            &merchant_account.merchant_id,
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
    };
    Ok(router_data)
}

#[instrument(skip_all)]
pub async fn construct_defend_dispute_router_data<'a>(
    state: &'a AppState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    dispute: &storage::Dispute,
) -> RouterResult<types::DefendDisputeRouterData> {
    let _db = &*state.store;
    let connector_id = &dispute.connector;
    let profile_id = get_profile_id_from_business_details(
        payment_intent.business_country,
        payment_intent.business_label.as_ref(),
        merchant_account,
        payment_intent.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
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
        merchant_id: merchant_account.merchant_id.clone(),
        connector: connector_id.to_string(),
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        payment_method_id: payment_attempt.payment_method_id.clone(),
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: payment_intent.amount_captured,
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
            &merchant_account.merchant_id,
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
    };
    Ok(router_data)
}

#[instrument(skip_all)]
pub async fn construct_retrieve_file_router_data<'a>(
    state: &'a AppState,
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
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in file_metadata")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
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
        merchant_id: merchant_account.merchant_id.clone(),
        connector: connector_id.to_string(),
        customer_id: None,
        connector_customer: None,
        payment_id: IRRELEVANT_PAYMENT_ID_IN_DISPUTE_FLOW.to_string(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_DISPUTE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        payment_method_id: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: None,
        request: types::RetrieveFileRequestData {
            provider_file_id: file_metadata
                .provider_file_id
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Missing provider file id")?,
        },
        response: Err(types::ErrorResponse::default()),
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
    };
    Ok(router_data)
}

pub fn is_merchant_enabled_for_payment_id_as_connector_request_id(
    conf: &settings::Settings,
    merchant_id: &str,
) -> bool {
    let config_map = &conf
        .connector_request_reference_id_config
        .merchant_ids_send_payment_id_as_connector_request_id;
    config_map.contains(merchant_id)
}

pub fn get_connector_request_reference_id(
    conf: &settings::Settings,
    merchant_id: &str,
    payment_attempt: &data_models::payments::payment_attempt::PaymentAttempt,
) -> String {
    let is_config_enabled_for_merchant =
        is_merchant_enabled_for_payment_id_as_connector_request_id(conf, merchant_id);
    // Send payment_id if config is enabled for a merchant, else send attempt_id
    if is_config_enabled_for_merchant {
        payment_attempt.payment_id.clone()
    } else {
        payment_attempt.attempt_id.clone()
    }
}

/// Validate whether the profile_id exists and is associated with the merchant_id
pub async fn validate_and_get_business_profile(
    db: &dyn StorageInterface,
    profile_id: Option<&String>,
    merchant_id: &str,
) -> RouterResult<Option<storage::business_profile::BusinessProfile>> {
    profile_id
        .async_map(|profile_id| async {
            db.find_business_profile_by_profile_id(profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: profile_id.to_owned(),
                })
        })
        .await
        .transpose()?
        .map(|business_profile| {
            // Check if the merchant_id of business profile is same as the current merchant_id
            if business_profile.merchant_id.ne(merchant_id) {
                Err(errors::ApiErrorResponse::AccessForbidden {
                    resource: business_profile.profile_id,
                })
            } else {
                Ok(business_profile)
            }
        })
        .transpose()
        .into_report()
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

/// If profile_id is not passed, use default profile if available, or
/// If business_details (business_country and business_label) are passed, get the business_profile
/// or return a `MissingRequiredField` error
pub async fn get_profile_id_from_business_details(
    business_country: Option<api_models::enums::CountryAlpha2>,
    business_label: Option<&String>,
    merchant_account: &domain::MerchantAccount,
    request_profile_id: Option<&String>,
    db: &dyn StorageInterface,
    should_validate: bool,
) -> RouterResult<String> {
    match request_profile_id.or(merchant_account.default_profile.as_ref()) {
        Some(profile_id) => {
            // Check whether this business profile belongs to the merchant
            if should_validate {
                let _ = validate_and_get_business_profile(
                    db,
                    Some(profile_id),
                    &merchant_account.merchant_id,
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
                        &profile_name,
                        &merchant_account.merchant_id,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                        id: profile_name,
                    })?;

                Ok(business_profile.profile_id)
            }
            _ => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "profile_id or business_country, business_label"
            })),
        },
    }
}

#[inline]
pub fn get_flow_name<F>() -> RouterResult<String> {
    Ok(std::any::type_name::<F>()
        .to_string()
        .rsplit("::")
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Flow stringify failed")?
        .to_string())
}
