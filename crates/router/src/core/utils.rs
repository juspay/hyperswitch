use std::marker::PhantomData;

use api_models::enums::{DisputeStage, DisputeStatus};
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::payments::{helpers, PaymentAddress};
use crate::{
    configs::settings,
    consts,
    core::errors::{self, RouterResult},
    routes::AppState,
    types::{
        self, domain,
        storage::{self, enums},
        ErrorResponse,
    },
    utils::{generate_id, OptionExt, ValueExt},
};

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_dispute_flow";
const IRRELEVANT_PAYMENT_ID_IN_DISPUTE_FLOW: &str = "irrelevant_payment_id_in_dispute_flow";
const IRRELEVANT_ATTEMPT_ID_IN_DISPUTE_FLOW: &str = "irrelevant_attempt_id_in_dispute_flow";

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
    let connector_label = helpers::get_connector_label(
        payment_intent.business_country,
        &payment_intent.business_label,
        None,
        connector_id,
    );

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        creds_identifier,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
        test_mode,
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
    let connector_id = &dispute.connector;
    let connector_label = helpers::get_connector_label(
        payment_intent.business_country,
        &payment_intent.business_label,
        payment_attempt.business_sub_label.as_ref(),
        connector_id,
    );
    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        None,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
        test_mode,
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
    let connector_label = helpers::get_connector_label(
        payment_intent.business_country,
        &payment_intent.business_label,
        payment_attempt.business_sub_label.as_ref(),
        connector_id,
    );
    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        None,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
        test_mode,
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
    connector_label: String,
) -> RouterResult<types::UploadFileRouterData> {
    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        None,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
        test_mode,
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
    let connector_label = helpers::get_connector_label(
        payment_intent.business_country,
        &payment_intent.business_label,
        payment_attempt.business_sub_label.as_ref(),
        connector_id,
    );
    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        None,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            payment_attempt,
        ),
        test_mode,
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
    let connector_label = file_metadata
        .connector_label
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Missing connector label")?;
    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        None,
        key_store,
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
        preprocessing_id: None,
        connector_request_reference_id: IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_DISPUTE_FLOW
            .to_string(),
        test_mode,
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
    config_map.contains(&merchant_id.to_string())
}

pub fn get_connector_request_reference_id(
    conf: &settings::Settings,
    merchant_id: &str,
    payment_attempt: &diesel_models::payment_attempt::PaymentAttempt,
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
