use std::str::FromStr;
mod transformers;
use transformers as configure_connector_webhook_flow;
use api_models::{
    admin::{self as admin_types},
    enums as api_enums, routing as routing_types,
};
use common_enums::{MerchantAccountRequestType, MerchantAccountType, OrganizationType};
use common_utils::{
    date_time,
    ext_traits::{AsyncExt, Encode, OptionExt, ValueExt},
    fp_utils, id_type, pii, type_name,
    types::keymanager::{self as km_types, KeyManagerState, ToEncryptable},
};
#[cfg(all(any(feature = "v1", feature = "v2"), feature = "olap"))]
use diesel_models::{business_profile::CardTestingGuardConfig, organization::OrganizationBridge};
use diesel_models::{configs, payment_method};
use error_stack::{report, FutureExt, ResultExt};
use external_services::http_client::client;
use hyperswitch_domain_models::{router_data::ErrorResponse,
    router_request_types::configure_connector_webhook::ConnectorWebhookRegisterData,
    router_response_types::configure_connector_webhook::ConnectorWebhookRegisterResponse,
    merchant_connector_account::{
    FromRequestEncryptableMerchantConnectorAccount, UpdateEncryptableMerchantConnectorAccount,
}};
use masking::{ExposeInterface, PeekInterface, Secret};
use pm_auth::types as pm_auth_types;
use uuid::Uuid;

use super::routing::helpers::redact_cgraph_cache;
#[cfg(any(feature = "v1", feature = "v2"))]
use crate::types::transformers::ForeignFrom;
use crate::{
    consts,
    core::{
        connector_validation::ConnectorAuthTypeAndMetadataValidation,
        disputes,
        encryption::transfer_encryption_key,
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::helpers,
        pm_auth::helpers::PaymentAuthConnectorDataExt,
        routing, utils as core_utils,
    },
    db::{AccountsStorageInterface, StorageInterface},
    logger,
    routes::{metrics, SessionState},
    services::{
        self,
        api::{self as service_api},
        authentication, pm_auth as payment_initiation_service,
    },
    types::{
        self,
        api::{self, admin},
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage::{self, enums::MerchantStorageScheme},
        transformers::{ForeignInto, ForeignTryFrom, ForeignTryInto},
    },
    utils,
};

#[cfg(feature = "v1")]
pub async fn register_connector_webhook(
    state: SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: Option<id_type::ProfileId>,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
    req: api_models::admin::ConnectorWebhookRegisterRequest,
) -> RouterResponse<api_models::admin::RegisterConnectorWebhookResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let key_store =  db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(
            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_connector_id.get_string_repr().to_string(),
            },
        )?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &mca)?;

        // validate request

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &mca.connector_name,
        api::GetToken::Connector,
        Some(mca.merchant_connector_id.clone()),
    )?;
    let connector_integration: services::BoxedConnectorWebhookConfigurationInterface<
        api::ConnectorWebhookRegister,
        ConnectorWebhookRegisterData,
        ConnectorWebhookRegisterResponse,
    > = connector_data.connector.get_connector_integration();

    let flow_specific_request_data = configure_connector_webhook_flow::construct_webhook_register_request_data(
        &state,
        mca,
        req,
    ).await?;

    let auth_type = mca
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(types::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: mca.merchant_id.clone(),
        customer_id: None,
        connector_customer: None,
        connector: mca.connector_name.clone(),
        payment_id: consts::IRRELEVANT_PAYMENT_INTENT_ID.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: consts::IRRELEVANT_PAYMENT_ATTEMPT_ID.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: types::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: mca.get_metadata().clone(),
        connector_wallets_details: mca.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: flow_specific_request_data,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id:consts::IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    });

    // Call connector to perform the operation

    // Handle the operation result and update db

    // generate and return the response the response
    let response  = api_models::admin::RegisterConnectorWebhookResponse {
    };

    Ok(service_api::ApplicationResponse::Json(response)) 
}
