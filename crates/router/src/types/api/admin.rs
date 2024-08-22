use std::collections::HashMap;

pub use api_models::{
    admin::{
        BusinessProfileCreate, BusinessProfileResponse, BusinessProfileUpdate,
        MerchantAccountCreate, MerchantAccountDeleteResponse, MerchantAccountResponse,
        MerchantAccountUpdate, MerchantConnectorCreate, MerchantConnectorDeleteResponse,
        MerchantConnectorDetails, MerchantConnectorDetailsWrap, MerchantConnectorId,
        MerchantConnectorResponse, MerchantDetails, MerchantId, PaymentMethodsEnabled,
        ToggleAllKVRequest, ToggleAllKVResponse, ToggleKVRequest, ToggleKVResponse, WebhookDetails,
    },
    organization::{OrganizationId, OrganizationRequest, OrganizationResponse},
};
use common_utils::ext_traits::ValueExt;
use diesel_models::organization::OrganizationBridge;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use masking::{ExposeInterface, Secret};

use crate::{
    core::errors,
    routes::SessionState,
    types::{
        domain,
        transformers::{ForeignInto, ForeignTryFrom},
        ForeignFrom,
    },
};

impl ForeignFrom<diesel_models::organization::Organization> for OrganizationResponse {
    fn foreign_from(org: diesel_models::organization::Organization) -> Self {
        Self {
            organization_id: org.get_organization_id(),
            organization_name: org.get_organization_name(),
            organization_details: org.organization_details,
            metadata: org.metadata,
            modified_at: org.modified_at,
            created_at: org.created_at,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
impl ForeignTryFrom<domain::MerchantAccount> for MerchantAccountResponse {
    type Error = error_stack::Report<errors::ParsingError>;
    fn foreign_try_from(item: domain::MerchantAccount) -> Result<Self, Self::Error> {
        let merchant_id = item.get_id().to_owned();
        let primary_business_details: Vec<api_models::admin::PrimaryBusinessDetails> = item
            .primary_business_details
            .parse_value("primary_business_details")?;

        let pm_collect_link_config: Option<api_models::admin::BusinessCollectLinkConfig> = item
            .pm_collect_link_config
            .map(|config| config.parse_value("pm_collect_link_config"))
            .transpose()?;

        Ok(Self {
            merchant_id,
            merchant_name: item.merchant_name,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            merchant_details: item.merchant_details,
            webhook_details: item.webhook_details.clone().map(ForeignInto::foreign_into),
            routing_algorithm: item.routing_algorithm,
            sub_merchants_enabled: item.sub_merchants_enabled,
            parent_merchant_id: item.parent_merchant_id,
            publishable_key: Some(item.publishable_key),
            metadata: item.metadata,
            locker_id: item.locker_id,
            primary_business_details,
            frm_routing_algorithm: item.frm_routing_algorithm,
            #[cfg(feature = "payouts")]
            payout_routing_algorithm: item.payout_routing_algorithm,
            organization_id: item.organization_id,
            is_recon_enabled: item.is_recon_enabled,
            default_profile: item.default_profile,
            recon_status: item.recon_status,
            pm_collect_link_config,
        })
    }
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
impl ForeignTryFrom<domain::MerchantAccount> for MerchantAccountResponse {
    type Error = error_stack::Report<errors::ValidationError>;
    fn foreign_try_from(item: domain::MerchantAccount) -> Result<Self, Self::Error> {
        use common_utils::ext_traits::OptionExt;

        let id = item.get_id().to_owned();

        let merchant_name = item
            .merchant_name
            .get_required_value("merchant_name")?
            .into_inner();

        Ok(Self {
            id,
            merchant_name,
            merchant_details: item.merchant_details,
            publishable_key: item.publishable_key,
            metadata: item.metadata,
            organization_id: item.organization_id,
            recon_status: item.recon_status,
        })
    }
}
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
impl ForeignTryFrom<domain::BusinessProfile> for BusinessProfileResponse {
    type Error = error_stack::Report<errors::ParsingError>;

    fn foreign_try_from(item: domain::BusinessProfile) -> Result<Self, Self::Error> {
        let outgoing_webhook_custom_http_headers = item
            .outgoing_webhook_custom_http_headers
            .map(|headers| {
                headers
                    .into_inner()
                    .expose()
                    .parse_value::<HashMap<String, Secret<String>>>(
                        "HashMap<String, Secret<String>>",
                    )
            })
            .transpose()?;

        Ok(Self {
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            profile_name: item.profile_name,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            webhook_details: item.webhook_details.map(ForeignInto::foreign_into),
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            intent_fulfillment_time: item.intent_fulfillment_time,
            frm_routing_algorithm: item.frm_routing_algorithm,
            #[cfg(feature = "payouts")]
            payout_routing_algorithm: item.payout_routing_algorithm,
            applepay_verified_domains: item.applepay_verified_domains,
            payment_link_config: item.payment_link_config.map(ForeignInto::foreign_into),
            session_expiry: item.session_expiry,
            authentication_connector_details: item
                .authentication_connector_details
                .map(ForeignInto::foreign_into),
            payout_link_config: item.payout_link_config.map(ForeignInto::foreign_into),
            use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
            extended_card_info_config: item
                .extended_card_info_config
                .map(|config| config.expose().parse_value("ExtendedCardInfoConfig"))
                .transpose()?,
            collect_shipping_details_from_wallet_connector: item
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: item
                .collect_billing_details_from_wallet_connector,
            is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
            outgoing_webhook_custom_http_headers,
        })
    }
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
impl ForeignTryFrom<domain::BusinessProfile> for BusinessProfileResponse {
    type Error = error_stack::Report<errors::ParsingError>;

    fn foreign_try_from(item: domain::BusinessProfile) -> Result<Self, Self::Error> {
        let outgoing_webhook_custom_http_headers = item
            .outgoing_webhook_custom_http_headers
            .map(|headers| {
                headers
                    .into_inner()
                    .expose()
                    .parse_value::<HashMap<String, Secret<String>>>(
                        "HashMap<String, Secret<String>>",
                    )
            })
            .transpose()?;

        let order_fulfillment_time = item
            .order_fulfillment_time
            .map(api_models::admin::OrderFulfillmentTime::new)
            .transpose()
            .change_context(errors::ParsingError::IntegerOverflow)?;

        Ok(Self {
            merchant_id: item.merchant_id,
            id: item.profile_id,
            profile_name: item.profile_name,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            webhook_details: item.webhook_details.map(ForeignInto::foreign_into),
            metadata: item.metadata,
            applepay_verified_domains: item.applepay_verified_domains,
            payment_link_config: item.payment_link_config.map(ForeignInto::foreign_into),
            session_expiry: item.session_expiry,
            authentication_connector_details: item
                .authentication_connector_details
                .map(ForeignInto::foreign_into),
            payout_link_config: item.payout_link_config.map(ForeignInto::foreign_into),
            use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
            extended_card_info_config: item
                .extended_card_info_config
                .map(|config| config.expose().parse_value("ExtendedCardInfoConfig"))
                .transpose()?,
            collect_shipping_details_from_wallet_connector: item
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: item
                .collect_billing_details_from_wallet_connector,
            is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
            outgoing_webhook_custom_http_headers,
            order_fulfillment_time,
            order_fulfillment_time_origin: item.order_fulfillment_time_origin,
        })
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(any(feature = "merchant_account_v2", feature = "business_profile_v2"))
))]
pub async fn create_business_profile_from_merchant_account(
    state: &SessionState,
    merchant_account: domain::MerchantAccount,
    request: BusinessProfileCreate,
    key_store: &MerchantKeyStore,
) -> Result<domain::BusinessProfile, error_stack::Report<errors::ApiErrorResponse>> {
    use common_utils::ext_traits::AsyncExt;

    use crate::core;

    // Generate a unique profile id
    let profile_id = common_utils::generate_id_with_default_len("pro");
    let merchant_id = merchant_account.get_id().to_owned();

    let current_time = common_utils::date_time::now();

    let webhook_details = request.webhook_details.map(ForeignInto::foreign_into);

    let payment_response_hash_key = request
        .payment_response_hash_key
        .or(merchant_account.payment_response_hash_key)
        .unwrap_or(common_utils::crypto::generate_cryptographically_secure_random_string(64));

    let payment_link_config = request.payment_link_config.map(ForeignInto::foreign_into);
    let outgoing_webhook_custom_http_headers = request
        .outgoing_webhook_custom_http_headers
        .async_map(|headers| {
            core::payment_methods::cards::create_encrypted_data(state, key_store, headers)
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt outgoing webhook custom HTTP headers")?;

    let payout_link_config = request
        .payout_link_config
        .map(|payout_conf| match payout_conf.config.validate() {
            Ok(_) => Ok(payout_conf.foreign_into()),
            Err(e) => Err(error_stack::report!(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: e.to_string()
                }
            )),
        })
        .transpose()?;

    Ok(domain::BusinessProfile {
        profile_id,
        merchant_id,
        profile_name: request.profile_name.unwrap_or("default".to_string()),
        created_at: current_time,
        modified_at: current_time,
        return_url: request
            .return_url
            .map(|return_url| return_url.to_string())
            .or(merchant_account.return_url),
        enable_payment_response_hash: request
            .enable_payment_response_hash
            .unwrap_or(merchant_account.enable_payment_response_hash),
        payment_response_hash_key: Some(payment_response_hash_key),
        redirect_to_merchant_with_http_post: request
            .redirect_to_merchant_with_http_post
            .unwrap_or(merchant_account.redirect_to_merchant_with_http_post),
        webhook_details: webhook_details.or(merchant_account.webhook_details),
        metadata: request.metadata,
        routing_algorithm: None,
        intent_fulfillment_time: request
            .intent_fulfillment_time
            .map(i64::from)
            .or(merchant_account.intent_fulfillment_time)
            .or(Some(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME)),
        frm_routing_algorithm: request
            .frm_routing_algorithm
            .or(merchant_account.frm_routing_algorithm),
        #[cfg(feature = "payouts")]
        payout_routing_algorithm: request
            .payout_routing_algorithm
            .or(merchant_account.payout_routing_algorithm),
        #[cfg(not(feature = "payouts"))]
        payout_routing_algorithm: None,
        is_recon_enabled: merchant_account.is_recon_enabled,
        applepay_verified_domains: request.applepay_verified_domains,
        payment_link_config,
        session_expiry: request
            .session_expiry
            .map(i64::from)
            .or(Some(common_utils::consts::DEFAULT_SESSION_EXPIRY)),
        authentication_connector_details: request
            .authentication_connector_details
            .map(ForeignInto::foreign_into),
        payout_link_config,
        is_connector_agnostic_mit_enabled: request.is_connector_agnostic_mit_enabled,
        is_extended_card_info_enabled: None,
        extended_card_info_config: None,
        use_billing_as_payment_method_billing: request
            .use_billing_as_payment_method_billing
            .or(Some(true)),
        collect_shipping_details_from_wallet_connector: request
            .collect_shipping_details_from_wallet_connector
            .or(Some(false)),
        collect_billing_details_from_wallet_connector: request
            .collect_billing_details_from_wallet_connector
            .or(Some(false)),
        outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers.map(Into::into),
    })
}
