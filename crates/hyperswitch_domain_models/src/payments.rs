#[cfg(feature = "v2")]
use std::marker::PhantomData;

#[cfg(feature = "v2")]
use api_models::payments::Address;
use common_utils::{self, crypto::Encryptable, id_type, pii, types::MinorUnit};
use diesel_models::payment_intent::TaxDetails;
#[cfg(feature = "v2")]
use error_stack::ResultExt;
use masking::Secret;
use time::PrimitiveDateTime;

pub mod payment_attempt;
pub mod payment_intent;

use common_enums as storage_enums;
#[cfg(feature = "v2")]
use diesel_models::types::{FeatureMetadata, OrderDetailsWithAmount};

use self::payment_attempt::PaymentAttempt;
use crate::RemoteStorageObject;
#[cfg(feature = "v2")]
use crate::{business_profile, merchant_account};
#[cfg(feature = "v2")]
use crate::{errors, payment_method_data, ApiModelToDieselModelConvertor};

#[cfg(feature = "v1")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentIntent {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: MinorUnit,
    pub shipping_cost: Option<MinorUnit>,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<id_type::CustomerId>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<id_type::ProfileId>,
    pub payment_link_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: Option<storage_enums::RequestIncrementalAuthorization>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub fingerprint_id: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub charges: Option<pii::SecretSerdeValue>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub is_payment_processor_token_flow: Option<bool>,
    pub organization_id: id_type::OrganizationId,
    pub tax_details: Option<TaxDetails>,
    pub skip_external_tax_calculation: Option<bool>,
}

impl PaymentIntent {
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &id_type::PaymentId {
        &self.payment_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &id_type::GlobalPaymentId {
        &self.id
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, Copy, serde::Serialize)]
pub enum TaxCalculationOverride {
    /// Skip calling the external tax provider
    Skip,
    /// Calculate tax by calling the external tax provider
    Calculate,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, Copy, serde::Serialize)]
pub enum SurchargeCalculationOverride {
    /// Skip calculating surcharge
    Skip,
    /// Calculate surcharge
    Calculate,
}

#[cfg(feature = "v2")]
impl From<Option<bool>> for TaxCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Calculate,
            _ => Self::Skip,
        }
    }
}

#[cfg(feature = "v2")]
impl From<Option<bool>> for SurchargeCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Calculate,
            _ => Self::Skip,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AmountDetails {
    /// The amount of the order in the lowest denomination of currency
    pub order_amount: MinorUnit,
    /// The currency of the order
    pub currency: common_enums::Currency,
    /// The shipping cost of the order. This has to be collected from the merchant
    pub shipping_cost: Option<MinorUnit>,
    /// Tax details related to the order. This will be calculated by the external tax provider
    pub tax_details: Option<TaxDetails>,
    /// The action to whether calculate tax by calling external tax provider or not
    pub skip_external_tax_calculation: TaxCalculationOverride,
    /// The action to whether calculate surcharge or not
    pub skip_surcharge_calculation: SurchargeCalculationOverride,
    /// The surcharge amount to be added to the order, collected from the merchant
    pub surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    pub tax_on_surcharge: Option<MinorUnit>,
    /// The total amount captured for the order. This is the sum of all the captured amounts for the order.
    /// For automatic captures, this will be the same as net amount for the order
    pub amount_captured: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
impl AmountDetails {
    /// Get the action to whether calculate surcharge or not as a boolean value
    fn get_surcharge_action_as_bool(&self) -> bool {
        match self.skip_surcharge_calculation {
            SurchargeCalculationOverride::Skip => false,
            SurchargeCalculationOverride::Calculate => true,
        }
    }

    /// Get the action to whether calculate external tax or not as a boolean value
    fn get_external_tax_action_as_bool(&self) -> bool {
        match self.skip_external_tax_calculation {
            TaxCalculationOverride::Skip => false,
            TaxCalculationOverride::Calculate => true,
        }
    }

    /// Calculate the net amount for the order
    pub fn calculate_net_amount(&self) -> MinorUnit {
        self.order_amount
            + self.shipping_cost.unwrap_or(MinorUnit::zero())
            + self.surcharge_amount.unwrap_or(MinorUnit::zero())
            + self.tax_on_surcharge.unwrap_or(MinorUnit::zero())
    }

    pub fn create_attempt_amount_details(
        &self,
        confirm_intent_request: &api_models::payments::PaymentsConfirmIntentRequest,
    ) -> payment_attempt::AttemptAmountDetails {
        let net_amount = self.calculate_net_amount();

        let surcharge_amount = match self.skip_surcharge_calculation {
            SurchargeCalculationOverride::Skip => self.surcharge_amount,
            SurchargeCalculationOverride::Calculate => None,
        };

        let tax_on_surcharge = match self.skip_surcharge_calculation {
            SurchargeCalculationOverride::Skip => self.tax_on_surcharge,
            SurchargeCalculationOverride::Calculate => None,
        };

        let order_tax_amount = match self.skip_external_tax_calculation {
            TaxCalculationOverride::Skip => self.tax_details.as_ref().and_then(|tax_details| {
                tax_details.get_tax_amount(confirm_intent_request.payment_method_subtype)
            }),
            TaxCalculationOverride::Calculate => None,
        };

        payment_attempt::AttemptAmountDetails {
            net_amount,
            amount_to_capture: None,
            surcharge_amount,
            tax_on_surcharge,
            // This will be updated when we receive response from the connector
            amount_capturable: MinorUnit::zero(),
            shipping_cost: self.shipping_cost,
            order_tax_amount,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentIntent {
    /// The global identifier for the payment intent. This is generated by the system.
    /// The format of the global id is `{cell_id:5}_pay_{time_ordered_uuid:32}`.
    pub id: id_type::GlobalPaymentId,
    /// The identifier for the merchant. This is automatically derived from the api key used to create the payment.
    pub merchant_id: id_type::MerchantId,
    /// The status of payment intent.
    pub status: storage_enums::IntentStatus,
    /// The amount related details of the payment
    pub amount_details: AmountDetails,
    /// The total amount captured for the order. This is the sum of all the captured amounts for the order.
    pub amount_captured: Option<MinorUnit>,
    /// The identifier for the customer. This is the identifier for the customer in the merchant's system.
    pub customer_id: Option<id_type::CustomerId>,
    /// The description of the order. This will be passed to connectors which support description.
    pub description: Option<common_utils::types::Description>,
    /// The return url for the payment. This is the url to which the user will be redirected after the payment is completed.
    pub return_url: Option<common_utils::types::Url>,
    /// The metadata for the payment intent. This is the metadata that will be passed to the connectors.
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The statement descriptor for the order, this will be displayed in the user's bank statement.
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,
    /// The time at which the order was created
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// The time at which the order was last modified
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: storage_enums::FutureUsage,
    /// The client secret that is generated for the payment. This is used to authenticate the payment from client facing apis.
    pub client_secret: common_utils::types::ClientSecret,
    /// The active attempt for the payment intent. This is the payment attempt that is currently active for the payment intent.
    pub active_attempt: Option<RemoteStorageObject<PaymentAttempt>>,
    /// The order details for the payment.
    pub order_details: Option<Vec<Secret<OrderDetailsWithAmount>>>,
    /// This is the list of payment method types that are allowed for the payment intent.
    /// This field allows the merchant to restrict the payment methods that can be used for the payment intent.
    pub allowed_payment_method_types: Option<Vec<common_enums::PaymentMethodType>>,
    /// This metadata contains details about
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub feature_metadata: Option<FeatureMetadata>,
    /// Number of attempts that have been made for the order
    pub attempt_count: i16,
    /// The profile id for the payment.
    pub profile_id: id_type::ProfileId,
    /// The payment link id for the payment. This is generated only if `enable_payment_link` is set to true.
    pub payment_link_id: Option<String>,
    /// This Denotes the action(approve or reject) taken by merchant in case of manual review.
    /// Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub frm_merchant_decision: Option<common_enums::MerchantDecision>,
    /// Denotes the last instance which updated the payment
    pub updated_by: String,
    /// Denotes whether merchant requested for incremental authorization to be enabled for this payment.
    pub request_incremental_authorization: storage_enums::RequestIncrementalAuthorization,
    /// Denotes the number of authorizations that have been made for the payment.
    pub authorization_count: Option<i32>,
    /// Denotes the client secret expiry for the payment. This is the time at which the client secret will expire.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: PrimitiveDateTime,
    /// Denotes whether merchant requested for 3ds authentication to be enabled for this payment.
    pub request_external_three_ds_authentication: common_enums::External3dsAuthenticationRequest,
    /// Metadata related to fraud and risk management
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    /// The details of the customer in a denormalized form. Only a subset of fields are stored.
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    /// The reference id for the order in the merchant's system. This value can be passed by the merchant.
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,
    /// The billing address for the order in a denormalized form.
    pub billing_address: Option<Encryptable<Secret<Address>>>,
    /// The shipping address for the order in a denormalized form.
    pub shipping_address: Option<Encryptable<Secret<Address>>>,
    /// Capture method for the payment
    pub capture_method: storage_enums::CaptureMethod,
    /// Authentication type that is requested by the merchant for this payment.
    pub authentication_type: common_enums::AuthenticationType,
    /// This contains the pre routing results that are done when routing is done during listing the payment methods.
    pub prerouting_algorithm: Option<serde_json::Value>,
    /// The organization id for the payment. This is derived from the merchant account
    pub organization_id: id_type::OrganizationId,
    /// Denotes the request by the merchant whether to enable a payment link for this payment.
    pub enable_payment_link: common_enums::EnablePaymentLinkRequest,
    /// Denotes the request by the merchant whether to apply MIT exemption for this payment
    pub apply_mit_exemption: common_enums::MitExemptionRequest,
    /// Denotes whether the customer is present during the payment flow. This information may be used for 3ds authentication
    pub customer_present: common_enums::PresenceOfCustomerDuringPayment,
    /// Denotes the override for payment link configuration
    pub payment_link_config: Option<diesel_models::PaymentLinkConfigRequestForPayments>,
    /// The straight through routing algorithm id that is used for this payment. This overrides the default routing algorithm that is configured in business profile.
    pub routing_algorithm_id: Option<id_type::RoutingId>,
}

#[cfg(feature = "v2")]
impl PaymentIntent {
    fn get_request_incremental_authorization_value(
        request: &api_models::payments::PaymentsCreateIntentRequest,
    ) -> common_utils::errors::CustomResult<
        common_enums::RequestIncrementalAuthorization,
        errors::api_error_response::ApiErrorResponse,
    > {
        request.request_incremental_authorization
            .map(|request_incremental_authorization| {
                if request_incremental_authorization == common_enums::RequestIncrementalAuthorization::True {
                    if request.capture_method == Some(common_enums::CaptureMethod::Automatic) {
                        Err(errors::api_error_response::ApiErrorResponse::InvalidRequestData { message: "incremental authorization is not supported when capture_method is automatic".to_owned() })?
                    }
                    Ok(common_enums::RequestIncrementalAuthorization::True)
                } else {
                    Ok(common_enums::RequestIncrementalAuthorization::False)
                }
            })
            .unwrap_or(Ok(common_enums::RequestIncrementalAuthorization::default()))
    }

    /// Check if the client secret is associated with the payment and if it has been expired
    pub fn validate_client_secret(
        &self,
        client_secret: &common_utils::types::ClientSecret,
    ) -> Result<(), errors::api_error_response::ApiErrorResponse> {
        common_utils::fp_utils::when(self.client_secret != *client_secret, || {
            Err(errors::api_error_response::ApiErrorResponse::ClientSecretInvalid)
        })?;

        common_utils::fp_utils::when(self.session_expiry < common_utils::date_time::now(), || {
            Err(errors::api_error_response::ApiErrorResponse::ClientSecretExpired)
        })?;

        Ok(())
    }

    pub async fn create_domain_model_from_request(
        payment_id: &id_type::GlobalPaymentId,
        merchant_account: &merchant_account::MerchantAccount,
        profile: &business_profile::Profile,
        request: api_models::payments::PaymentsCreateIntentRequest,
        billing_address: Option<Encryptable<Secret<Address>>>,
        shipping_address: Option<Encryptable<Secret<Address>>>,
    ) -> common_utils::errors::CustomResult<Self, errors::api_error_response::ApiErrorResponse>
    {
        let connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting connector metadata as value")?;
        let request_incremental_authorization =
            Self::get_request_incremental_authorization_value(&request)?;
        let allowed_payment_method_types = request.allowed_payment_method_types;

        let session_expiry =
            common_utils::date_time::now().saturating_add(time::Duration::seconds(
                request.session_expiry.map(i64::from).unwrap_or(
                    profile
                        .session_expiry
                        .unwrap_or(common_utils::consts::DEFAULT_SESSION_EXPIRY),
                ),
            ));
        let client_secret = payment_id.generate_client_secret();
        let order_details = request.order_details.map(|order_details| {
            order_details
                .into_iter()
                .map(|order_detail| Secret::new(OrderDetailsWithAmount::convert_from(order_detail)))
                .collect()
        });
        Ok(Self {
            id: payment_id.clone(),
            merchant_id: merchant_account.get_id().clone(),
            // Intent status would be RequiresPaymentMethod because we are creating a new payment intent
            status: common_enums::IntentStatus::RequiresPaymentMethod,
            amount_details: AmountDetails::from(request.amount_details),
            amount_captured: None,
            customer_id: request.customer_id,
            description: request.description,
            return_url: request.return_url,
            metadata: request.metadata,
            statement_descriptor: request.statement_descriptor,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: request.setup_future_usage.unwrap_or_default(),
            client_secret,
            active_attempt: None,
            order_details,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata: request.feature_metadata.map(FeatureMetadata::convert_from),
            // Attempt count is 0 in create intent as no attempt is made yet
            attempt_count: 0,
            profile_id: profile.get_id().clone(),
            payment_link_id: None,
            frm_merchant_decision: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            request_incremental_authorization,
            // Authorization count is 0 in create intent as no authorization is made yet
            authorization_count: Some(0),
            session_expiry,
            request_external_three_ds_authentication: request
                .request_external_three_ds_authentication
                .unwrap_or_default(),
            frm_metadata: request.frm_metadata,
            customer_details: None,
            merchant_reference_id: request.merchant_reference_id,
            billing_address,
            shipping_address,
            capture_method: request.capture_method.unwrap_or_default(),
            authentication_type: request.authentication_type.unwrap_or_default(),
            prerouting_algorithm: None,
            organization_id: merchant_account.organization_id.clone(),
            enable_payment_link: request.payment_link_enabled.unwrap_or_default(),
            apply_mit_exemption: request.apply_mit_exemption.unwrap_or_default(),
            customer_present: request.customer_present.unwrap_or_default(),
            payment_link_config: request
                .payment_link_config
                .map(ApiModelToDieselModelConvertor::convert_from),
            routing_algorithm_id: request.routing_algorithm_id,
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, Clone)]
pub struct HeaderPayload {
    pub payment_confirm_source: Option<common_enums::PaymentSource>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub x_hs_latency: Option<bool>,
    pub browser_name: Option<common_enums::BrowserName>,
    pub x_client_platform: Option<common_enums::ClientPlatform>,
    pub x_merchant_domain: Option<String>,
    pub locale: Option<String>,
    pub x_app_id: Option<String>,
    pub x_redirect_uri: Option<String>,
}

// TODO: uncomment fields as necessary
#[cfg(feature = "v2")]
#[derive(Default, Debug, Clone)]
pub struct HeaderPayload {
    /// The source with which the payment is confirmed.
    pub payment_confirm_source: Option<common_enums::PaymentSource>,
    // pub client_source: Option<String>,
    // pub client_version: Option<String>,
    pub x_hs_latency: Option<bool>,
    pub browser_name: Option<common_enums::BrowserName>,
    pub x_client_platform: Option<common_enums::ClientPlatform>,
    pub x_merchant_domain: Option<String>,
    pub locale: Option<String>,
    pub x_app_id: Option<String>,
    pub x_redirect_uri: Option<String>,
    pub client_secret: Option<common_utils::types::ClientSecret>,
}

impl HeaderPayload {
    pub fn with_source(payment_confirm_source: common_enums::PaymentSource) -> Self {
        Self {
            payment_confirm_source: Some(payment_confirm_source),
            ..Default::default()
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone)]
pub struct PaymentIntentData<F>
where
    F: Clone,
{
    pub flow: PhantomData<F>,
    pub payment_intent: PaymentIntent,
}

// TODO: Check if this can be merged with existing payment data
#[cfg(feature = "v2")]
#[derive(Clone)]
pub struct PaymentConfirmData<F>
where
    F: Clone,
{
    pub flow: PhantomData<F>,
    pub payment_intent: PaymentIntent,
    pub payment_attempt: PaymentAttempt,
    pub payment_method_data: Option<payment_method_data::PaymentMethodData>,
}
