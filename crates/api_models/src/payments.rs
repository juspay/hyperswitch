#[cfg(feature = "v1")]
use std::fmt;
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroI64,
};
pub mod additional_info;
pub mod trait_impls;
use cards::CardNumber;
#[cfg(feature = "v2")]
use common_enums::enums::PaymentConnectorTransmission;
use common_enums::{GooglePayCardFundingSource, ProductType};
#[cfg(feature = "v1")]
use common_types::primitive_wrappers::{
    ExtendedAuthorizationAppliedBool, RequestExtendedAuthorizationBool,
};
use common_types::{payments as common_payments_types, primitive_wrappers};
use common_utils::{
    consts::default_payments_list_limit,
    crypto,
    errors::ValidationError,
    ext_traits::{ConfigExt, Encode, ValueExt},
    hashing::HashedString,
    id_type,
    new_type::MaskedBankAccount,
    pii::{self, Email},
    types::{AmountConvertor, MinorUnit, SemanticVersion, StringMajorUnit},
};
use error_stack::ResultExt;

#[cfg(feature = "v2")]
fn parse_comma_separated<'de, D, T>(v: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug + std::fmt::Display + std::error::Error,
{
    let opt_str: Option<String> = Option::deserialize(v)?;
    match opt_str {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            // Estimate capacity based on comma count
            let capacity = s.matches(',').count() + 1;
            let mut result = Vec::with_capacity(capacity);

            for item in s.split(',') {
                let trimmed_item = item.trim();
                if !trimmed_item.is_empty() {
                    let parsed_item = trimmed_item.parse::<T>().map_err(|e| {
                        <D::Error as serde::de::Error>::custom(format!(
                            "Invalid value '{trimmed_item}': {e}"
                        ))
                    })?;
                    result.push(parsed_item);
                }
            }
            Ok(Some(result))
        }
        None => Ok(None),
    }
}
use masking::{PeekInterface, Secret, WithType};
use router_derive::Setter;
#[cfg(feature = "v1")]
use serde::{de, Deserializer};
use serde::{ser::Serializer, Deserialize, Serialize};
use smithy::SmithyModel;
use strum::Display;
use time::{Date, PrimitiveDateTime};
use url::Url;
use utoipa::ToSchema;

#[cfg(feature = "v2")]
use crate::mandates;
use crate::{
    admin::{self, MerchantConnectorInfo},
    enums as api_enums,
    mandates::RecurringDetails,
    payment_methods,
    payments::additional_info::{
        BankDebitAdditionalData, BankRedirectDetails, BankTransferAdditionalData,
        CardTokenAdditionalData, GiftCardAdditionalData, UpiAdditionalData,
        WalletAdditionalDataForCard,
    },
};
#[cfg(feature = "v1")]
use crate::{disputes, ephemeral_key::EphemeralKeyCreateResponse, refunds, ValidateFieldAndGet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentOp {
    Create,
    Update,
    Confirm,
}

use crate::enums;
#[derive(serde::Deserialize)]
pub struct BankData {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub code_information: Vec<BankCodeInformation>,
}

#[derive(serde::Deserialize)]
pub struct BankCodeInformation {
    pub bank_name: common_enums::BankNames,
    pub connector_codes: Vec<ConnectorCode>,
}

#[derive(serde::Deserialize)]
pub struct ConnectorCode {
    pub connector: api_enums::Connector,
    pub code: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct BankCodeResponse {
    #[schema(value_type = Vec<BankNames>)]
    pub bank_name: Vec<common_enums::BankNames>,
    pub eligible_connectors: Vec<String>,
}

/// Passing this object creates a new customer or attaches an existing customer to the payment
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, PartialEq, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CustomerDetails {
    /// The identifier for the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    #[smithy(value_type = "String")]
    pub id: id_type::CustomerId,

    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub name: Option<Secret<String>>,

    /// The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 10, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer's phone number
    #[schema(max_length = 2, example = "+1")]
    #[smithy(value_type = "Option<String>")]
    pub phone_country_code: Option<String>,

    /// The tax registration identifier of the customer.
    #[schema(value_type=Option<String>,max_length = 255)]
    #[smithy(value_type = "Option<String>")]
    pub tax_registration_id: Option<Secret<String>>,
}

#[cfg(feature = "v1")]
/// Details of customer attached to this payment
#[derive(
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    ToSchema,
    PartialEq,
    Setter,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CustomerDetailsResponse {
    /// The identifier for the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    #[smithy(value_type = "Option<String>")]
    pub id: Option<id_type::CustomerId>,

    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub name: Option<Secret<String>>,

    /// The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 10, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer's phone number
    #[schema(max_length = 2, example = "+1")]
    #[smithy(value_type = "Option<String>")]
    pub phone_country_code: Option<String>,
}

#[cfg(feature = "v2")]
/// Details of customer attached to this payment
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema, PartialEq, Setter)]
pub struct CustomerDetailsResponse {
    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "John Doe")]
    pub name: Option<Secret<String>>,

    /// The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 10, example = "9123456789")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer's phone number
    #[schema(max_length = 2, example = "+1")]
    pub phone_country_code: Option<String>,
}

// Serialize is required because the api event requires Serialize to be implemented
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct PaymentsCreateIntentRequest {
    /// The amount details for the payment
    pub amount_details: AmountDetails,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// The routing algorithm id to be used for the payment
    #[schema(value_type = Option<String>)]
    pub routing_algorithm_id: Option<id_type::RoutingId>,

    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    pub billing: Option<Address>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
    #[schema(example = "present", value_type = Option<PresenceOfCustomerDuringPayment>)]
    pub customer_present: Option<common_enums::PresenceOfCustomerDuringPayment>,

    /// A description for the payment
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Apply MIT exemption for a payment
    #[schema(value_type = Option<MitExemptionRequest>)]
    pub apply_mit_exemption: Option<common_enums::MitExemptionRequest>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 22, example = "Hyperswitch Router", value_type = Option<String>)]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(value_type = Option<EnablePaymentLinkRequest>)]
    pub payment_link_enabled: Option<common_enums::EnablePaymentLinkRequest>,

    /// Configure a custom payment link for the particular payment
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    pub payment_link_config: Option<admin::PaymentLinkConfigRequest>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[schema(value_type = Option<RequestIncrementalAuthorization>)]
    pub request_incremental_authorization: Option<common_enums::RequestIncrementalAuthorization>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds, if not sent it will be taken from profile config
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = Option<External3dsAuthenticationRequest>)]
    pub request_external_three_ds_authentication:
        Option<common_enums::External3dsAuthenticationRequest>,

    /// Indicates if 3ds challenge is forced
    pub force_3ds_challenge: Option<bool>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,
}
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentAttemptListRequest {
    #[schema(value_type = String)]
    pub payment_intent_id: id_type::GlobalPaymentId,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentAttemptListResponse {
    pub payment_attempt_list: Vec<PaymentAttemptResponse>,
}
#[cfg(feature = "v2")]
impl PaymentsCreateIntentRequest {
    pub fn get_feature_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<pii::SecretSerdeValue>,
        common_utils::errors::ParsingError,
    > {
        Ok(self
            .feature_metadata
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()?
            .map(Secret::new))
    }

    pub fn get_connector_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<pii::SecretSerdeValue>,
        common_utils::errors::ParsingError,
    > {
        Ok(self
            .connector_metadata
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()?
            .map(Secret::new))
    }

    pub fn get_allowed_payment_method_types_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<pii::SecretSerdeValue>,
        common_utils::errors::ParsingError,
    > {
        Ok(self
            .allowed_payment_method_types
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()?
            .map(Secret::new))
    }

    pub fn get_order_details_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<Vec<pii::SecretSerdeValue>>,
        common_utils::errors::ParsingError,
    > {
        self.order_details
            .as_ref()
            .map(|od| {
                od.iter()
                    .map(|order| order.encode_to_value().map(Secret::new))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    }
}

// This struct is only used internally, not visible in API Reference
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "v2")]
pub struct PaymentsGetIntentRequest {
    pub id: id_type::GlobalPaymentId,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct PaymentsUpdateIntentRequest {
    pub amount_details: Option<AmountDetailsUpdate>,

    /// The routing algorithm id to be used for the payment
    #[schema(value_type = Option<String>)]
    pub routing_algorithm_id: Option<id_type::RoutingId>,

    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    pub billing: Option<Address>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
    #[schema(example = "present", value_type = Option<PresenceOfCustomerDuringPayment>)]
    pub customer_present: Option<common_enums::PresenceOfCustomerDuringPayment>,

    /// A description for the payment
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Apply MIT exemption for a payment
    #[schema(value_type = Option<MitExemptionRequest>)]
    pub apply_mit_exemption: Option<common_enums::MitExemptionRequest>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 22, example = "Hyperswitch Router", value_type = Option<String>)]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Metadata is useful for storing additional, unstructured information on an object. This metadata will override the metadata that was passed in payments
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[schema(value_type = Option<FeatureMetadata>)]
    pub feature_metadata: Option<FeatureMetadata>,

    /// Configure a custom payment link for the particular payment
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    pub payment_link_config: Option<admin::PaymentLinkConfigRequest>,

    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[schema(value_type = Option<RequestIncrementalAuthorization>)]
    pub request_incremental_authorization: Option<common_enums::RequestIncrementalAuthorization>,

    /// Will be used to expire client secret after certain amount of time to be supplied in seconds, if not sent it will be taken from profile config
    ///(900) for 15 mins
    #[schema(value_type = Option<u32>, example = 900)]
    pub session_expiry: Option<u32>,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = Option<External3dsAuthenticationRequest>)]
    pub request_external_three_ds_authentication:
        Option<common_enums::External3dsAuthenticationRequest>,

    #[schema(value_type = Option<UpdateActiveAttempt>)]
    /// Whether to set / unset the active attempt id
    pub set_active_attempt_id: Option<api_enums::UpdateActiveAttempt>,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,
}

#[cfg(feature = "v2")]
impl PaymentsUpdateIntentRequest {
    pub fn update_feature_metadata_and_active_attempt_with_api(
        feature_metadata: FeatureMetadata,
        set_active_attempt_id: api_enums::UpdateActiveAttempt,
    ) -> Self {
        Self {
            feature_metadata: Some(feature_metadata),
            set_active_attempt_id: Some(set_active_attempt_id),
            amount_details: None,
            routing_algorithm_id: None,
            capture_method: None,
            authentication_type: None,
            billing: None,
            shipping: None,
            customer_present: None,
            description: None,
            return_url: None,
            setup_future_usage: None,
            apply_mit_exemption: None,
            statement_descriptor: None,
            order_details: None,
            allowed_payment_method_types: None,
            metadata: None,
            connector_metadata: None,
            payment_link_config: None,
            request_incremental_authorization: None,
            session_expiry: None,
            frm_metadata: None,
            request_external_three_ds_authentication: None,
            enable_partial_authorization: None,
        }
    }
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct PaymentsIntentResponse {
    /// Global Payment Id for the payment
    #[schema(value_type = String)]
    pub id: id_type::GlobalPaymentId,

    /// The status of the payment
    #[schema(value_type = IntentStatus, example = "succeeded")]
    pub status: common_enums::IntentStatus,

    /// The amount details for the payment
    pub amount_details: AmountDetailsResponse,

    /// It's a token used for client side verification.
    #[schema(value_type = String, example = "cs_0195b34da95d75239c6a4bf514458896")]
    pub client_secret: Option<Secret<String>>,

    /// The identifier for the profile. This is inferred from the `x-profile-id` header
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// The routing algorithm id to be used for the payment
    #[schema(value_type = Option<String>)]
    pub routing_algorithm_id: Option<id_type::RoutingId>,

    #[schema(value_type = CaptureMethod, example = "automatic")]
    pub capture_method: api_enums::CaptureMethod,

    /// The authentication type for the payment
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// The shipping address for the payment
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
    #[schema(example = "present", value_type = PresenceOfCustomerDuringPayment)]
    pub customer_present: common_enums::PresenceOfCustomerDuringPayment,

    /// A description for the payment
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    #[schema(value_type = FutureUsage, example = "off_session")]
    pub setup_future_usage: api_enums::FutureUsage,

    /// Apply MIT exemption for a payment
    #[schema(value_type = MitExemptionRequest)]
    pub apply_mit_exemption: common_enums::MitExemptionRequest,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 22, example = "Hyperswitch Router", value_type = Option<String>)]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[schema(value_type = Option<FeatureMetadata>)]
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(value_type = EnablePaymentLinkRequest)]
    pub payment_link_enabled: common_enums::EnablePaymentLinkRequest,

    /// Configure a custom payment link for the particular payment
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    pub payment_link_config: Option<admin::PaymentLinkConfigRequest>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[schema(value_type = RequestIncrementalAuthorization)]
    pub request_incremental_authorization: common_enums::RequestIncrementalAuthorization,

    /// Enable split payments, i.e., split the amount between multiple payment methods
    #[schema(value_type = SplitTxnsEnabled, default = "skip")]
    pub split_txns_enabled: common_enums::SplitTxnsEnabled,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_on: PrimitiveDateTime,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = External3dsAuthenticationRequest)]
    pub request_external_three_ds_authentication: common_enums::External3dsAuthenticationRequest,

    /// The type of the payment that differentiates between normal and various types of mandate payments
    #[schema(value_type = PaymentType)]
    pub payment_type: api_enums::PaymentType,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct RevenueRecoveryGetIntentResponse {
    /// Global Payment Id for the payment
    #[schema(value_type = String)]
    pub id: id_type::GlobalPaymentId,

    /// The recovery status of the payment
    #[schema(value_type = RecoveryStatus, example = "scheduled")]
    pub status: common_enums::RecoveryStatus,

    /// The amount details for the payment
    pub amount_details: AmountDetailsResponse,

    /// It's a token used for client side verification.
    #[schema(value_type = String, example = "cs_0195b34da95d75239c6a4bf514458896")]
    pub client_secret: Option<Secret<String>>,

    /// The identifier for the profile. This is inferred from the `x-profile-id` header
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// The routing algorithm id to be used for the payment
    #[schema(value_type = Option<String>)]
    pub routing_algorithm_id: Option<id_type::RoutingId>,

    #[schema(value_type = CaptureMethod, example = "automatic")]
    pub capture_method: api_enums::CaptureMethod,

    /// The authentication type for the payment
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// The shipping address for the payment
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
    #[schema(example = "present", value_type = PresenceOfCustomerDuringPayment)]
    pub customer_present: common_enums::PresenceOfCustomerDuringPayment,

    /// A description for the payment
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    #[schema(value_type = FutureUsage, example = "off_session")]
    pub setup_future_usage: api_enums::FutureUsage,

    /// Apply MIT exemption for a payment
    #[schema(value_type = MitExemptionRequest)]
    pub apply_mit_exemption: common_enums::MitExemptionRequest,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 22, example = "Hyperswitch Router", value_type = Option<String>)]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[schema(value_type = Option<FeatureMetadata>)]
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(value_type = EnablePaymentLinkRequest)]
    pub payment_link_enabled: common_enums::EnablePaymentLinkRequest,

    /// Configure a custom payment link for the particular payment
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    pub payment_link_config: Option<admin::PaymentLinkConfigRequest>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[schema(value_type = RequestIncrementalAuthorization)]
    pub request_incremental_authorization: common_enums::RequestIncrementalAuthorization,

    /// Enable split payments, i.e., split the amount between multiple payment methods
    #[schema(value_type = SplitTxnsEnabled, default = "skip")]
    pub split_txns_enabled: common_enums::SplitTxnsEnabled,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_on: PrimitiveDateTime,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = External3dsAuthenticationRequest)]
    pub request_external_three_ds_authentication: common_enums::External3dsAuthenticationRequest,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,

    /// Number of cards/tokens attached to the connector customer in Redis
    #[schema(value_type = u32, example = 2)]
    pub card_attached: u32,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AmountDetails {
    /// The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥ is a zero-decimal currency. Read more about [the Decimal and Non-Decimal Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
    #[schema(value_type = u64, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize")]
    order_amount: Amount,
    /// The currency of the order
    #[schema(example = "USD", value_type = Currency)]
    currency: common_enums::Currency,
    /// The shipping cost of the order. This has to be collected from the merchant
    shipping_cost: Option<MinorUnit>,
    /// Tax amount related to the order. This will be calculated by the external tax provider
    order_tax_amount: Option<MinorUnit>,
    /// The action to whether calculate tax by calling external tax provider or not
    #[serde(default)]
    #[schema(value_type = TaxCalculationOverride)]
    skip_external_tax_calculation: common_enums::TaxCalculationOverride,
    /// The action to whether calculate surcharge or not
    #[serde(default)]
    #[schema(value_type = SurchargeCalculationOverride)]
    skip_surcharge_calculation: common_enums::SurchargeCalculationOverride,
    /// The surcharge amount to be added to the order, collected from the merchant
    surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    tax_on_surcharge: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
impl AmountDetails {
    pub fn new_for_zero_auth_payment(currency: common_enums::Currency) -> Self {
        Self {
            order_amount: Amount::Zero,
            currency,
            shipping_cost: None,
            order_tax_amount: None,
            skip_external_tax_calculation: common_enums::TaxCalculationOverride::Skip,
            skip_surcharge_calculation: common_enums::SurchargeCalculationOverride::Skip,
            surcharge_amount: None,
            tax_on_surcharge: None,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AmountDetailsUpdate {
    /// The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥ is a zero-decimal currency. Read more about [the Decimal and Non-Decimal Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
    #[schema(value_type = Option<u64>, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize_option")]
    order_amount: Option<Amount>,
    /// The currency of the order
    #[schema(example = "USD", value_type = Option<Currency>)]
    currency: Option<common_enums::Currency>,
    /// The shipping cost of the order. This has to be collected from the merchant
    shipping_cost: Option<MinorUnit>,
    /// Tax amount related to the order. This will be calculated by the external tax provider
    order_tax_amount: Option<MinorUnit>,
    /// The action to whether calculate tax by calling external tax provider or not
    #[schema(value_type = Option<TaxCalculationOverride>)]
    skip_external_tax_calculation: Option<common_enums::TaxCalculationOverride>,
    /// The action to whether calculate surcharge or not
    #[schema(value_type = Option<SurchargeCalculationOverride>)]
    skip_surcharge_calculation: Option<common_enums::SurchargeCalculationOverride>,
    /// The surcharge amount to be added to the order, collected from the merchant
    surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    tax_on_surcharge: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
pub struct AmountDetailsSetter {
    pub order_amount: Amount,
    pub currency: common_enums::Currency,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
    pub skip_external_tax_calculation: common_enums::TaxCalculationOverride,
    pub skip_surcharge_calculation: common_enums::SurchargeCalculationOverride,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_on_surcharge: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct AmountDetailsResponse {
    /// The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥ is a zero-decimal currency. Read more about [the Decimal and Non-Decimal Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
    #[schema(value_type = u64, example = 6540)]
    pub order_amount: MinorUnit,
    /// The currency of the order
    #[schema(example = "USD", value_type = Currency)]
    pub currency: common_enums::Currency,
    /// The shipping cost of the order. This has to be collected from the merchant
    pub shipping_cost: Option<MinorUnit>,
    /// Tax amount related to the order. This will be calculated by the external tax provider
    pub order_tax_amount: Option<MinorUnit>,
    /// The action to whether calculate tax by calling external tax provider or not
    #[schema(value_type = TaxCalculationOverride)]
    pub external_tax_calculation: common_enums::TaxCalculationOverride,
    /// The action to whether calculate surcharge or not
    #[schema(value_type = SurchargeCalculationOverride)]
    pub surcharge_calculation: common_enums::SurchargeCalculationOverride,
    /// The surcharge amount to be added to the order, collected from the merchant
    pub surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    pub tax_on_surcharge: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct PaymentAmountDetailsResponse {
    /// The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥ is a zero-decimal currency. Read more about [the Decimal and Non-Decimal Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
    #[schema(value_type = u64, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize")]
    pub order_amount: MinorUnit,
    /// The currency of the order
    #[schema(example = "USD", value_type = Currency)]
    pub currency: common_enums::Currency,
    /// The shipping cost of the order. This has to be collected from the merchant
    pub shipping_cost: Option<MinorUnit>,
    /// Tax amount related to the order. This will be calculated by the external tax provider
    pub order_tax_amount: Option<MinorUnit>,
    /// The action to whether calculate tax by calling external tax provider or not
    #[schema(value_type = TaxCalculationOverride)]
    pub external_tax_calculation: common_enums::TaxCalculationOverride,
    /// The action to whether calculate surcharge or not
    #[schema(value_type = SurchargeCalculationOverride)]
    pub surcharge_calculation: common_enums::SurchargeCalculationOverride,
    /// The surcharge amount to be added to the order, collected from the merchant
    pub surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    pub tax_on_surcharge: Option<MinorUnit>,
    /// The total amount of the order including tax, surcharge and shipping cost
    pub net_amount: MinorUnit,
    /// The amount that was requested to be captured for this payment
    pub amount_to_capture: Option<MinorUnit>,
    /// The amount that can be captured on the payment. Either in one go or through multiple captures.
    /// This is applicable in case the capture method was either `manual` or `manual_multiple`
    pub amount_capturable: MinorUnit,
    /// The amount that was captured for this payment. This is the sum of all the captures done on this payment
    pub amount_captured: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]

pub struct PaymentAttemptAmountDetails {
    /// The total amount of the order including tax, surcharge and shipping cost
    pub net_amount: MinorUnit,
    /// The amount that was requested to be captured for this payment
    pub amount_to_capture: Option<MinorUnit>,
    /// Surcharge amount for the payment attempt.
    /// This is either derived by surcharge rules, or sent by the merchant
    pub surcharge_amount: Option<MinorUnit>,
    /// Tax amount for the payment attempt
    /// This is either derived by surcharge rules, or sent by the merchant
    pub tax_on_surcharge: Option<MinorUnit>,
    /// The total amount that can be captured for this payment attempt.
    pub amount_capturable: MinorUnit,
    /// Shipping cost for the payment attempt.
    /// Shipping cost for the payment attempt.
    pub shipping_cost: Option<MinorUnit>,
    /// Tax amount for the order.
    /// This is either derived by calling an external tax processor, or sent by the merchant
    pub order_tax_amount: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
impl AmountDetails {
    pub fn new(amount_details_setter: AmountDetailsSetter) -> Self {
        Self {
            order_amount: amount_details_setter.order_amount,
            currency: amount_details_setter.currency,
            shipping_cost: amount_details_setter.shipping_cost,
            order_tax_amount: amount_details_setter.order_tax_amount,
            skip_external_tax_calculation: amount_details_setter.skip_external_tax_calculation,
            skip_surcharge_calculation: amount_details_setter.skip_surcharge_calculation,
            surcharge_amount: amount_details_setter.surcharge_amount,
            tax_on_surcharge: amount_details_setter.tax_on_surcharge,
        }
    }
    pub fn order_amount(&self) -> Amount {
        self.order_amount
    }
    pub fn currency(&self) -> common_enums::Currency {
        self.currency
    }
    pub fn shipping_cost(&self) -> Option<MinorUnit> {
        self.shipping_cost
    }
    pub fn order_tax_amount(&self) -> Option<MinorUnit> {
        self.order_tax_amount
    }
    pub fn skip_external_tax_calculation(&self) -> common_enums::TaxCalculationOverride {
        self.skip_external_tax_calculation
    }
    pub fn skip_surcharge_calculation(&self) -> common_enums::SurchargeCalculationOverride {
        self.skip_surcharge_calculation
    }
    pub fn surcharge_amount(&self) -> Option<MinorUnit> {
        self.surcharge_amount
    }
    pub fn tax_on_surcharge(&self) -> Option<MinorUnit> {
        self.tax_on_surcharge
    }
}

#[cfg(feature = "v2")]
impl AmountDetailsUpdate {
    pub fn order_amount(&self) -> Option<Amount> {
        self.order_amount
    }
    pub fn currency(&self) -> Option<common_enums::Currency> {
        self.currency
    }
    pub fn shipping_cost(&self) -> Option<MinorUnit> {
        self.shipping_cost
    }
    pub fn order_tax_amount(&self) -> Option<MinorUnit> {
        self.order_tax_amount
    }
    pub fn skip_external_tax_calculation(&self) -> Option<common_enums::TaxCalculationOverride> {
        self.skip_external_tax_calculation
    }
    pub fn skip_surcharge_calculation(&self) -> Option<common_enums::SurchargeCalculationOverride> {
        self.skip_surcharge_calculation
    }
    pub fn surcharge_amount(&self) -> Option<MinorUnit> {
        self.surcharge_amount
    }
    pub fn tax_on_surcharge(&self) -> Option<MinorUnit> {
        self.tax_on_surcharge
    }
}
#[cfg(feature = "v1")]
#[derive(
    Default,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    ToSchema,
    router_derive::PolymorphicSchema,
    router_derive::ValidateSchema,
    SmithyModel,
)]
#[generate_schemas(PaymentsCreateRequest, PaymentsUpdateRequest, PaymentsConfirmRequest)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentsRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    #[schema(value_type = Option<u64>, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize_option")]
    #[mandatory_in(PaymentsCreateRequest = u64)]
    #[smithy(value_type = "Option<u64>")]
    // Makes the field mandatory in PaymentsCreateRequest
    pub amount: Option<Amount>,

    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub order_tax_amount: Option<MinorUnit>,

    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    #[schema(example = "USD", value_type = Option<Currency>)]
    #[mandatory_in(PaymentsCreateRequest = Currency)]
    #[smithy(value_type = "Option<Currency>")]
    pub currency: Option<api_enums::Currency>,

    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount_to_capture: Option<MinorUnit>,

    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub shipping_cost: Option<MinorUnit>,

    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    #[serde(default, deserialize_with = "payment_id_type::deserialize_option")]
    #[smithy(value_type = "Option<String>", length = "30..=30")]
    pub payment_id: Option<PaymentIdType>,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825", value_type = Option<String>)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub merchant_id: Option<id_type::MerchantId>,

    /// Details of the routing configuration for that payment
    #[schema(value_type = Option<StraightThroughAlgorithm>, example = json!({
        "type": "single",
        "data": {"connector": "stripe", "merchant_connector_id": "mca_123"}
    }))]
    #[smithy(value_type = "Option<Object>")]
    pub routing: Option<serde_json::Value>,

    /// This allows to manually select a connector with which the payment can go through.
    #[schema(value_type = Option<Vec<Connector>>, max_length = 255, example = json!(["stripe", "adyen"]))]
    #[smithy(value_type = "Option<Vec<Connector>>")]
    pub connector: Option<Vec<api_enums::Connector>>,

    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    #[smithy(value_type = "Option<CaptureMethod>")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    #[smithy(value_type = "Option<AuthenticationType>")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    #[smithy(value_type = "Option<Address>")]
    pub billing: Option<Address>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub capture_on: Option<PrimitiveDateTime>,

    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    #[schema(default = false, example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub confirm: Option<bool>,

    /// Passing this object creates a new customer or attaches an existing customer to the payment
    #[smithy(value_type = "Option<CustomerDetails>")]
    pub customer: Option<CustomerDetails>,

    /// The identifier for the customer
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    #[smithy(value_type = "Option<String>")]
    pub customer_id: Option<id_type::CustomerId>,

    /// The customer's email address.
    /// This field will be deprecated soon, use the customer object instead
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com", deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub email: Option<Email>,

    /// The customer's name.
    /// This field will be deprecated soon, use the customer object instead.
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test", deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub name: Option<Secret<String>>,

    /// The customer's phone number
    /// This field will be deprecated soon, use the customer object instead
    #[schema(value_type = Option<String>, max_length = 255, example = "9123456789", deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer phone number
    /// This field will be deprecated soon, use the customer object instead
    #[schema(max_length = 255, example = "+1", deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub phone_country_code: Option<String>,

    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    #[schema(example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub off_session: Option<bool>,

    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    #[schema(example = "It's my first payment request")]
    #[smithy(value_type = "Option<String>")]
    pub description: Option<String>,

    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io", max_length = 2048)]
    #[smithy(value_type = "Option<String>")]
    pub return_url: Option<Url>,

    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    #[smithy(value_type = "Option<FutureUsage>")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    #[schema(example = "bank_transfer")]
    #[serde(with = "payment_method_data_serde", default)]
    #[smithy(value_type = "Option<PaymentMethodDataRequest>")]
    pub payment_method_data: Option<PaymentMethodDataRequest>,

    #[schema(value_type = Option<PaymentMethod>, example = "card")]
    #[smithy(value_type = "Option<PaymentMethod>")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    #[smithy(value_type = "Option<String>")]
    pub payment_token: Option<String>,

    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    #[schema(value_type = Option<String>, deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub card_cvc: Option<Secret<String>>,

    /// The shipping address for the payment
    #[smithy(value_type = "Option<Address>")]
    pub shipping: Option<Address>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters. To be deprecated soon, use billing_descriptor instead.
    #[schema(max_length = 255, example = "Hyperswitch Router", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor. To be deprecated soon, use billing_descriptor instead.
    #[schema(max_length = 255, example = "Payment for shoes purchase", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_suffix: Option<String>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    #[smithy(value_type = "Option<OrderDetailsWithAmount>")]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// It's a token used for client side verification.
    #[schema(example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest)]
    #[smithy(value_type = "Option<String>")]
    pub client_secret: Option<String>,

    /// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
    #[smithy(value_type = "Option<MandateData>")]
    pub mandate_data: Option<MandateData>,

    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    #[schema(value_type = Option<CustomerAcceptance>)]
    #[smithy(value_type = "Option<CustomerAcceptance>")]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,

    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    #[schema(max_length = 64, example = "mandate_iwer89rnjef349dni3")]
    #[remove_in(PaymentsUpdateRequest)]
    #[smithy(value_type = "Option<String>")]
    pub mandate_id: Option<String>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>, example = r#"{
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
        "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
        "language": "nl-NL",
        "color_depth": 24,
        "screen_height": 723,
        "screen_width": 1536,
        "time_zone": 0,
        "java_enabled": true,
        "java_script_enabled":true
    }"#)]
    #[smithy(value_type = "Option<BrowserInformation>")]
    pub browser_info: Option<serde_json::Value>,

    /// To indicate the type of payment experience that the payment method would go through
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    #[smithy(value_type = "Option<PaymentExperience>")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// Can be used to specify the Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    #[smithy(value_type = "Option<PaymentMethodType>")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// Business country of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    #[smithy(value_type = "Option<CountryAlpha2>")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    #[schema(example = "food")]
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    #[smithy(value_type = "Option<String>")]
    pub business_label: Option<String>,

    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    #[smithy(value_type = "Option<MerchantConnectorDetailsWrap>")]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    #[smithy(value_type = "Option<Vec<PaymentMethodType>>")]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Business sub label for the payment
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest, PaymentsCreateRequest)]
    pub business_sub_label: Option<String>,

    /// Denotes the retry action
    #[schema(value_type = Option<RetryAction>)]
    #[remove_in(PaymentsCreateRequest)]
    #[smithy(value_type = "Option<RetryAction>")]
    pub retry_action: Option<api_enums::RetryAction>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<serde_json::Value>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    #[smithy(value_type = "Option<ConnectorMetadata>")]
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(default = false, example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub payment_link: Option<bool>,

    #[schema(value_type = Option<PaymentCreatePaymentLinkConfig>)]
    pub payment_link_config: Option<PaymentCreatePaymentLinkConfig>,

    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    #[smithy(value_type = "Option<String>")]
    pub payment_link_config_id: Option<String>,

    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub profile_id: Option<id_type::ProfileId>,

    #[remove_in(PaymentsConfirmRequest)]
    #[schema(value_type = Option<RequestSurchargeDetails>)]
    #[smithy(value_type = "Option<RequestSurchargeDetails>")]
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// The type of the payment that differentiates between normal and various types of mandate payments
    #[schema(value_type = Option<PaymentType>)]
    #[smithy(value_type = "Option<PaymentType>")]
    pub payment_type: Option<api_enums::PaymentType>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[smithy(value_type = "Option<bool>")]
    pub request_incremental_authorization: Option<bool>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    #[smithy(value_type = "Option<u32>")]
    pub session_expiry: Option<u32>,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub request_external_three_ds_authentication: Option<bool>,

    /// Three Ds Data supplied by the Merchant, Merchant have done the external authentication
    pub three_ds_data: Option<ExternalThreeDsData>,

    /// Details required for recurring payment
    #[smithy(value_type = "Option<RecurringDetails>")]
    pub recurring_details: Option<RecurringDetails>,

    /// Fee information to be charged on the payment being collected
    #[schema(value_type = Option<SplitPaymentsRequest>)]
    #[smithy(value_type = "Option<SplitPaymentsRequest>")]
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,

    /// Optional boolean value to extent authorization period of this payment
    ///
    /// capture method must be manual or manual_multiple
    #[schema(value_type = Option<bool>, default = false)]
    #[smithy(value_type = "Option<bool>")]
    pub request_extended_authorization: Option<RequestExtendedAuthorizationBool>,

    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    #[schema(
        value_type = Option<String>,
        max_length = 255,
        example = "Custom_Order_id_123"
    )]
    #[smithy(value_type = "Option<String>")]
    pub merchant_order_reference_id: Option<String>,

    /// Whether to calculate tax for this payment intent
    #[smithy(value_type = "Option<bool>")]
    pub skip_external_tax_calculation: Option<bool>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    #[smithy(value_type = "Option<ScaExemptionType>")]
    pub psd2_sca_exemption_type: Option<api_enums::ScaExemptionType>,

    /// Service details for click to pay external authentication
    #[schema(value_type = Option<CtpServiceDetails>)]
    #[smithy(value_type = "Option<CtpServiceDetails>")]
    pub ctp_service_details: Option<CtpServiceDetails>,

    /// Indicates if 3ds challenge is forced
    #[smithy(value_type = "Option<bool>")]
    pub force_3ds_challenge: Option<bool>,

    /// Indicates if 3DS method data was successfully completed or not
    #[smithy(value_type = "Option<ThreeDsCompletionIndicator>")]
    pub threeds_method_comp_ind: Option<ThreeDsCompletionIndicator>,

    /// Indicates if the redirection has to open in the iframe
    #[smithy(value_type = "Option<bool>")]
    pub is_iframe_redirection_enabled: Option<bool>,

    /// If enabled, provides whole connector response
    #[smithy(value_type = "Option<bool>")]
    pub all_keys_required: Option<bool>,

    /// Indicates whether the `payment_id` was provided by the merchant
    /// This value is inferred internally based on the request
    #[serde(skip_deserializing)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub is_payment_id_from_merchant: bool,

    /// Indicates how the payment was initiated (e.g., ecommerce, mail, or telephone).
    #[schema(value_type = Option<PaymentChannel>)]
    #[smithy(value_type = "Option<PaymentChannel>")]
    pub payment_channel: Option<common_enums::PaymentChannel>,

    /// Your tax status for this order or transaction.
    #[schema(value_type = Option<TaxStatus>)]
    #[smithy(value_type = "Option<TaxStatus>")]
    pub tax_status: Option<api_enums::TaxStatus>,

    /// Total amount of the discount you have applied to the order or transaction.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub discount_amount: Option<MinorUnit>,

    /// Tax amount applied to shipping charges.
    #[smithy(value_type = "Option<i64>")]
    pub shipping_amount_tax: Option<MinorUnit>,

    /// Duty or customs fee amount for international transactions.
    #[smithy(value_type = "Option<i64>")]
    pub duty_amount: Option<MinorUnit>,

    /// Date the payer placed the order.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub order_date: Option<PrimitiveDateTime>,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    #[smithy(value_type = "Option<bool>")]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,

    /// Boolean indicating whether to enable overcapture for this payment
    #[remove_in(PaymentsConfirmRequest)]
    #[schema(value_type = Option<bool>, example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub enable_overcapture: Option<primitive_wrappers::EnableOvercaptureBool>,

    /// Boolean flag indicating whether this payment method is stored and has been previously used for payments
    #[schema(value_type = Option<bool>, example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub is_stored_credential: Option<bool>,

    /// The category of the MIT transaction
    #[schema(value_type = Option<MitCategory>, example = "recurring")]
    #[smithy(value_type = "Option<MitCategory>")]
    pub mit_category: Option<api_enums::MitCategory>,

    /// Billing descriptor information for the payment
    #[schema(value_type = Option<BillingDescriptor>)]
    pub billing_descriptor: Option<common_types::payments::BillingDescriptor>,

    /// The tokenization preference for the payment method. This is used to control whether a PSP token is created or not.
    #[schema(value_type = Option<Tokenization>, example = "tokenize_at_psp")]
    pub tokenization: Option<enums::Tokenization>,

    /// Information identifying partner and merchant details
    #[schema(value_type = Option<PartnerMerchantIdentifierDetails>)]
    pub partner_merchant_identifier_details:
        Option<common_types::payments::PartnerMerchantIdentifierDetails>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CtpServiceDetails {
    /// merchant transaction id
    #[smithy(value_type = "Option<String>")]
    pub merchant_transaction_id: Option<String>,
    /// network transaction correlation id
    #[smithy(value_type = "Option<String>")]
    pub correlation_id: Option<String>,
    /// session transaction flow id
    #[smithy(value_type = "Option<String>")]
    pub x_src_flow_id: Option<String>,
    /// provider Eg: Visa, Mastercard
    #[schema(value_type = Option<CtpServiceProvider>)]
    #[smithy(value_type = "Option<CtpServiceProvider>")]
    pub provider: Option<api_enums::CtpServiceProvider>,
    /// Encrypted payload
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub encrypted_payload: Option<Secret<String>>,
}

impl CtpServiceDetails {
    pub fn is_network_confirmation_call_required(&self) -> bool {
        self.provider == Some(api_enums::CtpServiceProvider::Mastercard)
    }
}

#[cfg(feature = "v1")]
/// Checks if the inner values of two options are equal
/// Returns true if values are not equal, returns false in other cases
fn are_optional_values_invalid<T: PartialEq>(
    first_option: Option<&T>,
    second_option: Option<&T>,
) -> bool {
    match (first_option, second_option) {
        (Some(first_option), Some(second_option)) => first_option != second_option,
        _ => false,
    }
}

#[cfg(feature = "v1")]
impl PaymentsRequest {
    /// Get the customer id
    ///
    /// First check the id for `customer.id`
    /// If not present, check for `customer_id` at the root level
    pub fn get_customer_id(&self) -> Option<&id_type::CustomerId> {
        self.customer_id
            .as_ref()
            .or(self.customer.as_ref().map(|customer| &customer.id))
    }

    pub fn validate_and_get_request_extended_authorization(
        &self,
    ) -> common_utils::errors::CustomResult<Option<RequestExtendedAuthorizationBool>, ValidationError>
    {
        self.request_extended_authorization
            .as_ref()
            .map(|request_extended_authorization| {
                request_extended_authorization.validate_field_and_get(self)
            })
            .transpose()
    }

    /// Checks if the customer details are passed in both places
    /// If they are passed in both places, check for both the values to be equal
    /// Or else, return the field which has inconsistent data
    pub fn validate_customer_details_in_request(&self) -> Option<Vec<&str>> {
        if let Some(CustomerDetails {
            id,
            name,
            email,
            phone,
            phone_country_code,
            ..
        }) = self.customer.as_ref()
        {
            let invalid_fields = [
                are_optional_values_invalid(self.customer_id.as_ref(), Some(id))
                    .then_some("customer_id and customer.id"),
                are_optional_values_invalid(self.email.as_ref(), email.as_ref())
                    .then_some("email and customer.email"),
                are_optional_values_invalid(self.name.as_ref(), name.as_ref())
                    .then_some("name and customer.name"),
                are_optional_values_invalid(self.phone.as_ref(), phone.as_ref())
                    .then_some("phone and customer.phone"),
                are_optional_values_invalid(
                    self.phone_country_code.as_ref(),
                    phone_country_code.as_ref(),
                )
                .then_some("phone_country_code and customer.phone_country_code"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

            if invalid_fields.is_empty() {
                None
            } else {
                Some(invalid_fields)
            }
        } else {
            None
        }
    }

    pub fn get_feature_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.feature_metadata
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
    }

    pub fn get_connector_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.connector_metadata
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
    }

    pub fn get_allowed_payment_method_types_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.allowed_payment_method_types
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
    }

    pub fn get_order_details_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<Vec<pii::SecretSerdeValue>>,
        common_utils::errors::ParsingError,
    > {
        self.order_details
            .as_ref()
            .map(|od| {
                od.iter()
                    .map(|order| order.encode_to_value().map(Secret::new))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    }
    pub fn validate_stored_credential(
        &self,
    ) -> common_utils::errors::CustomResult<(), ValidationError> {
        if self.is_stored_credential == Some(false)
            && (self.recurring_details.is_some()
                || self.payment_token.is_some()
                || self.mandate_id.is_some())
        {
            Err(ValidationError::InvalidValue {
                message:
                    "is_stored_credential should be true when reusing stored payment method data"
                        .to_string(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    pub fn validate_mit_request(&self) -> common_utils::errors::CustomResult<(), ValidationError> {
        if self.mit_category.is_some()
            && (!matches!(self.off_session, Some(true)) || self.recurring_details.is_none())
        {
            return Err(ValidationError::InvalidValue {
                message: "`mit_category` requires both: (1) `off_session = true`, and (2) `recurring_details`.".to_string(),
            }
            .into());
        }

        Ok(())
    }
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod payments_request_test {
    use common_utils::generate_customer_id_of_default_length;

    use super::*;

    #[test]
    fn test_valid_case_where_customer_details_are_passed_only_once() {
        let customer_id = generate_customer_id_of_default_length();
        let payments_request = PaymentsRequest {
            customer_id: Some(customer_id),
            ..Default::default()
        };

        assert!(payments_request
            .validate_customer_details_in_request()
            .is_none());
    }

    #[test]
    fn test_valid_case_where_customer_id_is_passed_in_both_places() {
        let customer_id = generate_customer_id_of_default_length();

        let customer_object = CustomerDetails {
            id: customer_id.clone(),
            name: None,
            email: None,
            phone: None,
            phone_country_code: None,
            tax_registration_id: None,
        };

        let payments_request = PaymentsRequest {
            customer_id: Some(customer_id),
            customer: Some(customer_object),
            ..Default::default()
        };

        assert!(payments_request
            .validate_customer_details_in_request()
            .is_none());
    }

    #[test]
    fn test_invalid_case_where_customer_id_is_passed_in_both_places() {
        let customer_id = generate_customer_id_of_default_length();
        let another_customer_id = generate_customer_id_of_default_length();

        let customer_object = CustomerDetails {
            id: customer_id.clone(),
            name: None,
            email: None,
            phone: None,
            phone_country_code: None,
            tax_registration_id: None,
        };

        let payments_request = PaymentsRequest {
            customer_id: Some(another_customer_id),
            customer: Some(customer_object),
            ..Default::default()
        };

        assert_eq!(
            payments_request.validate_customer_details_in_request(),
            Some(vec!["customer_id and customer.id"])
        );
    }
}

/// Details of surcharge applied on this payment, if applicable
#[derive(
    Default,
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    Copy,
    ToSchema,
    PartialEq,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RequestSurchargeDetails {
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub surcharge_amount: MinorUnit,
    #[smithy(value_type = "Option<i64>")]
    pub tax_amount: Option<MinorUnit>,
}

// for v2 use the type from common_utils::types
#[cfg(feature = "v1")]
/// Browser information to be used for 3DS 2.0
#[derive(ToSchema, Debug, serde::Deserialize, serde::Serialize, Clone, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BrowserInformation {
    /// Color depth supported by the browser
    #[smithy(value_type = "Option<u8>")]
    pub color_depth: Option<u8>,

    /// Whether java is enabled in the browser
    #[smithy(value_type = "Option<bool>")]
    pub java_enabled: Option<bool>,

    /// Whether javascript is enabled in the browser
    #[smithy(value_type = "Option<bool>")]
    pub java_script_enabled: Option<bool>,

    /// Language supported
    #[smithy(value_type = "Option<String>")]
    pub language: Option<String>,

    /// The screen height in pixels
    #[smithy(value_type = "Option<u32>")]
    pub screen_height: Option<u32>,

    /// The screen width in pixels
    #[smithy(value_type = "Option<u32>")]
    pub screen_width: Option<u32>,

    /// Time zone of the client
    #[smithy(value_type = "Option<i32>")]
    pub time_zone: Option<i32>,

    /// Ip address of the client
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub ip_address: Option<std::net::IpAddr>,

    /// List of headers that are accepted
    #[schema(
        example = "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
    )]
    #[smithy(value_type = "Option<String>")]
    pub accept_header: Option<String>,

    /// User-agent of the browser
    #[smithy(value_type = "Option<String>")]
    pub user_agent: Option<String>,

    /// The os type of the client device
    #[smithy(value_type = "Option<String>")]
    pub os_type: Option<String>,

    /// The os version of the client device
    #[smithy(value_type = "Option<String>")]
    pub os_version: Option<String>,

    /// The device model of the client
    #[smithy(value_type = "Option<String>")]
    pub device_model: Option<String>,

    /// Accept-language of the browser
    #[smithy(value_type = "Option<String>")]
    pub accept_language: Option<String>,

    /// Identifier of the source that initiated the request.
    #[smithy(value_type = "Option<String>")]
    pub referer: Option<String>,
}

impl RequestSurchargeDetails {
    pub fn is_surcharge_zero(&self) -> bool {
        self.surcharge_amount == MinorUnit::new(0)
            && self.tax_amount.unwrap_or_default() == MinorUnit::new(0)
    }
    pub fn get_total_surcharge_amount(&self) -> MinorUnit {
        self.surcharge_amount + self.tax_amount.unwrap_or_default()
    }

    pub fn get_surcharge_amount(&self) -> MinorUnit {
        self.surcharge_amount
    }

    pub fn get_tax_amount(&self) -> Option<MinorUnit> {
        self.tax_amount
    }
}

#[cfg(feature = "v1")]
#[derive(
    Debug,
    serde::Serialize,
    Clone,
    PartialEq,
    ToSchema,
    router_derive::PolymorphicSchema,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentAttemptResponse {
    /// A unique identifier for this specific payment attempt.
    #[smithy(value_type = "String")]
    pub attempt_id: String,
    /// The status of the attempt
    #[schema(value_type = AttemptStatus, example = "charged")]
    #[smithy(value_type = "AttemptStatus")]
    pub status: enums::AttemptStatus,
    /// The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    /// The payment attempt tax_amount.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub order_tax_amount: Option<MinorUnit>,
    /// The currency of the amount of the payment attempt
    #[schema(value_type = Option<Currency>, example = "USD")]
    #[smithy(value_type = "Option<Currency>")]
    pub currency: Option<enums::Currency>,
    /// The name of the payment connector (e.g., 'stripe', 'adyen') used for this attempt.
    #[smithy(value_type = "Option<String>")]
    pub connector: Option<String>,
    /// A human-readable message from the connector explaining the error, if one occurred during this payment attempt.
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
    /// The payment method that is to be used
    #[schema(value_type = Option<PaymentMethod>, example = "bank_transfer")]
    #[smithy(value_type = "Option<PaymentMethod>")]
    pub payment_method: Option<enums::PaymentMethod>,
    /// A unique identifier for a payment provided by the connector
    #[smithy(value_type = "Option<String>")]
    pub connector_transaction_id: Option<String>,
    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "scheduled")]
    #[smithy(value_type = "Option<CaptureMethod>")]
    pub capture_method: Option<enums::CaptureMethod>,
    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    #[smithy(value_type = "Option<AuthenticationType>")]
    pub authentication_type: Option<enums::AuthenticationType>,
    /// Time at which the payment attempt was created
    #[schema(value_type = PrimitiveDateTime, example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    #[smithy(value_type = "String")]
    pub created_at: PrimitiveDateTime,
    /// Time at which the payment attempt was last modified
    #[schema(value_type = PrimitiveDateTime, example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    #[smithy(value_type = "String")]
    pub modified_at: PrimitiveDateTime,
    /// If the payment was cancelled the reason will be provided here
    #[smithy(value_type = "Option<String>")]
    pub cancellation_reason: Option<String>,
    /// If this payment attempt is associated with a mandate (e.g., for a recurring or subsequent payment), this field will contain the ID of that mandate.
    #[smithy(value_type = "Option<String>")]
    pub mandate_id: Option<String>,
    /// The error code returned by the connector if this payment attempt failed. This code is specific to the connector.
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// If a tokenized (saved) payment method was used for this attempt, this field contains the payment token representing that payment method.
    #[smithy(value_type = "Option<String>")]
    pub payment_token: Option<String>,
    /// Additional data related to some connectors
    #[smithy(value_type = "Option<Object>")]
    pub connector_metadata: Option<serde_json::Value>,
    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    #[smithy(value_type = "Option<PaymentExperience>")]
    pub payment_experience: Option<enums::PaymentExperience>,
    /// Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    #[smithy(value_type = "Option<PaymentMethodType>")]
    pub payment_method_type: Option<enums::PaymentMethodType>,
    /// The connector's own reference or transaction ID for this specific payment attempt. Useful for reconciliation with the connector.
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    #[smithy(value_type = "Option<String>")]
    pub reference_id: Option<String>,
    /// (This field is not live yet)Error code unified across the connectors is received here if there was an error while calling connector
    #[smithy(value_type = "Option<String>")]
    pub unified_code: Option<String>,
    /// (This field is not live yet)Error message unified across the connectors is received here if there was an error while calling connector
    #[smithy(value_type = "Option<String>")]
    pub unified_message: Option<String>,
    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    #[smithy(value_type = "Option<String>")]
    pub client_source: Option<String>,
    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
    #[smithy(value_type = "Option<String>")]
    pub client_version: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, PartialEq, ToSchema, router_derive::PolymorphicSchema)]
pub struct PaymentAttemptResponse {
    /// The global identifier for the payment attempt
    #[schema(value_type = String)]
    pub id: id_type::GlobalAttemptId,

    /// /// The status of the attempt
    #[schema(value_type = AttemptStatus, example = "charged")]
    pub status: enums::AttemptStatus,

    /// Amount related information for this payment and attempt
    pub amount: PaymentAttemptAmountDetails,

    /// Name of the connector that was used for the payment attempt.
    #[schema(example = "stripe")]
    pub connector: Option<String>,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS, as the 3DS method helps with more robust payer authentication
    #[schema(value_type = AuthenticationType, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: api_enums::AuthenticationType,

    /// Date and time of Payment attempt creation
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,

    /// Time at which the payment attempt was last modified
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,

    /// The reason for the cancellation of the payment attempt. Some connectors will have strict rules regarding the values this can have
    /// Cancellation reason will be validated at the connector level when building the request
    pub cancellation_reason: Option<String>,

    /// Payment token is the token used for temporary use in case the payment method is stored in vault
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// Additional data related to some connectors
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<pii::SecretSerdeValue>,

    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<enums::PaymentExperience>,

    /// Payment method type for the payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: common_enums::PaymentMethod,

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_reference_id: Option<String>,

    /// The payment method subtype for the payment attempt.
    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = String)]
    pub connector_payment_id: Option<common_utils::types::ConnectorTransactionId>,

    /// Identifier for Payment Method used for the payment attempt
    #[schema(value_type = Option<String>, example = "12345_pm_01926c58bc6e77c09e809964e72af8c8")]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    pub client_source: Option<String>,

    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
    pub client_version: Option<String>,

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    pub feature_metadata: Option<PaymentAttemptFeatureMetadata>,

    /// The payment method information for the payment attempt
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentAttemptRecordResponse {
    /// The global identifier for the payment attempt
    #[schema(value_type = String)]
    pub id: id_type::GlobalAttemptId,
    /// The status of the attempt
    #[schema(value_type = AttemptStatus, example = "charged")]
    pub status: enums::AttemptStatus,
    /// The amount of the payment attempt
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// Error details for the payment attempt, if any.
    /// This includes fields like error code, network advice code, and network decline code.
    pub error_details: Option<RecordAttemptErrorDetails>,
    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[schema(value_type = Option<FeatureMetadata>)]
    pub payment_intent_feature_metadata: Option<FeatureMetadata>,
    /// Additional data that might be required by hyperswitch, to enable some specific features.
    pub payment_attempt_feature_metadata: Option<PaymentAttemptFeatureMetadata>,
    /// attempt created at timestamp
    pub created_at: PrimitiveDateTime,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct RecoveryPaymentsResponse {
    /// Unique identifier for the payment.
    #[schema(
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    pub id: id_type::GlobalPaymentId,

    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    pub intent_status: api_enums::IntentStatus,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentAttemptFeatureMetadata {
    /// Revenue recovery metadata that might be required by hyperswitch.
    pub revenue_recovery: Option<PaymentAttemptRevenueRecoveryData>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentAttemptRevenueRecoveryData {
    /// Flag to find out whether an attempt was created by external or internal system.
    #[schema(value_type = Option<TriggeredBy>, example = "internal")]
    pub attempt_triggered_by: common_enums::TriggeredBy,
    // stripe specific field used to identify duplicate attempts.
    #[schema(value_type = Option<String>, example = "ch_123abc456def789ghi012klmn")]
    pub charge_id: Option<String>,
}

#[derive(
    Default,
    Debug,
    serde::Serialize,
    Clone,
    PartialEq,
    ToSchema,
    router_derive::PolymorphicSchema,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CaptureResponse {
    /// A unique identifier for this specific capture operation.
    #[smithy(value_type = "String")]
    pub capture_id: String,
    /// The status of the capture
    #[schema(value_type = CaptureStatus, example = "charged")]
    #[smithy(value_type = "CaptureStatus")]
    pub status: enums::CaptureStatus,
    /// The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    /// The currency of the amount of the capture
    #[schema(value_type = Option<Currency>, example = "USD")]
    #[smithy(value_type = "Option<Currency>")]
    pub currency: Option<enums::Currency>,
    /// The name of the payment connector that processed this capture.
    #[smithy(value_type = "String")]
    pub connector: String,
    /// The ID of the payment attempt that was successfully authorized and subsequently captured by this operation.
    #[smithy(value_type = "String")]
    pub authorized_attempt_id: String,
    /// A unique identifier for this capture provided by the connector
    #[smithy(value_type = "Option<String>")]
    pub connector_capture_id: Option<String>,
    /// Sequence number of this capture, in the series of captures made for the parent attempt
    #[smithy(value_type = "i16")]
    pub capture_sequence: i16,
    /// A human-readable message from the connector explaining why this capture operation failed, if applicable.
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
    /// The error code returned by the connector if this capture operation failed. This code is connector-specific.
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// A more detailed reason from the connector explaining the capture failure, if available.
    #[smithy(value_type = "Option<String>")]
    pub error_reason: Option<String>,
    /// The connector's own reference or transaction ID for this specific capture operation. Useful for reconciliation.
    #[smithy(value_type = "Option<String>")]
    pub reference_id: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    Value(NonZeroI64),
    #[default]
    Zero,
}

impl From<Amount> for MinorUnit {
    fn from(amount: Amount) -> Self {
        match amount {
            Amount::Value(val) => Self::new(val.get()),
            Amount::Zero => Self::new(0),
        }
    }
}

impl From<MinorUnit> for Amount {
    fn from(minor_unit: MinorUnit) -> Self {
        match minor_unit.get_amount_as_i64() {
            0 => Self::Zero,
            val => NonZeroI64::new(val).map_or(Self::Zero, Self::Value),
        }
    }
}
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRedirectRequest {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub connector: String,
    pub param: String,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    // The merchant_id is generated through api key
    // and is later passed in the struct
    pub merchant_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub email: Option<Email>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_data: Option<PaymentMethodData>,
    pub payment_token: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MandateTransactionType {
    NewMandateTransaction,
    RecurringMandateTransaction,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct MandateIds {
    pub mandate_id: Option<String>,
    pub mandate_reference_id: Option<MandateReferenceId>,
}

impl MandateIds {
    pub fn is_network_transaction_id_flow(&self) -> bool {
        matches!(
            self.mandate_reference_id,
            Some(MandateReferenceId::NetworkMandateId(_))
        )
    }

    pub fn get_connector_mandate_id(&self) -> Option<String> {
        match &self.mandate_reference_id {
            Some(MandateReferenceId::ConnectorMandateId(data)) => data.connector_mandate_id.clone(),
            _ => None,
        }
    }

    pub fn get_connector_mandate_metadata(&self) -> Option<pii::SecretSerdeValue> {
        match &self.mandate_reference_id {
            Some(MandateReferenceId::ConnectorMandateId(data)) => data.mandate_metadata.clone(),
            _ => None,
        }
    }

    pub fn get_updated_mandate_details_of_connector_mandate_id(
        &self,
    ) -> Option<UpdatedMandateDetails> {
        match &self.mandate_reference_id {
            Some(MandateReferenceId::ConnectorMandateId(data)) => {
                data.updated_mandate_details.clone()
            }
            _ => None,
        }
    }
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum MandateReferenceId {
    ConnectorMandateId(ConnectorMandateReferenceId), // mandate_id send by connector
    NetworkMandateId(String), // network_txns_id send by Issuer to connector, Used for PG agnostic mandate txns along with card data
    NetworkTokenWithNTI(NetworkTokenWithNTIRef), // network_txns_id send by Issuer to connector, Used for PG agnostic mandate txns along with network token data
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub struct NetworkTokenWithNTIRef {
    pub network_transaction_id: String,
    pub token_exp_month: Option<Secret<String>>,
    pub token_exp_year: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub struct ConnectorMandateReferenceId {
    connector_mandate_id: Option<String>,
    payment_method_id: Option<String>,
    update_history: Option<Vec<UpdateHistory>>,
    mandate_metadata: Option<pii::SecretSerdeValue>,
    connector_mandate_request_reference_id: Option<String>,
    updated_mandate_details: Option<UpdatedMandateDetails>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UpdatedMandateDetails {
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_isin: Option<String>,
}

impl From<AdditionalCardInfo> for UpdatedMandateDetails {
    fn from(card_info: AdditionalCardInfo) -> Self {
        Self {
            card_network: card_info.card_network,
            card_exp_month: card_info.card_exp_month,
            card_exp_year: card_info.card_exp_year,
            card_isin: card_info.card_isin,
        }
    }
}

impl From<&UpdatedMandateDetails> for AdditionalCardInfo {
    fn from(card_info: &UpdatedMandateDetails) -> Self {
        Self {
            card_network: card_info.card_network.clone(),
            card_exp_month: card_info.card_exp_month.clone(),
            card_exp_year: card_info.card_exp_year.clone(),
            card_isin: card_info.card_isin.clone(),
            card_issuer: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            last4: None,
            card_extended_bin: None,
            card_holder_name: None,
            payment_checks: None,
            authentication_data: None,
            is_regulated: None,
            signature_network: None,
        }
    }
}

impl ConnectorMandateReferenceId {
    pub fn new(
        connector_mandate_id: Option<String>,
        payment_method_id: Option<String>,
        update_history: Option<Vec<UpdateHistory>>,
        mandate_metadata: Option<pii::SecretSerdeValue>,
        connector_mandate_request_reference_id: Option<String>,
        updated_mandate_details: Option<UpdatedMandateDetails>,
    ) -> Self {
        Self {
            connector_mandate_id,
            payment_method_id,
            update_history,
            mandate_metadata,
            connector_mandate_request_reference_id,
            updated_mandate_details,
        }
    }

    pub fn get_connector_mandate_id(&self) -> Option<String> {
        self.connector_mandate_id.clone()
    }
    pub fn get_payment_method_id(&self) -> Option<String> {
        self.payment_method_id.clone()
    }
    pub fn get_mandate_metadata(&self) -> Option<pii::SecretSerdeValue> {
        self.mandate_metadata.clone()
    }
    pub fn get_connector_mandate_request_reference_id(&self) -> Option<String> {
        self.connector_mandate_request_reference_id.clone()
    }

    pub fn update(
        &mut self,
        connector_mandate_id: Option<String>,
        payment_method_id: Option<String>,
        update_history: Option<Vec<UpdateHistory>>,
        mandate_metadata: Option<pii::SecretSerdeValue>,
        connector_mandate_request_reference_id: Option<String>,
    ) {
        self.connector_mandate_id = connector_mandate_id.or(self.connector_mandate_id.clone());
        self.payment_method_id = payment_method_id.or(self.payment_method_id.clone());
        self.update_history = update_history.or(self.update_history.clone());
        self.mandate_metadata = mandate_metadata.or(self.mandate_metadata.clone());
        self.connector_mandate_request_reference_id = connector_mandate_request_reference_id
            .or(self.connector_mandate_request_reference_id.clone());
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UpdateHistory {
    pub connector_mandate_id: Option<String>,
    pub payment_method_id: String,
    pub original_payment_id: Option<id_type::PaymentId>,
}

impl MandateIds {
    pub fn new(mandate_id: String) -> Self {
        Self {
            mandate_id: Some(mandate_id),
            mandate_reference_id: None,
        }
    }
}

/// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
// The fields on this struct are optional, as we want to allow the merchant to provide partial
// information about creating mandates
#[derive(
    Default,
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    ToSchema,
    SmithyModel,
)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MandateData {
    /// A way to update the mandate's payment method details
    #[smithy(value_type = "Option<String>")]
    pub update_mandate_id: Option<String>,
    /// A consent from the customer to store the payment method
    #[schema(value_type = Option<CustomerAcceptance>)]
    #[smithy(value_type = "Option<CustomerAcceptance>")]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    /// A way to select the type of mandate used
    #[smithy(value_type = "Option<MandateType>")]
    pub mandate_type: Option<MandateType>,
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SingleUseMandate {
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
}

#[derive(
    Clone,
    Eq,
    PartialEq,
    Debug,
    Default,
    ToSchema,
    serde::Serialize,
    serde::Deserialize,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MandateAmountData {
    /// The maximum amount to be debited for the mandate transaction
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount: MinorUnit,
    /// The currency for the transaction
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency: api_enums::Currency,
    /// Specifying start date of the mandate
    #[schema(example = "2022-09-10T00:00:00Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<PrimitiveDateTime>")]
    pub start_date: Option<PrimitiveDateTime>,
    /// Specifying end date of the mandate
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<PrimitiveDateTime>")]
    pub end_date: Option<PrimitiveDateTime>,
    /// Additional details required by mandate
    #[schema(value_type = Option<Object>, example = r#"{
        "frequency": "DAILY"
    }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(
    Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum MandateType {
    /// If the mandate should only be valid for 1 off-session use
    #[smithy(value_type = "MandateAmountData")]
    SingleUse(MandateAmountData),
    /// If the mandate should be valid for multiple debits
    #[smithy(value_type = "Option<MandateAmountData>")]
    MultiUse(Option<MandateAmountData>),
}

impl Default for MandateType {
    fn default() -> Self {
        Self::MultiUse(None)
    }
}

#[derive(
    Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NetworkDetails {
    #[smithy(value_type = "Option<String>")]
    pub network_advice_code: Option<String>,
}

#[derive(
    Default,
    Eq,
    PartialEq,
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    ToSchema,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct Card {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    #[smithy(value_type = "String")]
    pub card_number: CardNumber,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = String, example = "242")]
    #[smithy(value_type = "String")]
    pub card_cvc: Secret<String>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,

    #[schema(example = "CREDIT")]
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,

    #[schema(example = "INDIA")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country: Option<String>,

    #[schema(example = "JP_AMEX")]
    #[smithy(value_type = "Option<String>")]
    pub bank_code: Option<String>,
    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub nick_name: Option<Secret<String>>,
}

#[cfg(feature = "v2")]
impl TryFrom<payment_methods::CardDetail> for Card {
    type Error = error_stack::Report<ValidationError>;

    fn try_from(value: payment_methods::CardDetail) -> Result<Self, Self::Error> {
        use common_utils::ext_traits::OptionExt;

        let payment_methods::CardDetail {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name,
            nick_name,
            card_network,
            card_issuer,
            card_cvc,
            ..
        } = value;

        let card_cvc = card_cvc.get_required_value("card_cvc")?;

        Ok(Self {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name,
            card_cvc,
            card_issuer,
            card_network,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name,
        })
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ExtendedCardInfo {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    pub card_number: CardNumber,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = String, example = "242")]
    pub card_cvc: Secret<String>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: Option<api_enums::CardNetwork>,

    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,

    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,

    #[schema(example = "JP_AMEX")]
    pub bank_code: Option<String>,
}

impl From<Card> for ExtendedCardInfo {
    fn from(value: Card) -> Self {
        Self {
            card_number: value.card_number,
            card_exp_month: value.card_exp_month,
            card_exp_year: value.card_exp_year,
            card_holder_name: value.card_holder_name,
            card_cvc: value.card_cvc,
            card_issuer: value.card_issuer,
            card_network: value.card_network,
            card_type: value.card_type,
            card_issuing_country: value.card_issuing_country,
            bank_code: value.bank_code,
        }
    }
}

impl GetAddressFromPaymentMethodData for Card {
    fn get_billing_address(&self) -> Option<Address> {
        // Create billing address if first_name is some or if it is not ""
        self.card_holder_name
            .as_ref()
            .filter(|card_holder_name| !card_holder_name.is_empty_after_trim())
            .map(|card_holder_name| {
                // Split the `card_holder_name` into `first_name` and `last_name` based on the
                // first occurrence of ' '. For example
                // John Wheat Dough
                // first_name -> John
                // last_name -> Wheat Dough
                card_holder_name.peek().split_whitespace()
            })
            .map(|mut card_holder_name_iter| {
                let first_name = card_holder_name_iter
                    .next()
                    .map(ToOwned::to_owned)
                    .map(Secret::new);

                let last_name = card_holder_name_iter.collect::<Vec<_>>().join(" ");
                let last_name = if last_name.is_empty_after_trim() {
                    None
                } else {
                    Some(Secret::new(last_name))
                };

                AddressDetails {
                    first_name,
                    last_name,
                    ..Default::default()
                }
            })
            .map(|address_details| Address {
                address: Some(address_details),
                phone: None,
                email: None,
            })
    }
}

impl Card {
    fn apply_additional_card_info(
        &self,
        additional_card_info: AdditionalCardInfo,
    ) -> Result<Self, error_stack::Report<ValidationError>> {
        Ok(Self {
            card_number: self.card_number.clone(),
            card_exp_month: self.card_exp_month.clone(),
            card_exp_year: self.card_exp_year.clone(),
            card_holder_name: self.card_holder_name.clone(),
            card_cvc: self.card_cvc.clone(),
            card_issuer: self
                .card_issuer
                .clone()
                .or(additional_card_info.card_issuer),
            card_network: self
                .card_network
                .clone()
                .or(additional_card_info.card_network.clone()),
            card_type: self.card_type.clone().or(additional_card_info.card_type),
            card_issuing_country: self
                .card_issuing_country
                .clone()
                .or(additional_card_info.card_issuing_country),
            bank_code: self.bank_code.clone().or(additional_card_info.bank_code),
            nick_name: self.nick_name.clone(),
        })
    }
}

#[derive(
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    ToSchema,
    Default,
    SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardToken {
    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_cvc: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum CardRedirectData {
    #[smithy(nested_value_type)]
    Knet {},
    #[smithy(nested_value_type)]
    Benefit {},
    #[smithy(nested_value_type)]
    MomoAtm {},
    #[smithy(nested_value_type)]
    CardRedirect {},
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum PayLaterData {
    /// For KlarnaRedirect as PayLater Option
    #[smithy(nested_value_type)]
    KlarnaRedirect {
        /// The billing email
        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        billing_email: Option<Email>,
        // The billing country code
        #[schema(value_type = Option<CountryAlpha2>, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        billing_country: Option<api_enums::CountryAlpha2>,
    },
    /// For Klarna Sdk as PayLater Option
    #[smithy(nested_value_type)]
    KlarnaSdk {
        /// The token for the sdk workflow
        #[smithy(value_type = "String")]
        token: String,
    },
    /// For Affirm redirect as PayLater Option
    #[smithy(nested_value_type)]
    AffirmRedirect {},
    /// For AfterpayClearpay redirect as PayLater Option
    #[smithy(nested_value_type)]
    AfterpayClearpayRedirect {
        /// The billing email
        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        billing_email: Option<Email>,
        /// The billing name
        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        billing_name: Option<Secret<String>>,
    },
    /// For PayBright Redirect as PayLater Option
    #[smithy(nested_value_type)]
    PayBrightRedirect {},
    /// For Flexiti Redirect as PayLater long term finance Option
    #[smithy(nested_value_type)]
    FlexitiRedirect {},
    /// For WalleyRedirect as PayLater Option
    #[smithy(nested_value_type)]
    WalleyRedirect {},
    /// For Alma Redirection as PayLater Option
    #[smithy(nested_value_type)]
    AlmaRedirect {},
    #[smithy(nested_value_type)]
    AtomeRedirect {},
    #[smithy(nested_value_type)]
    BreadpayRedirect {},
    PayjustnowRedirect {},
}

impl GetAddressFromPaymentMethodData for PayLaterData {
    fn get_billing_address(&self) -> Option<Address> {
        match self {
            Self::KlarnaRedirect {
                billing_email,
                billing_country,
            } => {
                let address_details = AddressDetails {
                    country: *billing_country,
                    ..AddressDetails::default()
                };

                Some(Address {
                    address: Some(address_details),
                    email: billing_email.clone(),
                    phone: None,
                })
            }
            Self::AfterpayClearpayRedirect {
                billing_email,
                billing_name,
            } => {
                let address_details = AddressDetails {
                    first_name: billing_name.clone(),
                    ..AddressDetails::default()
                };

                Some(Address {
                    address: Some(address_details),
                    email: billing_email.clone(),
                    phone: None,
                })
            }
            Self::PayBrightRedirect {}
            | Self::FlexitiRedirect {}
            | Self::WalleyRedirect {}
            | Self::AlmaRedirect {}
            | Self::KlarnaSdk { .. }
            | Self::AffirmRedirect {}
            | Self::AtomeRedirect {}
            | Self::BreadpayRedirect {}
            | Self::PayjustnowRedirect {} => None,
        }
    }
}

#[derive(
    serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankDebitData {
    /// Payment Method data for Ach bank debit
    #[smithy(nested_value_type)]
    AchBankDebit {
        /// Billing details for bank debit
        #[smithy(value_type = "Option<BankDebitBilling>")]
        billing_details: Option<BankDebitBilling>,
        /// Account number for ach bank debit payment
        #[schema(value_type = String, example = "000123456789")]
        #[smithy(value_type = "String")]
        account_number: Secret<String>,
        /// Routing number for ach bank debit payment
        #[schema(value_type = String, example = "110000000")]
        #[smithy(value_type = "String")]
        routing_number: Secret<String>,

        #[schema(value_type = String, example = "John Test")]
        #[smithy(value_type = "Option<String>")]
        card_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "John Doe")]
        #[smithy(value_type = "Option<String>")]
        bank_account_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "ACH")]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,

        #[schema(value_type = String, example = "Checking")]
        #[smithy(value_type = "Option<BankType>")]
        bank_type: Option<common_enums::BankType>,

        #[schema(value_type = String, example = "Personal")]
        #[smithy(value_type = "Option<BankHolderType>")]
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
    #[smithy(nested_value_type)]
    SepaBankDebit {
        /// Billing details for bank debit
        #[smithy(value_type = "Option<BankDebitBilling>")]
        billing_details: Option<BankDebitBilling>,
        /// International bank account number (iban) for SEPA
        #[schema(value_type = String, example = "DE89370400440532013000")]
        #[smithy(value_type = "String")]
        iban: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        #[smithy(value_type = "Option<String>")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    #[smithy(nested_value_type)]
    SepaGuarenteedBankDebit {
        /// Billing details for bank debit
        #[smithy(value_type = "Option<BankDebitBilling>")]
        billing_details: Option<BankDebitBilling>,
        /// International bank account number (iban) for SEPA
        #[schema(value_type = String, example = "DE89370400440532013000")]
        #[smithy(value_type = "String")]
        iban: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        #[smithy(value_type = "Option<String>")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    #[smithy(nested_value_type)]
    BecsBankDebit {
        /// Billing details for bank debit
        #[smithy(value_type = "Option<BankDebitBilling>")]
        billing_details: Option<BankDebitBilling>,
        /// Account number for Becs payment method
        #[schema(value_type = String, example = "000123456")]
        #[smithy(value_type = "String")]
        account_number: Secret<String>,
        /// Bank-State-Branch (bsb) number
        #[schema(value_type = String, example = "000000")]
        #[smithy(value_type = "String")]
        bsb_number: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = Option<String>, example = "A. Schneider")]
        #[smithy(value_type = "Option<String>")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    #[smithy(nested_value_type)]
    BacsBankDebit {
        /// Billing details for bank debit
        #[smithy(value_type = "Option<BankDebitBilling>")]
        billing_details: Option<BankDebitBilling>,
        /// Account number for Bacs payment method
        #[schema(value_type = String, example = "00012345")]
        #[smithy(value_type = "String")]
        account_number: Secret<String>,
        /// Sort code for Bacs payment method
        #[schema(value_type = String, example = "108800")]
        #[smithy(value_type = "String")]
        sort_code: Secret<String>,
        /// holder name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        #[smithy(value_type = "Option<String>")]
        bank_account_holder_name: Option<Secret<String>>,
    },
}

impl GetAddressFromPaymentMethodData for BankDebitData {
    fn get_billing_address(&self) -> Option<Address> {
        fn get_billing_address_inner(
            bank_debit_billing: Option<&BankDebitBilling>,
            bank_account_holder_name: Option<&Secret<String>>,
        ) -> Option<Address> {
            // We will always have address here
            let mut address = bank_debit_billing
                .and_then(GetAddressFromPaymentMethodData::get_billing_address)?;

            // Prefer `account_holder_name` over `name`
            address.address.as_mut().map(|address| {
                address.first_name = bank_account_holder_name
                    .or(address.first_name.as_ref())
                    .cloned();
            });

            Some(address)
        }

        match self {
            Self::AchBankDebit {
                billing_details,
                bank_account_holder_name,
                ..
            }
            | Self::SepaBankDebit {
                billing_details,
                bank_account_holder_name,
                ..
            }
            | Self::SepaGuarenteedBankDebit {
                billing_details,
                bank_account_holder_name,
                ..
            }
            | Self::BecsBankDebit {
                billing_details,
                bank_account_holder_name,
                ..
            }
            | Self::BacsBankDebit {
                billing_details,
                bank_account_holder_name,
                ..
            } => get_billing_address_inner(
                billing_details.as_ref(),
                bank_account_holder_name.as_ref(),
            ),
        }
    }
}

#[cfg(feature = "v1")]
/// Custom serializer and deserializer for PaymentMethodData
mod payment_method_data_serde {

    use super::*;

    /// Deserialize `reward` payment_method as string for backwards compatibility
    /// The api contract would be
    /// ```json
    /// {
    ///   "payment_method": "reward",
    ///   "payment_method_type": "evoucher",
    ///   "payment_method_data": "reward",
    /// }
    /// ```
    ///
    /// For other payment methods, use the provided deserializer
    /// ```json
    /// "payment_method_data": {
    ///   "card": {
    ///     "card_number": "4242424242424242",
    ///     "card_exp_month": "10",
    ///     "card_exp_year": "25",
    ///     "card_holder_name": "joseph Doe",
    ///     "card_cvc": "123"
    ///    }
    /// }
    /// ```
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<PaymentMethodDataRequest>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize, Debug)]
        #[serde(untagged)]
        enum __Inner {
            RewardString(String),
            OptionalPaymentMethod(serde_json::Value),
        }

        // This struct is an intermediate representation
        // This is required in order to catch deserialization errors when deserializing `payment_method_data`
        // The #[serde(flatten)] attribute applied on `payment_method_data` discards
        // any of the error when deserializing and deserializes to an option instead
        #[derive(serde::Deserialize, Debug)]
        struct __InnerPaymentMethodData {
            billing: Option<Address>,
            #[serde(flatten)]
            payment_method_data: Option<serde_json::Value>,
        }

        let deserialize_to_inner = __Inner::deserialize(deserializer)?;

        match deserialize_to_inner {
            __Inner::OptionalPaymentMethod(value) => {
                let parsed_value = serde_json::from_value::<__InnerPaymentMethodData>(value)
                    .map_err(|serde_json_error| de::Error::custom(serde_json_error.to_string()))?;

                let payment_method_data = if let Some(payment_method_data_value) =
                    parsed_value.payment_method_data
                {
                    // Even though no data is passed, the flatten serde_json::Value is deserialized as Some(Object {})
                    if let serde_json::Value::Object(ref inner_map) = payment_method_data_value {
                        if inner_map.is_empty() {
                            None
                        } else {
                            let payment_method_data = serde_json::from_value::<PaymentMethodData>(
                                payment_method_data_value,
                            )
                            .map_err(|serde_json_error| {
                                de::Error::custom(serde_json_error.to_string())
                            })?;
                            let address_details = parsed_value
                                .billing
                                .as_ref()
                                .and_then(|billing| billing.address.clone());
                            match (payment_method_data.clone(), address_details.as_ref()) {
                                (
                                    PaymentMethodData::Card(ref mut card),
                                    Some(billing_address_details),
                                ) => {
                                    if card.card_holder_name.is_none() {
                                        card.card_holder_name =
                                            billing_address_details.get_optional_full_name();
                                    }
                                    Some(PaymentMethodData::Card(card.clone()))
                                }
                                _ => Some(payment_method_data),
                            }
                        }
                    } else {
                        Err(de::Error::custom("Expected a map for payment_method_data"))?
                    }
                } else {
                    None
                };

                Ok(Some(PaymentMethodDataRequest {
                    payment_method_data,
                    billing: parsed_value.billing,
                }))
            }
            __Inner::RewardString(inner_string) => {
                let payment_method_data = match inner_string.as_str() {
                    "reward" => PaymentMethodData::Reward,
                    _ => Err(de::Error::custom("Invalid Variant"))?,
                };

                Ok(Some(PaymentMethodDataRequest {
                    payment_method_data: Some(payment_method_data),
                    billing: None,
                }))
            }
        }
    }

    pub fn serialize<S>(
        payment_method_data_request: &Option<PaymentMethodDataRequest>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(payment_method_data_request) = payment_method_data_request {
            if let Some(payment_method_data) =
                payment_method_data_request.payment_method_data.as_ref()
            {
                match payment_method_data {
                    PaymentMethodData::Reward => serializer.serialize_str("reward"),
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::Card(_)
                    | PaymentMethodData::MandatePayment
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::Wallet(_) => {
                        payment_method_data_request.serialize(serializer)
                    }
                }
            } else {
                payment_method_data_request.serialize(serializer)
            }
        } else {
            serializer.serialize_none()
        }
    }
}

/// The payment method information provided for making a payment
#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentMethodDataRequest {
    /// This field is optional because, in case of saved cards we pass the payment_token
    /// There might be cases where we don't need to pass the payment_method_data and pass only payment method billing details
    /// We have flattened it because to maintain backwards compatibility with the old API contract
    #[serde(flatten)]
    #[smithy(value_type = "Option<PaymentMethodData>")]
    pub payment_method_data: Option<PaymentMethodData>,
    /// billing details for the payment method.
    /// This billing details will be passed to the processor as billing address.
    /// If not passed, then payment.billing will be considered
    pub billing: Option<Address>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SplitPaymentMethodDataRequest {
    pub payment_method_data: PaymentMethodData,
    #[schema(value_type = PaymentMethod)]
    pub payment_method_type: api_enums::PaymentMethod,
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_subtype: api_enums::PaymentMethodType,
}

/// The payment method information provided for making a payment
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
pub struct RecordAttemptPaymentMethodDataRequest {
    /// Additional details for the payment method (e.g., card expiry date, card network).
    #[serde(flatten)]
    pub payment_method_data: AdditionalPaymentData,
    /// billing details for the payment method.
    pub billing: Option<Address>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
pub struct ProxyPaymentMethodDataRequest {
    /// This field is optional because, in case of saved cards we pass the payment_token
    /// There might be cases where we don't need to pass the payment_method_data and pass only payment method billing details
    /// We have flattened it because to maintain backwards compatibility with the old API contract
    #[serde(flatten)]
    pub payment_method_data: Option<ProxyPaymentMethodData>,
    /// billing details for the payment method.
    /// This billing details will be passed to the processor as billing address.
    /// If not passed, then payment.billing will be considered
    pub billing: Option<Address>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyPaymentMethodData {
    #[schema(title = "ProxyCardData")]
    VaultDataCard(Box<ProxyCardData>),
    VaultToken(VaultToken),
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ProxyCardData {
    /// The token which refers to the card number
    #[schema(value_type = String, example = "token_card_number")]
    pub card_number: Secret<String>,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = String, example = "242")]
    pub card_cvc: Secret<String>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: Option<api_enums::CardNetwork>,

    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,

    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,

    #[schema(example = "JP_AMEX")]
    pub bank_code: Option<String>,
    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    pub nick_name: Option<Secret<String>>,

    /// The first six digit of the card number
    #[schema(value_type = String, example = "424242")]
    pub bin_number: Option<String>,

    /// The last four digit of the card number
    #[schema(value_type = String, example = "4242")]
    pub last_four: Option<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct VaultToken {
    /// The tokenized CVC number for the card
    #[schema(value_type = String, example = "242")]
    pub card_cvc: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum PaymentMethodData {
    #[schema(title = "Card")]
    #[smithy(value_type = "Card")]
    Card(Card),
    #[schema(title = "CardRedirect")]
    #[smithy(value_type = "CardRedirectData")]
    CardRedirect(CardRedirectData),
    #[schema(title = "Wallet")]
    Wallet(WalletData),
    #[schema(title = "PayLater")]
    #[smithy(value_type = "PayLaterData")]
    PayLater(PayLaterData),
    #[schema(title = "BankRedirect")]
    #[smithy(value_type = "BankRedirectData")]
    BankRedirect(BankRedirectData),
    #[schema(title = "BankDebit")]
    #[smithy(value_type = "BankDebitData")]
    BankDebit(BankDebitData),
    #[schema(title = "BankTransfer")]
    #[smithy(value_type = "BankTransferData")]
    BankTransfer(Box<BankTransferData>),
    #[schema(title = "RealTimePayment")]
    #[smithy(value_type = "RealTimePaymentData")]
    RealTimePayment(Box<RealTimePaymentData>),
    #[schema(title = "Crypto")]
    #[smithy(value_type = "CryptoData")]
    Crypto(CryptoData),
    #[schema(title = "MandatePayment")]
    #[smithy(value_type = "smithy.api#Unit")]
    MandatePayment,
    #[schema(title = "Reward")]
    #[smithy(value_type = "smithy.api#Unit")]
    Reward,
    #[schema(title = "Upi")]
    #[smithy(value_type = "UpiData")]
    Upi(UpiData),
    #[schema(title = "Voucher")]
    #[smithy(value_type = "VoucherData")]
    Voucher(VoucherData),
    #[schema(title = "GiftCard")]
    #[smithy(value_type = "GiftCardData")]
    GiftCard(Box<GiftCardData>),
    #[schema(title = "CardToken")]
    #[smithy(value_type = "CardToken")]
    CardToken(CardToken),
    #[schema(title = "OpenBanking")]
    #[smithy(value_type = "OpenBankingData")]
    OpenBanking(OpenBankingData),
    #[schema(title = "MobilePayment")]
    #[smithy(value_type = "MobilePaymentData")]
    MobilePayment(MobilePaymentData),
}

pub trait GetAddressFromPaymentMethodData {
    fn get_billing_address(&self) -> Option<Address>;
}

impl GetAddressFromPaymentMethodData for PaymentMethodData {
    fn get_billing_address(&self) -> Option<Address> {
        match self {
            Self::Card(card_data) => card_data.get_billing_address(),
            Self::CardRedirect(_) => None,
            Self::Wallet(wallet_data) => wallet_data.get_billing_address(),
            Self::PayLater(pay_later) => pay_later.get_billing_address(),
            Self::BankRedirect(bank_redirect_data) => bank_redirect_data.get_billing_address(),
            Self::BankDebit(bank_debit_data) => bank_debit_data.get_billing_address(),
            Self::BankTransfer(bank_transfer_data) => bank_transfer_data.get_billing_address(),
            Self::Voucher(voucher_data) => voucher_data.get_billing_address(),
            Self::Crypto(_)
            | Self::Reward
            | Self::RealTimePayment(_)
            | Self::Upi(_)
            | Self::GiftCard(_)
            | Self::CardToken(_)
            | Self::OpenBanking(_)
            | Self::MandatePayment
            | Self::MobilePayment(_) => None,
        }
    }
}

impl PaymentMethodData {
    pub fn apply_additional_payment_data(
        &self,
        additional_payment_data: AdditionalPaymentData,
    ) -> Result<Self, error_stack::Report<ValidationError>> {
        if let AdditionalPaymentData::Card(additional_card_info) = additional_payment_data {
            match self {
                Self::Card(card) => Ok(Self::Card(
                    card.apply_additional_card_info(*additional_card_info)?,
                )),
                _ => Ok(self.to_owned()),
            }
        } else {
            Ok(self.to_owned())
        }
    }

    pub fn get_payment_method(&self) -> Option<api_enums::PaymentMethod> {
        match self {
            Self::Card(_) => Some(api_enums::PaymentMethod::Card),
            Self::CardRedirect(_) => Some(api_enums::PaymentMethod::CardRedirect),
            Self::Wallet(_) => Some(api_enums::PaymentMethod::Wallet),
            Self::PayLater(_) => Some(api_enums::PaymentMethod::PayLater),
            Self::BankRedirect(_) => Some(api_enums::PaymentMethod::BankRedirect),
            Self::BankDebit(_) => Some(api_enums::PaymentMethod::BankDebit),
            Self::BankTransfer(_) => Some(api_enums::PaymentMethod::BankTransfer),
            Self::RealTimePayment(_) => Some(api_enums::PaymentMethod::RealTimePayment),
            Self::Crypto(_) => Some(api_enums::PaymentMethod::Crypto),
            Self::Reward => Some(api_enums::PaymentMethod::Reward),
            Self::Upi(_) => Some(api_enums::PaymentMethod::Upi),
            Self::Voucher(_) => Some(api_enums::PaymentMethod::Voucher),
            Self::GiftCard(_) => Some(api_enums::PaymentMethod::GiftCard),
            Self::OpenBanking(_) => Some(api_enums::PaymentMethod::OpenBanking),
            Self::MobilePayment(_) => Some(api_enums::PaymentMethod::MobilePayment),
            Self::CardToken(_) | Self::MandatePayment => None,
        }
    }
}

pub trait GetPaymentMethodType {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType;
}

impl GetPaymentMethodType for CardRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Knet {} => api_enums::PaymentMethodType::Knet,
            Self::Benefit {} => api_enums::PaymentMethodType::Benefit,
            Self::MomoAtm {} => api_enums::PaymentMethodType::MomoAtm,
            Self::CardRedirect {} => api_enums::PaymentMethodType::CardRedirect,
        }
    }
}

impl GetPaymentMethodType for MobilePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::DirectCarrierBilling { .. } => api_enums::PaymentMethodType::DirectCarrierBilling,
        }
    }
}

impl GetPaymentMethodType for WalletData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::BluecodeRedirect {} => api_enums::PaymentMethodType::Bluecode,
            Self::AliPayQr(_) | Self::AliPayRedirect(_) => api_enums::PaymentMethodType::AliPay,
            Self::AliPayHkRedirect(_) => api_enums::PaymentMethodType::AliPayHk,
            Self::AmazonPay(_) | Self::AmazonPayRedirect(_) => {
                api_enums::PaymentMethodType::AmazonPay
            }
            Self::Skrill(_) => api_enums::PaymentMethodType::Skrill,
            Self::Paysera(_) => api_enums::PaymentMethodType::Paysera,
            Self::MomoRedirect(_) => api_enums::PaymentMethodType::Momo,
            Self::KakaoPayRedirect(_) => api_enums::PaymentMethodType::KakaoPay,
            Self::GoPayRedirect(_) => api_enums::PaymentMethodType::GoPay,
            Self::GcashRedirect(_) => api_enums::PaymentMethodType::Gcash,
            Self::ApplePay(_) | Self::ApplePayRedirect(_) | Self::ApplePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::ApplePay
            }
            Self::DanaRedirect {} => api_enums::PaymentMethodType::Dana,
            Self::GooglePay(_) | Self::GooglePayRedirect(_) | Self::GooglePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::GooglePay
            }
            Self::MbWayRedirect(_) => api_enums::PaymentMethodType::MbWay,
            Self::MobilePayRedirect(_) => api_enums::PaymentMethodType::MobilePay,
            Self::PaypalRedirect(_) | Self::PaypalSdk(_) => api_enums::PaymentMethodType::Paypal,
            Self::Paze(_) => api_enums::PaymentMethodType::Paze,
            Self::SamsungPay(_) => api_enums::PaymentMethodType::SamsungPay,
            Self::TwintRedirect {} => api_enums::PaymentMethodType::Twint,
            Self::VippsRedirect {} => api_enums::PaymentMethodType::Vipps,
            Self::TouchNGoRedirect(_) => api_enums::PaymentMethodType::TouchNGo,
            Self::WeChatPayRedirect(_) | Self::WeChatPayQr(_) => {
                api_enums::PaymentMethodType::WeChatPay
            }
            Self::CashappQr(_) => api_enums::PaymentMethodType::Cashapp,
            Self::SwishQr(_) => api_enums::PaymentMethodType::Swish,
            Self::Mifinity(_) => api_enums::PaymentMethodType::Mifinity,
            Self::RevolutPay(_) => api_enums::PaymentMethodType::RevolutPay,
        }
    }
}

impl GetPaymentMethodType for PayLaterData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::KlarnaRedirect { .. } => api_enums::PaymentMethodType::Klarna,
            Self::KlarnaSdk { .. } => api_enums::PaymentMethodType::Klarna,
            Self::AffirmRedirect {} => api_enums::PaymentMethodType::Affirm,
            Self::AfterpayClearpayRedirect { .. } => api_enums::PaymentMethodType::AfterpayClearpay,
            Self::PayBrightRedirect {} => api_enums::PaymentMethodType::PayBright,
            Self::FlexitiRedirect {} => api_enums::PaymentMethodType::Flexiti,
            Self::WalleyRedirect {} => api_enums::PaymentMethodType::Walley,
            Self::AlmaRedirect {} => api_enums::PaymentMethodType::Alma,
            Self::AtomeRedirect {} => api_enums::PaymentMethodType::Atome,
            Self::BreadpayRedirect {} => api_enums::PaymentMethodType::Breadpay,
            Self::PayjustnowRedirect {} => api_enums::PaymentMethodType::Payjustnow,
        }
    }
}

impl GetPaymentMethodType for OpenBankingData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::OpenBankingPIS {} => api_enums::PaymentMethodType::OpenBankingPIS,
        }
    }
}

impl GetPaymentMethodType for BankRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::BancontactCard { .. } => api_enums::PaymentMethodType::BancontactCard,
            Self::Bizum {} => api_enums::PaymentMethodType::Bizum,
            Self::Blik { .. } => api_enums::PaymentMethodType::Blik,
            Self::Eft { .. } => api_enums::PaymentMethodType::Eft,
            Self::Eps { .. } => api_enums::PaymentMethodType::Eps,
            Self::Giropay { .. } => api_enums::PaymentMethodType::Giropay,
            Self::Ideal { .. } => api_enums::PaymentMethodType::Ideal,
            Self::Interac { .. } => api_enums::PaymentMethodType::Interac,
            Self::OnlineBankingCzechRepublic { .. } => {
                api_enums::PaymentMethodType::OnlineBankingCzechRepublic
            }
            Self::OnlineBankingFinland { .. } => api_enums::PaymentMethodType::OnlineBankingFinland,
            Self::OnlineBankingPoland { .. } => api_enums::PaymentMethodType::OnlineBankingPoland,
            Self::OnlineBankingSlovakia { .. } => {
                api_enums::PaymentMethodType::OnlineBankingSlovakia
            }
            Self::OpenBankingUk { .. } => api_enums::PaymentMethodType::OpenBankingUk,
            Self::Przelewy24 { .. } => api_enums::PaymentMethodType::Przelewy24,
            Self::Sofort { .. } => api_enums::PaymentMethodType::Sofort,
            Self::Trustly { .. } => api_enums::PaymentMethodType::Trustly,
            Self::OnlineBankingFpx { .. } => api_enums::PaymentMethodType::OnlineBankingFpx,
            Self::OnlineBankingThailand { .. } => {
                api_enums::PaymentMethodType::OnlineBankingThailand
            }
            Self::LocalBankRedirect { .. } => api_enums::PaymentMethodType::LocalBankRedirect,
        }
    }
}

impl GetPaymentMethodType for BankDebitData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankDebit { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankDebit { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BecsBankDebit { .. } => api_enums::PaymentMethodType::Becs,
            Self::BacsBankDebit { .. } => api_enums::PaymentMethodType::Bacs,
            Self::SepaGuarenteedBankDebit { .. } => {
                api_enums::PaymentMethodType::SepaGuarenteedDebit
            }
        }
    }
}

impl GetPaymentMethodType for BankTransferData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankTransfer { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankTransfer { .. } => api_enums::PaymentMethodType::SepaBankTransfer,
            Self::BacsBankTransfer { .. } => api_enums::PaymentMethodType::Bacs,
            Self::MultibancoBankTransfer { .. } => api_enums::PaymentMethodType::Multibanco,
            Self::PermataBankTransfer { .. } => api_enums::PaymentMethodType::PermataBankTransfer,
            Self::BcaBankTransfer { .. } => api_enums::PaymentMethodType::BcaBankTransfer,
            Self::BniVaBankTransfer { .. } => api_enums::PaymentMethodType::BniVa,
            Self::BriVaBankTransfer { .. } => api_enums::PaymentMethodType::BriVa,
            Self::CimbVaBankTransfer { .. } => api_enums::PaymentMethodType::CimbVa,
            Self::DanamonVaBankTransfer { .. } => api_enums::PaymentMethodType::DanamonVa,
            Self::MandiriVaBankTransfer { .. } => api_enums::PaymentMethodType::MandiriVa,
            Self::Pix { .. } => api_enums::PaymentMethodType::Pix,
            Self::Pse {} => api_enums::PaymentMethodType::Pse,
            Self::LocalBankTransfer { .. } => api_enums::PaymentMethodType::LocalBankTransfer,
            Self::InstantBankTransfer {} => api_enums::PaymentMethodType::InstantBankTransfer,
            Self::InstantBankTransferFinland {} => {
                api_enums::PaymentMethodType::InstantBankTransferFinland
            }
            Self::InstantBankTransferPoland {} => {
                api_enums::PaymentMethodType::InstantBankTransferPoland
            }
            Self::IndonesianBankTransfer { .. } => {
                api_enums::PaymentMethodType::IndonesianBankTransfer
            }
        }
    }
}

impl GetPaymentMethodType for CryptoData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        api_enums::PaymentMethodType::CryptoCurrency
    }
}

impl GetPaymentMethodType for RealTimePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Fps {} => api_enums::PaymentMethodType::Fps,
            Self::DuitNow {} => api_enums::PaymentMethodType::DuitNow,
            Self::PromptPay {} => api_enums::PaymentMethodType::PromptPay,
            Self::VietQr {} => api_enums::PaymentMethodType::VietQr,
        }
    }
}

impl GetPaymentMethodType for UpiData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::UpiCollect(_) => api_enums::PaymentMethodType::UpiCollect,
            Self::UpiIntent(_) => api_enums::PaymentMethodType::UpiIntent,
            Self::UpiQr(_) => api_enums::PaymentMethodType::UpiQr,
        }
    }
}
impl GetPaymentMethodType for VoucherData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Boleto(_) => api_enums::PaymentMethodType::Boleto,
            Self::Efecty => api_enums::PaymentMethodType::Efecty,
            Self::PagoEfectivo => api_enums::PaymentMethodType::PagoEfectivo,
            Self::RedCompra => api_enums::PaymentMethodType::RedCompra,
            Self::RedPagos => api_enums::PaymentMethodType::RedPagos,
            Self::Alfamart(_) => api_enums::PaymentMethodType::Alfamart,
            Self::Indomaret(_) => api_enums::PaymentMethodType::Indomaret,
            Self::Oxxo => api_enums::PaymentMethodType::Oxxo,
            Self::SevenEleven(_) => api_enums::PaymentMethodType::SevenEleven,
            Self::Lawson(_) => api_enums::PaymentMethodType::Lawson,
            Self::MiniStop(_) => api_enums::PaymentMethodType::MiniStop,
            Self::FamilyMart(_) => api_enums::PaymentMethodType::FamilyMart,
            Self::Seicomart(_) => api_enums::PaymentMethodType::Seicomart,
            Self::PayEasy(_) => api_enums::PaymentMethodType::PayEasy,
        }
    }
}
impl GetPaymentMethodType for GiftCardData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Givex(_) => api_enums::PaymentMethodType::Givex,
            Self::PaySafeCard {} => api_enums::PaymentMethodType::PaySafeCard,
            Self::BhnCardNetwork(_) => api_enums::PaymentMethodType::BhnCardNetwork,
        }
    }
}

#[derive(
    serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum GiftCardData {
    #[smithy(value_type = "GiftCardDetails")]
    Givex(GiftCardDetails),
    #[smithy(nested_value_type)]
    PaySafeCard {},
    #[smithy(value_type = "BHNGiftCardDetails")]
    BhnCardNetwork(BHNGiftCardDetails),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BalanceCheckPaymentMethodData {
    GiftCard(GiftCardData),
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct ApplyPaymentMethodDataRequest {
    pub payment_methods: Vec<BalanceCheckPaymentMethodData>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PMBalanceCheckSuccessResponse {
    pub balance: MinorUnit,
    pub applicable_amount: MinorUnit,
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PMBalanceCheckFailureResponse {
    pub error: String,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PMBalanceCheckEligibilityResponse {
    Success(PMBalanceCheckSuccessResponse),
    Failure(PMBalanceCheckFailureResponse),
}

impl PMBalanceCheckEligibilityResponse {
    pub fn get_balance(&self) -> MinorUnit {
        match self {
            Self::Success(resp) => resp.balance,
            Self::Failure(_) => MinorUnit::zero(),
        }
    }
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct EligibilityBalanceCheckResponseItem {
    pub payment_method_data: BalanceCheckPaymentMethodData,
    pub eligibility: PMBalanceCheckEligibilityResponse,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct CheckAndApplyPaymentMethodDataResponse {
    pub balances: Vec<EligibilityBalanceCheckResponseItem>,
    /// The amount left after subtracting applied payment method balance from order amount
    pub remaining_amount: MinorUnit,
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,
    /// Whether the applied payment method balance is enough for the order amount or additional PM is required
    pub requires_additional_pm_data: bool,
    pub surcharge_details: Option<Vec<ApplyPaymentMethodDataSurchargeResponseItem>>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct ApplyPaymentMethodDataSurchargeResponseItem {
    #[schema(value_type = PaymentMethod)]
    pub payment_method_type: api_enums::PaymentMethod,
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_subtype: api_enums::PaymentMethodType,
    #[schema(value_type = Option<SurchargeDetailsResponse>)]
    pub surcharge_details: Option<payment_methods::SurchargeDetailsResponse>,
}

#[derive(
    serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BHNGiftCardDetails {
    /// The gift card or account number
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub account_number: Secret<String>,
    /// The security PIN for gift cards requiring it
    #[schema(value_type = String)]
    #[smithy(value_type = "Option<String>")]
    pub pin: Option<Secret<String>>,
    /// The CVV2 code for Open Loop/VPLN products
    #[schema(value_type = String)]
    #[smithy(value_type = "Option<String>")]
    pub cvv2: Option<Secret<String>>,
    /// The expiration date in MMYYYY format for Open Loop/VPLN products
    #[schema(value_type = String)]
    #[smithy(value_type = "Option<String>")]
    pub expiration_date: Option<String>,
}

#[derive(
    serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GiftCardDetails {
    /// The gift card number
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub number: Secret<String>,
    /// The card verification code.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub cvc: Secret<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AdditionalCardInfo {
    /// The name of issuer of the card
    pub card_issuer: Option<String>,

    /// Card network of the card
    pub card_network: Option<api_enums::CardNetwork>,

    /// Card type, can be either `credit` or `debit`
    pub card_type: Option<String>,

    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,

    /// Last 4 digits of the card number
    pub last4: Option<String>,

    /// The ISIN of the card
    pub card_isin: Option<String>,

    /// Extended bin of card, contains the first 8 digits of card number
    pub card_extended_bin: Option<String>,

    pub card_exp_month: Option<Secret<String>>,

    pub card_exp_year: Option<Secret<String>>,

    pub card_holder_name: Option<Secret<String>>,

    /// Additional payment checks done on the cvv and billing address by the processors.
    /// This is a free form field and the structure varies from processor to processor
    pub payment_checks: Option<serde_json::Value>,

    /// Details about the threeds environment.
    /// This is a free form field and the structure varies from processor to processor
    pub authentication_data: Option<serde_json::Value>,

    /// Indicates if the card issuer is regulated under government-imposed interchange fee caps.
    /// In the United States, this includes debit cards that fall under the Durbin Amendment,
    /// which imposes capped interchange fees.
    pub is_regulated: Option<bool>,

    /// The global signature network under which the card is issued.
    /// This represents the primary global card brand, even if the transaction uses a local network
    pub signature_network: Option<api_enums::CardNetwork>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalPaymentData {
    Card(Box<AdditionalCardInfo>),
    BankRedirect {
        bank_name: Option<common_enums::BankNames>,
        #[serde(flatten)]
        details: Option<BankRedirectDetails>,
        interac: Option<InteracPaymentMethod>,
    },
    Wallet {
        apple_pay: Option<Box<ApplepayPaymentMethod>>,
        google_pay: Option<Box<WalletAdditionalDataForCard>>,
        samsung_pay: Option<Box<WalletAdditionalDataForCard>>,
    },
    PayLater {
        klarna_sdk: Option<KlarnaSdkPaymentMethod>,
    },
    BankTransfer {
        #[serde(flatten)]
        details: Option<BankTransferAdditionalData>,
    },
    Crypto {
        #[serde(flatten)]
        details: Option<CryptoData>,
    },
    BankDebit {
        #[serde(flatten)]
        details: Option<BankDebitAdditionalData>,
    },
    MandatePayment {},
    Reward {},
    RealTimePayment {
        #[serde(flatten)]
        details: Option<RealTimePaymentData>,
    },
    Upi {
        #[serde(flatten)]
        details: Option<UpiAdditionalData>,
    },
    GiftCard {
        #[serde(flatten)]
        details: Option<GiftCardAdditionalData>,
    },
    Voucher {
        #[serde(flatten)]
        details: Option<VoucherData>,
    },
    CardRedirect {
        #[serde(flatten)]
        details: Option<CardRedirectData>,
    },
    CardToken {
        #[serde(flatten)]
        details: Option<CardTokenAdditionalData>,
    },
    OpenBanking {
        #[serde(flatten)]
        details: Option<OpenBankingData>,
    },
    MobilePayment {
        #[serde(flatten)]
        details: Option<MobilePaymentData>,
    },
}

impl AdditionalPaymentData {
    pub fn get_additional_card_info(&self) -> Option<AdditionalCardInfo> {
        match self {
            Self::Card(additional_card_info) => Some(*additional_card_info.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct KlarnaSdkPaymentMethod {
    pub payment_type: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct InteracPaymentMethod {
    #[schema(value_type = Option<Object>)]
    pub customer_info: Option<pii::SecretSerdeValue>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankRedirectData {
    #[smithy(nested_value_type)]
    BancontactCard {
        /// The card number
        #[schema(value_type = String, example = "4242424242424242")]
        #[smithy(value_type = "Option<String>")]
        card_number: Option<CardNumber>,
        /// The card's expiry month
        #[schema(value_type = String, example = "24")]
        #[smithy(value_type = "Option<String>")]
        card_exp_month: Option<Secret<String>>,

        /// The card's expiry year
        #[schema(value_type = String, example = "24")]
        #[smithy(value_type = "Option<String>")]
        card_exp_year: Option<Secret<String>>,

        /// The card holder's name
        #[schema(value_type = String, example = "John Test")]
        #[smithy(value_type = "Option<String>")]
        card_holder_name: Option<Secret<String>>,

        //Required by Stripes
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,
    },
    #[smithy(nested_value_type)]
    Bizum {},
    #[smithy(nested_value_type)]
    Blik {
        // Blik Code
        #[smithy(value_type = "Option<String>")]
        blik_code: Option<String>,
    },
    #[smithy(nested_value_type)]
    Eps {
        /// The billing details for bank redirection
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,

        /// The hyperswitch bank code for eps
        #[schema(value_type = BankNames, example = "triodos_bank")]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,
    },
    #[smithy(nested_value_type)]
    Giropay {
        /// The billing details for bank redirection
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,

        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        /// Bank account bic code
        bank_account_bic: Option<Secret<String>>,

        /// Bank account iban
        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        bank_account_iban: Option<Secret<String>>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,
    },
    #[smithy(nested_value_type)]
    Ideal {
        /// The billing details for bank redirection
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,

        /// The hyperswitch bank code for ideal
        #[schema(value_type = BankNames, example = "abn_amro")]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,
    },
    #[smithy(nested_value_type)]
    Interac {
        /// The country for bank payment
        #[schema(value_type = Option<CountryAlpha2>, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,

        #[schema(value_type = Option<String>, example = "john.doe@example.com")]
        #[smithy(value_type = "Option<String>")]
        email: Option<Email>,
    },
    #[smithy(nested_value_type)]
    OnlineBankingCzechRepublic {
        // Issuer banks
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "BankNames")]
        issuer: common_enums::BankNames,
    },
    #[smithy(nested_value_type)]
    OnlineBankingFinland {
        // Shopper Email
        #[schema(value_type = Option<String>)]
        #[smithy(value_type = "Option<String>")]
        email: Option<Email>,
    },
    #[smithy(nested_value_type)]
    OnlineBankingPoland {
        // Issuer banks
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "BankNames")]
        issuer: common_enums::BankNames,
    },
    #[smithy(nested_value_type)]
    OnlineBankingSlovakia {
        // Issuer value corresponds to the bank
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "BankNames")]
        issuer: common_enums::BankNames,
    },
    #[smithy(nested_value_type)]
    OpenBankingUk {
        // Issuer banks
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "Option<BankNames>")]
        issuer: Option<common_enums::BankNames>,
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,
    },
    #[smithy(nested_value_type)]
    Przelewy24 {
        //Issuer banks
        #[schema(value_type = Option<BankNames>)]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,

        // The billing details for bank redirect
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,
    },
    #[smithy(nested_value_type)]
    Sofort {
        /// The billing details for bank redirection
        #[smithy(value_type = "Option<BankRedirectBilling>")]
        billing_details: Option<BankRedirectBilling>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,

        /// The preferred language
        #[schema(example = "en")]
        #[smithy(value_type = "Option<String>")]
        preferred_language: Option<String>,
    },
    #[smithy(nested_value_type)]
    Trustly {
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "CountryAlpha2")]
        country: api_enums::CountryAlpha2,
    },
    #[smithy(nested_value_type)]
    OnlineBankingFpx {
        // Issuer banks
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "BankNames")]
        issuer: common_enums::BankNames,
    },
    #[smithy(nested_value_type)]
    OnlineBankingThailand {
        #[schema(value_type = BankNames)]
        #[smithy(value_type = "BankNames")]
        issuer: common_enums::BankNames,
    },
    #[smithy(nested_value_type)]
    LocalBankRedirect {},
    #[smithy(nested_value_type)]
    Eft {
        /// The preferred eft provider
        #[schema(example = "ozow")]
        #[smithy(value_type = "String")]
        provider: String,
    },
}

impl GetAddressFromPaymentMethodData for BankRedirectData {
    fn get_billing_address(&self) -> Option<Address> {
        let get_billing_address_inner = |bank_redirect_billing: Option<&BankRedirectBilling>,
                                         billing_country: Option<&common_enums::CountryAlpha2>,
                                         billing_email: Option<&Email>|
         -> Option<Address> {
            let address = bank_redirect_billing
                .and_then(GetAddressFromPaymentMethodData::get_billing_address);

            let address = match (address, billing_country) {
                (Some(mut address), Some(billing_country)) => {
                    address
                        .address
                        .as_mut()
                        .map(|address| address.country = Some(*billing_country));

                    Some(address)
                }
                (Some(address), None) => Some(address),
                (None, Some(billing_country)) => Some(Address {
                    address: Some(AddressDetails {
                        country: Some(*billing_country),
                        ..AddressDetails::default()
                    }),
                    phone: None,
                    email: None,
                }),
                (None, None) => None,
            };

            match (address, billing_email) {
                (Some(mut address), Some(email)) => {
                    address.email = Some(email.clone());
                    Some(address)
                }
                (Some(address), None) => Some(address),
                (None, Some(billing_email)) => Some(Address {
                    address: None,
                    phone: None,
                    email: Some(billing_email.clone()),
                }),
                (None, None) => None,
            }
        };

        match self {
            Self::BancontactCard {
                billing_details,
                card_holder_name,
                ..
            } => {
                let address = get_billing_address_inner(billing_details.as_ref(), None, None);

                if let Some(mut address) = address {
                    address.address.as_mut().map(|address| {
                        address.first_name = card_holder_name
                            .as_ref()
                            .or(address.first_name.as_ref())
                            .cloned();
                    });

                    Some(address)
                } else {
                    Some(Address {
                        address: Some(AddressDetails {
                            first_name: card_holder_name.clone(),
                            ..AddressDetails::default()
                        }),
                        phone: None,
                        email: None,
                    })
                }
            }
            Self::Eps {
                billing_details,
                country,
                ..
            }
            | Self::Giropay {
                billing_details,
                country,
                ..
            }
            | Self::Ideal {
                billing_details,
                country,
                ..
            }
            | Self::Sofort {
                billing_details,
                country,
                ..
            } => get_billing_address_inner(billing_details.as_ref(), country.as_ref(), None),
            Self::Interac { country, email } => {
                get_billing_address_inner(None, country.as_ref(), email.as_ref())
            }
            Self::OnlineBankingFinland { email } => {
                get_billing_address_inner(None, None, email.as_ref())
            }
            Self::OpenBankingUk { country, .. } => {
                get_billing_address_inner(None, country.as_ref(), None)
            }
            Self::Przelewy24 {
                billing_details, ..
            } => get_billing_address_inner(billing_details.as_ref(), None, None),
            Self::Trustly { country } => get_billing_address_inner(None, Some(country), None),
            Self::OnlineBankingFpx { .. }
            | Self::LocalBankRedirect {}
            | Self::OnlineBankingThailand { .. }
            | Self::Bizum {}
            | Self::OnlineBankingPoland { .. }
            | Self::OnlineBankingSlovakia { .. }
            | Self::OnlineBankingCzechRepublic { .. }
            | Self::Blik { .. }
            | Self::Eft { .. } => None,
        }
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AlfamartVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = Option<String>, example = "Jane")]
    #[smithy(value_type = "Option<String>")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Alfamart
    #[schema(value_type = Option<String>, example = "Doe")]
    #[smithy(value_type = "Option<String>")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct IndomaretVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = Option<String>, example = "Jane")]
    #[smithy(value_type = "Option<String>")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Alfamart
    #[schema(value_type = Option<String>, example = "Doe")]
    #[smithy(value_type = "Option<String>")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "Jane")]
    #[smithy(value_type = "Option<String>")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name Japanese convenience stores
    #[schema(value_type = Option<String>, example = "Doe")]
    #[smithy(value_type = "Option<String>")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
    /// The telephone number for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub phone_number: Option<String>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AchBillingDetails {
    /// The Email ID for ACH billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct DokuBillingDetails {
    /// The billing first name for Doku
    #[schema(value_type = Option<String>, example = "Jane")]
    #[smithy(value_type = "Option<String>")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Doku
    #[schema(value_type = Option<String>, example = "Doe")]
    #[smithy(value_type = "Option<String>")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Doku billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MultibancoBillingDetails {
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
    /// The billing name for SEPA and BACS billing
    #[schema(value_type = Option<String>, example = "Jane Doe")]
    #[smithy(value_type = "Option<String>")]
    pub name: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CryptoData {
    #[smithy(value_type = "Option<String>")]
    pub pay_currency: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub network: Option<String>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum UpiData {
    #[smithy(value_type = "UpiCollectData")]
    UpiCollect(UpiCollectData),
    #[smithy(value_type = "UpiIntentData")]
    UpiIntent(UpiIntentData),
    UpiQr(UpiQrData),
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct UpiCollectData {
    #[schema(value_type = Option<String>, example = "successtest@iata")]
    #[smithy(value_type = "Option<String>")]
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct UpiQrData {}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct UpiIntentData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SofortBilling {
    /// The country associated with the billing
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub billing_country: String,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankRedirectBilling {
    /// The name for which billing is issued
    #[schema(value_type = String, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub billing_name: Option<Secret<String>>,
    /// The billing email for bank redirect
    #[schema(value_type = String, example = "example@example.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

impl GetAddressFromPaymentMethodData for BankRedirectBilling {
    fn get_billing_address(&self) -> Option<Address> {
        let address_details = self
            .billing_name
            .as_ref()
            .map(|billing_name| AddressDetails {
                first_name: Some(billing_name.clone()),
                ..AddressDetails::default()
            });

        if address_details.is_some() || self.email.is_some() {
            Some(Address {
                address: address_details,
                phone: None,
                email: self.email.clone(),
            })
        } else {
            None
        }
    }
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankTransferData {
    #[smithy(nested_value_type)]
    AchBankTransfer {
        /// The billing details for ACH Bank Transfer
        #[smithy(value_type = "Option<AchBillingDetails>")]
        billing_details: Option<AchBillingDetails>,
    },
    #[smithy(nested_value_type)]
    SepaBankTransfer {
        /// The billing details for SEPA
        #[smithy(value_type = "Option<SepaAndBacsBillingDetails>")]
        billing_details: Option<SepaAndBacsBillingDetails>,

        /// The two-letter ISO country code for SEPA and BACS
        #[schema(value_type = CountryAlpha2, example = "US")]
        #[smithy(value_type = "Option<CountryAlpha2>")]
        country: Option<api_enums::CountryAlpha2>,
    },
    #[smithy(nested_value_type)]
    BacsBankTransfer {
        /// The billing details for SEPA
        #[smithy(value_type = "Option<SepaAndBacsBillingDetails>")]
        billing_details: Option<SepaAndBacsBillingDetails>,
    },
    #[smithy(nested_value_type)]
    MultibancoBankTransfer {
        /// The billing details for Multibanco
        #[smithy(value_type = "Option<MultibancoBillingDetails>")]
        billing_details: Option<MultibancoBillingDetails>,
    },
    #[smithy(nested_value_type)]
    PermataBankTransfer {
        /// The billing details for Permata Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    BcaBankTransfer {
        /// The billing details for BCA Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    BniVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    BriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    CimbVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    DanamonVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    MandiriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        #[smithy(value_type = "Option<DokuBillingDetails>")]
        billing_details: Option<DokuBillingDetails>,
    },
    #[smithy(nested_value_type)]
    Pix {
        /// Unique key for pix transfer
        #[schema(value_type = Option<String>, example = "a1f4102e-a446-4a57-bcce-6fa48899c1d1")]
        #[smithy(value_type = "Option<String>")]
        pix_key: Option<Secret<String>>,
        /// CPF is a Brazilian tax identification number
        #[schema(value_type = Option<String>, example = "10599054689")]
        #[smithy(value_type = "Option<String>")]
        cpf: Option<Secret<String>>,
        /// CNPJ is a Brazilian company tax identification number
        #[schema(value_type = Option<String>, example = "74469027417312")]
        #[smithy(value_type = "Option<String>")]
        cnpj: Option<Secret<String>>,
        /// Source bank account number
        #[schema(value_type = Option<String>, example = "8b******-****-****-****-*******08bc5")]
        #[smithy(value_type = "Option<String>")]
        source_bank_account_id: Option<MaskedBankAccount>,
        /// Partially masked destination bank account number _Deprecated: Will be removed in next stable release._
        #[schema(value_type = Option<String>, example = "********-****-460b-****-f23b4e71c97b", deprecated)]
        #[smithy(value_type = "Option<String>")]
        destination_bank_account_id: Option<MaskedBankAccount>,
        /// The expiration date and time for the Pix QR code in ISO 8601 format
        #[schema(value_type = Option<String>, example = "2025-09-10T10:11:12Z")]
        #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
        #[smithy(value_type = "Option<String>")]
        expiry_date: Option<PrimitiveDateTime>,
    },
    #[smithy(nested_value_type)]
    Pse {},
    #[smithy(nested_value_type)]
    LocalBankTransfer {
        #[smithy(value_type = "Option<String>")]
        bank_code: Option<String>,
    },
    #[smithy(nested_value_type)]
    InstantBankTransfer {},
    #[smithy(nested_value_type)]
    InstantBankTransferFinland {},
    #[smithy(nested_value_type)]
    InstantBankTransferPoland {},
    #[smithy(nested_value_type)]
    IndonesianBankTransfer {
        #[schema(value_type = Option<BankNames>, example = "bri")]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,
    },
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum RealTimePaymentData {
    #[smithy(nested_value_type)]
    Fps {},
    #[smithy(nested_value_type)]
    DuitNow {},
    #[smithy(nested_value_type)]
    PromptPay {},
    #[smithy(nested_value_type)]
    VietQr {},
}

impl GetAddressFromPaymentMethodData for BankTransferData {
    fn get_billing_address(&self) -> Option<Address> {
        match self {
            Self::AchBankTransfer { billing_details } => {
                billing_details.as_ref().map(|details| Address {
                    address: None,
                    phone: None,
                    email: details.email.clone(),
                })
            }
            Self::SepaBankTransfer {
                billing_details,
                country,
            } => billing_details.as_ref().map(|details| Address {
                address: Some(AddressDetails {
                    country: *country,
                    first_name: details.name.clone(),
                    ..AddressDetails::default()
                }),
                phone: None,
                email: details.email.clone(),
            }),
            Self::BacsBankTransfer { billing_details } => {
                billing_details.as_ref().map(|details| Address {
                    address: Some(AddressDetails {
                        first_name: details.name.clone(),
                        ..AddressDetails::default()
                    }),
                    phone: None,
                    email: details.email.clone(),
                })
            }
            Self::MultibancoBankTransfer { billing_details } => {
                billing_details.as_ref().map(|details| Address {
                    address: None,
                    phone: None,
                    email: details.email.clone(),
                })
            }
            Self::PermataBankTransfer { billing_details }
            | Self::BcaBankTransfer { billing_details }
            | Self::BniVaBankTransfer { billing_details }
            | Self::BriVaBankTransfer { billing_details }
            | Self::CimbVaBankTransfer { billing_details }
            | Self::DanamonVaBankTransfer { billing_details }
            | Self::MandiriVaBankTransfer { billing_details } => {
                billing_details.as_ref().map(|details| Address {
                    address: Some(AddressDetails {
                        first_name: details.first_name.clone(),
                        last_name: details.last_name.clone(),
                        ..AddressDetails::default()
                    }),
                    phone: None,
                    email: details.email.clone(),
                })
            }
            Self::LocalBankTransfer { .. }
            | Self::Pix { .. }
            | Self::Pse {}
            | Self::InstantBankTransfer {}
            | Self::InstantBankTransferFinland {}
            | Self::IndonesianBankTransfer { .. }
            | Self::InstantBankTransferPoland {} => None,
        }
    }
}

#[derive(
    serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankDebitBilling {
    /// The billing name for bank debits
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub name: Option<Secret<String>>,
    /// The billing email for bank debits
    #[schema(value_type = Option<String>, example = "example@example.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
    /// The billing address for bank debits
    #[smithy(value_type = "Option<AddressDetails>")]
    pub address: Option<AddressDetails>,
}

impl GetAddressFromPaymentMethodData for BankDebitBilling {
    fn get_billing_address(&self) -> Option<Address> {
        let address = if let Some(mut address) = self.address.clone() {
            address.first_name = self.name.clone().or(address.first_name);
            Address {
                address: Some(address),
                email: self.email.clone(),
                phone: None,
            }
        } else {
            Address {
                address: Some(AddressDetails {
                    first_name: self.name.clone(),
                    ..AddressDetails::default()
                }),
                email: self.email.clone(),
                phone: None,
            }
        };

        Some(address)
    }
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum WalletData {
    /// The wallet data for Ali Pay HK redirect
    #[schema(title = "AliPayHkRedirect")]
    #[smithy(value_type = "AliPayHkRedirection")]
    AliPayHkRedirect(AliPayHkRedirection),
    /// The wallet data for Ali Pay QrCode
    #[schema(title = "AliPayQr")]
    #[smithy(value_type = "AliPayQr")]
    AliPayQr(Box<AliPayQr>),
    /// The wallet data for Ali Pay redirect
    #[schema(title = "AliPayRedirect")]
    #[smithy(value_type = "AliPayRedirection")]
    AliPayRedirect(AliPayRedirection),
    /// The wallet data for Amazon Pay
    #[schema(title = "AmazonPay")]
    #[smithy(value_type = "AmazonPayWalletData")]
    AmazonPay(AmazonPayWalletData),
    /// The wallet data for Amazon Pay redirect
    #[schema(title = "AmazonPayRedirect")]
    #[smithy(value_type = "AmazonPayRedirectData")]
    AmazonPayRedirect(AmazonPayRedirectData),
    /// The wallet data for Apple pay
    #[schema(title = "ApplePay")]
    #[smithy(value_type = "ApplePayWalletData")]
    ApplePay(ApplePayWalletData),
    /// Wallet data for apple pay redirect flow
    #[schema(title = "ApplePayRedirect")]
    #[smithy(value_type = "ApplePayRedirectData")]
    ApplePayRedirect(Box<ApplePayRedirectData>),
    /// Wallet data for apple pay third party sdk flow
    #[schema(title = "ApplePayThirdPartySdk")]
    #[smithy(value_type = "ApplePayThirdPartySdkData")]
    ApplePayThirdPartySdk(Box<ApplePayThirdPartySdkData>),
    /// The wallet data for Bluecode QR Code Redirect
    #[schema(title = "BluecodeRedirect")]
    #[smithy(nested_value_type)]
    BluecodeRedirect {},
    /// The wallet data for Cashapp Qr
    #[schema(title = "CashappQr")]
    #[smithy(value_type = "CashappQr")]
    CashappQr(Box<CashappQr>),
    /// Wallet data for DANA redirect flow
    #[schema(title = "DanaRedirect")]
    #[smithy(nested_value_type)]
    DanaRedirect {},
    /// The wallet data for Gcash redirect
    #[schema(title = "GcashRedirect")]
    #[smithy(value_type = "GcashRedirection")]
    GcashRedirect(GcashRedirection),
    /// The wallet data for GoPay redirect
    #[schema(title = "GoPayRedirect")]
    #[smithy(value_type = "GoPayRedirection")]
    GoPayRedirect(GoPayRedirection),
    /// The wallet data for Google pay
    #[schema(title = "GooglePay")]
    #[smithy(value_type = "GooglePayWalletData")]
    GooglePay(GooglePayWalletData),
    /// Wallet data for google pay redirect flow
    #[schema(title = "GooglePayRedirect")]
    #[smithy(value_type = "GooglePayRedirectData")]
    GooglePayRedirect(Box<GooglePayRedirectData>),
    /// Wallet data for Google pay third party sdk flow
    #[schema(title = "GooglePayThirdPartySdk")]
    #[smithy(value_type = "GooglePayThirdPartySdkData")]
    GooglePayThirdPartySdk(Box<GooglePayThirdPartySdkData>),
    /// The wallet data for KakaoPay redirect
    #[schema(title = "KakaoPayRedirect")]
    #[smithy(value_type = "KakaoPayRedirection")]
    KakaoPayRedirect(KakaoPayRedirection),
    /// Wallet data for MbWay redirect flow
    #[schema(title = "MbWayRedirect")]
    #[smithy(value_type = "MbWayRedirection")]
    MbWayRedirect(Box<MbWayRedirection>),
    // The wallet data for Mifinity Ewallet
    #[schema(title = "Mifinity")]
    #[smithy(value_type = "MifinityData")]
    Mifinity(MifinityData),
    /// The wallet data for MobilePay redirect
    #[schema(title = "MobilePayRedirect")]
    #[smithy(value_type = "MobilePayRedirection")]
    MobilePayRedirect(Box<MobilePayRedirection>),
    /// The wallet data for Momo redirect
    #[schema(title = "MomoRedirect")]
    #[smithy(value_type = "MomoRedirection")]
    MomoRedirect(MomoRedirection),
    /// This is for paypal redirection
    #[schema(title = "PaypalRedirect")]
    #[smithy(value_type = "PaypalRedirection")]
    PaypalRedirect(PaypalRedirection),
    /// The wallet data for Paypal
    #[schema(title = "PaypalSdk")]
    #[smithy(value_type = "PayPalWalletData")]
    PaypalSdk(PayPalWalletData),
    /// The wallet data for Paysera
    #[schema(title = "Paysera")]
    #[smithy(value_type = "PayseraData")]
    Paysera(PayseraData),
    /// The wallet data for Paze
    #[schema(title = "Paze")]
    #[smithy(value_type = "PazeWalletData")]
    Paze(PazeWalletData),
    // The wallet data for RevolutPay
    #[schema(title = "RevolutPay")]
    #[smithy(value_type = "RevolutPayData")]
    RevolutPay(RevolutPayData),
    /// The wallet data for Samsung Pay
    #[schema(title = "SamsungPay")]
    #[smithy(value_type = "SamsungPayWalletData")]
    SamsungPay(Box<SamsungPayWalletData>),
    /// The wallet data for Skrill
    #[schema(title = "Skrill")]
    #[smithy(value_type = "SkrillData")]
    Skrill(SkrillData),
    // The wallet data for Swish
    #[schema(title = "SwishQr")]
    #[smithy(value_type = "SwishQrData")]
    SwishQr(SwishQrData),
    /// The wallet data for Touch n Go Redirection
    #[schema(title = "TouchNGoRedirect")]
    #[smithy(value_type = "TouchNGoRedirection")]
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    /// Wallet data for Twint Redirection
    #[schema(title = "TwintRedirect")]
    #[smithy(nested_value_type)]
    TwintRedirect {},
    /// Wallet data for Vipps Redirection
    #[schema(title = "VippsRedirect")]
    #[smithy(nested_value_type)]
    VippsRedirect {},
    /// The wallet data for WeChat Pay Display QrCode
    #[schema(title = "WeChatPayQr")]
    #[smithy(value_type = "WeChatPayQr")]
    WeChatPayQr(Box<WeChatPayQr>),
    /// The wallet data for WeChat Pay Redirection
    #[schema(title = "WeChatPayRedirect")]
    #[smithy(value_type = "WeChatPayRedirection")]
    WeChatPayRedirect(Box<WeChatPayRedirection>),
}

impl GetAddressFromPaymentMethodData for WalletData {
    fn get_billing_address(&self) -> Option<Address> {
        match self {
            Self::MbWayRedirect(mb_way_redirect) => {
                let phone = PhoneDetails {
                    // Portuguese country code, this payment method is applicable only in portugal
                    country_code: Some("+351".into()),
                    number: mb_way_redirect.telephone_number.clone(),
                };

                Some(Address {
                    phone: Some(phone),
                    address: None,
                    email: None,
                })
            }
            Self::MobilePayRedirect(_) => None,
            Self::PaypalRedirect(paypal_redirect) => {
                paypal_redirect.email.clone().map(|email| Address {
                    email: Some(email),
                    address: None,
                    phone: None,
                })
            }
            Self::Mifinity(_)
            | Self::AliPayQr(_)
            | Self::AliPayRedirect(_)
            | Self::AliPayHkRedirect(_)
            | Self::MomoRedirect(_)
            | Self::KakaoPayRedirect(_)
            | Self::GoPayRedirect(_)
            | Self::GcashRedirect(_)
            | Self::AmazonPay(_)
            | Self::AmazonPayRedirect(_)
            | Self::Skrill(_)
            | Self::Paysera(_)
            | Self::ApplePay(_)
            | Self::ApplePayRedirect(_)
            | Self::ApplePayThirdPartySdk(_)
            | Self::DanaRedirect {}
            | Self::GooglePay(_)
            | Self::GooglePayRedirect(_)
            | Self::GooglePayThirdPartySdk(_)
            | Self::PaypalSdk(_)
            | Self::Paze(_)
            | Self::SamsungPay(_)
            | Self::TwintRedirect {}
            | Self::VippsRedirect {}
            | Self::TouchNGoRedirect(_)
            | Self::WeChatPayRedirect(_)
            | Self::WeChatPayQr(_)
            | Self::CashappQr(_)
            | Self::SwishQr(_)
            | Self::RevolutPay(_)
            | Self::BluecodeRedirect {} => None,
        }
    }
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PazeWalletData {
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub complete_response: Secret<String>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayWalletData {
    #[smithy(value_type = "SamsungPayWalletCredentials")]
    pub payment_credential: SamsungPayWalletCredentials,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case", untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum SamsungPayWalletCredentials {
    #[smithy(value_type = "SamsungPayWebWalletData")]
    SamsungPayWalletDataForWeb(SamsungPayWebWalletData),
    #[smithy(value_type = "SamsungPayAppWalletData")]
    SamsungPayWalletDataForApp(SamsungPayAppWalletData),
}

impl From<SamsungPayCardBrand> for common_enums::SamsungPayCardBrand {
    fn from(samsung_pay_card_brand: SamsungPayCardBrand) -> Self {
        match samsung_pay_card_brand {
            SamsungPayCardBrand::Visa => Self::Visa,
            SamsungPayCardBrand::MasterCard => Self::MasterCard,
            SamsungPayCardBrand::Amex => Self::Amex,
            SamsungPayCardBrand::Discover => Self::Discover,
            SamsungPayCardBrand::Unknown => Self::Unknown,
        }
    }
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayAppWalletData {
    /// Samsung Pay token data
    #[serde(rename = "3_d_s")]
    #[smithy(value_type = "SamsungPayTokenData")]
    pub token_data: SamsungPayTokenData,
    /// Brand of the payment card
    #[smithy(value_type = "SamsungPayCardBrand")]
    pub payment_card_brand: SamsungPayCardBrand,
    /// Currency type of the payment
    #[smithy(value_type = "String")]
    pub payment_currency_type: String,
    /// Last 4 digits of the device specific card number
    #[smithy(value_type = "Option<String>")]
    pub payment_last4_dpan: Option<String>,
    /// Last 4 digits of the card number
    #[smithy(value_type = "String")]
    pub payment_last4_fpan: String,
    /// Merchant reference id that was passed in the session call request
    #[smithy(value_type = "Option<String>")]
    pub merchant_ref: Option<String>,
    /// Specifies authentication method used
    #[smithy(value_type = "Option<String>")]
    pub method: Option<String>,
    /// Value if credential is enabled for recurring payment
    #[smithy(value_type = "Option<bool>")]
    pub recurring_payment: Option<bool>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayWebWalletData {
    /// Specifies authentication method used
    #[smithy(value_type = "Option<String>")]
    pub method: Option<String>,
    /// Value if credential is enabled for recurring payment
    #[smithy(value_type = "Option<bool>")]
    pub recurring_payment: Option<bool>,
    /// Brand of the payment card
    #[smithy(value_type = "SamsungPayCardBrand")]
    pub card_brand: SamsungPayCardBrand,
    /// Last 4 digits of the card number
    #[serde(rename = "card_last4digits")]
    #[smithy(value_type = "String")]
    pub card_last_four_digits: String,
    /// Samsung Pay token data
    #[serde(rename = "3_d_s")]
    #[smithy(value_type = "SamsungPayTokenData")]
    pub token_data: SamsungPayTokenData,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayTokenData {
    /// 3DS type used by Samsung Pay
    #[serde(rename = "type")]
    #[smithy(value_type = "Option<String>")]
    pub three_ds_type: Option<String>,
    /// 3DS version used by Samsung Pay
    #[smithy(value_type = "String")]
    pub version: String,
    /// Samsung Pay encrypted payment credential data
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub data: Secret<String>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum SamsungPayCardBrand {
    #[serde(alias = "VI")]
    Visa,
    #[serde(alias = "MC")]
    MasterCard,
    #[serde(alias = "AX")]
    Amex,
    #[serde(alias = "DC")]
    Discover,
    #[serde(other)]
    Unknown,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum OpenBankingData {
    #[serde(rename = "open_banking_pis")]
    #[smithy(nested_value_type)]
    OpenBankingPIS {},
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum MobilePaymentData {
    #[smithy(nested_value_type)]
    DirectCarrierBilling {
        /// The phone number of the user
        #[schema(value_type = String, example = "1234567890")]
        #[smithy(value_type = "String")]
        msisdn: String,
        /// Unique user id
        #[schema(value_type = Option<String>, example = "02iacdYXGI9CnyJdoN8c7")]
        #[smithy(value_type = "Option<String>")]
        client_uid: Option<String>,
    },
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayWalletData {
    /// The type of payment method
    #[serde(rename = "type")]
    #[smithy(value_type = "String")]
    pub pm_type: String,
    /// User-facing message to describe the payment method that funds this transaction.
    #[smithy(value_type = "String")]
    pub description: String,
    /// The information of the payment method
    #[smithy(value_type = "GooglePayPaymentMethodInfo")]
    pub info: GooglePayPaymentMethodInfo,
    /// The tokenization data of Google pay
    #[schema(value_type = GpayTokenizationData)]
    #[smithy(value_type = "Object")]
    pub tokenization_data: common_types::payments::GpayTokenizationData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AmazonPaySessionTokenData {
    #[serde(rename = "amazon_pay")]
    pub data: AmazonPayMerchantCredentials,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AmazonPayMerchantCredentials {
    /// Amazon Pay merchant account identifier
    pub merchant_id: String,
    /// Amazon Pay store ID
    pub store_id: String,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayRedirectData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPayRedirectData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SkrillData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PayseraData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayRedirectData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayThirdPartySdkData {
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub token: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayThirdPartySdkData {
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub token: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct WeChatPayRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct WeChatPay {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct WeChatPayQr {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CashappQr {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaypalRedirection {
    /// paypal's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AliPayQr {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AliPayRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AliPayHkRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BluecodeQrRedirect {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MomoRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct KakaoPayRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GoPayRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GcashRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MobilePayRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    #[schema(value_type = String)]
    #[smithy(value_type = "Option<String>")]
    pub telephone_number: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    #[smithy(value_type = "String")]
    pub card_network: String,
    /// The details of the card
    #[smithy(value_type = "String")]
    pub card_details: String,
    //assurance_details of the card
    #[smithy(value_type = "Option<GooglePayAssuranceDetails>")]
    pub assurance_details: Option<GooglePayAssuranceDetails>,
    /// Card funding source for the selected payment method
    #[smithy(value_type = "Option<GooglePayCardFundingSource>")]
    pub card_funding_source: Option<GooglePayCardFundingSource>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayAssuranceDetails {
    ///indicates that Cardholder possession validation has been performed
    #[smithy(value_type = "bool")]
    pub card_holder_authenticated: bool,
    /// indicates that identification and verifications (ID&V) was performed
    #[smithy(value_type = "bool")]
    pub account_verified: bool,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    #[smithy(value_type = "String")]
    pub token: String,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct TouchNGoRedirection {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SwishQrData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RevolutPayData {}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MifinityData {
    #[schema(value_type = Date)]
    #[smithy(value_type = "String")]
    pub date_of_birth: Secret<Date>,
    #[smithy(value_type = "Option<String>")]
    pub language_preference: Option<String>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPayWalletData {
    /// Checkout Session identifier
    #[smithy(value_type = "String")]
    pub checkout_session_id: String,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    #[schema(value_type = ApplePayPaymentData)]
    #[smithy(value_type = "Object")]
    pub payment_data: common_types::payments::ApplePayPaymentData,
    /// The payment method of Apple pay
    #[smithy(value_type = "ApplepayPaymentMethod")]
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    #[smithy(value_type = "String")]
    pub transaction_identifier: String,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplepayPaymentMethod {
    /// The name to be displayed on Apple Pay button
    #[smithy(value_type = "String")]
    pub display_name: String,
    /// The network of the Apple pay payment method
    #[smithy(value_type = "String")]
    pub network: String,
    /// The type of the payment method
    #[serde(rename = "type")]
    #[smithy(value_type = "String")]
    pub pm_type: String,
    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "12")]
    pub card_exp_month: Option<Secret<String>>,
    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "25")]
    pub card_exp_year: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardResponse {
    #[smithy(value_type = "Option<String>")]
    pub last4: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub card_isin: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub card_extended_bin: Option<String>,
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_month: Option<Secret<String>>,
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_year: Option<Secret<String>>,
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,
    #[smithy(value_type = "Option<Object>")]
    pub payment_checks: Option<serde_json::Value>,
    #[smithy(value_type = "Option<Object>")]
    pub authentication_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RewardData {
    /// The merchant ID with which we have to call the connector
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BoletoVoucherData {
    /// The shopper's social security number (CPF or CNPJ)
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub social_security_number: Option<Secret<String>>,

    /// The shopper's bank account number associated with the boleto
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub bank_number: Option<Secret<String>>,

    /// The type of identification document used (e.g., CPF or CNPJ)
    #[schema(value_type = Option<DocumentKind>, example = "Cpf", default = "Cnpj")]
    #[smithy(value_type = "Option<DocumentKind>")]
    pub document_type: Option<common_enums::DocumentKind>,

    /// The fine percentage charged if payment is overdue
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub fine_percentage: Option<String>,

    /// The number of days after the due date when the fine is applied
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub fine_quantity_days: Option<String>,

    /// The interest percentage charged on late payments
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub interest_percentage: Option<String>,

    /// The number of days after which the boleto is written off (canceled)
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub write_off_quantity_days: Option<String>,

    /// Custom messages or instructions to display on the boleto
    #[schema(value_type = Option<Vec<String>>)]
    #[smithy(value_type = "Option<Vec<String>>")]
    pub messages: Option<Vec<String>>,

    // #[serde(with = "common_utils::custom_serde::date_yyyy_mm_dd::option")]
    #[schema(value_type = Option<String>, format = "date", example = "2025-08-22")]
    #[smithy(value_type = "Option<String>")]
    // The date upon which the boleto is due and is of format: "YYYY-MM-DD"
    pub due_date: Option<String>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum VoucherData {
    #[smithy(value_type = "BoletoVoucherData")]
    Boleto(Box<BoletoVoucherData>),
    #[smithy(value_type = "smithy.api#Unit")]
    Efecty,
    #[smithy(value_type = "smithy.api#Unit")]
    PagoEfectivo,
    #[smithy(value_type = "smithy.api#Unit")]
    RedCompra,
    #[smithy(value_type = "smithy.api#Unit")]
    RedPagos,
    #[smithy(value_type = "AlfamartVoucherData")]
    Alfamart(Box<AlfamartVoucherData>),
    #[smithy(value_type = "IndomaretVoucherData")]
    Indomaret(Box<IndomaretVoucherData>),
    #[smithy(value_type = "smithy.api#Unit")]
    Oxxo,
    #[smithy(value_type = "JCSVoucherData")]
    SevenEleven(Box<JCSVoucherData>),
    #[smithy(value_type = "JCSVoucherData")]
    Lawson(Box<JCSVoucherData>),
    #[smithy(value_type = "JCSVoucherData")]
    MiniStop(Box<JCSVoucherData>),
    #[smithy(value_type = "JCSVoucherData")]
    FamilyMart(Box<JCSVoucherData>),
    #[smithy(value_type = "JCSVoucherData")]
    Seicomart(Box<JCSVoucherData>),
    #[smithy(value_type = "JCSVoucherData")]
    PayEasy(Box<JCSVoucherData>),
}

impl GetAddressFromPaymentMethodData for VoucherData {
    fn get_billing_address(&self) -> Option<Address> {
        match self {
            Self::Alfamart(voucher_data) => Some(Address {
                address: Some(AddressDetails {
                    first_name: voucher_data.first_name.clone(),
                    last_name: voucher_data.last_name.clone(),
                    ..AddressDetails::default()
                }),
                phone: None,
                email: voucher_data.email.clone(),
            }),
            Self::Indomaret(voucher_data) => Some(Address {
                address: Some(AddressDetails {
                    first_name: voucher_data.first_name.clone(),
                    last_name: voucher_data.last_name.clone(),
                    ..AddressDetails::default()
                }),
                phone: None,
                email: voucher_data.email.clone(),
            }),
            Self::Lawson(voucher_data)
            | Self::MiniStop(voucher_data)
            | Self::FamilyMart(voucher_data)
            | Self::Seicomart(voucher_data)
            | Self::PayEasy(voucher_data)
            | Self::SevenEleven(voucher_data) => Some(Address {
                address: Some(AddressDetails {
                    first_name: voucher_data.first_name.clone(),
                    last_name: voucher_data.last_name.clone(),
                    ..AddressDetails::default()
                }),
                phone: Some(PhoneDetails {
                    number: voucher_data.phone_number.clone().map(Secret::new),
                    country_code: None,
                }),
                email: voucher_data.email.clone(),
            }),
            Self::Boleto(_)
            | Self::Efecty
            | Self::PagoEfectivo
            | Self::RedCompra
            | Self::RedPagos
            | Self::Oxxo => None,
        }
    }
}

/// Use custom serializer to provide backwards compatible response for `reward` payment_method_data
pub fn serialize_payment_method_data_response<S>(
    payment_method_data_response: &Option<PaymentMethodDataResponseWithBilling>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(payment_method_data_response) = payment_method_data_response {
        if let Some(payment_method_data) = payment_method_data_response.payment_method_data.as_ref()
        {
            match payment_method_data {
                PaymentMethodDataResponse::Reward {} => serializer.serialize_str("reward"),
                PaymentMethodDataResponse::BankDebit(_)
                | PaymentMethodDataResponse::BankRedirect(_)
                | PaymentMethodDataResponse::Card(_)
                | PaymentMethodDataResponse::CardRedirect(_)
                | PaymentMethodDataResponse::CardToken(_)
                | PaymentMethodDataResponse::Crypto(_)
                | PaymentMethodDataResponse::MandatePayment {}
                | PaymentMethodDataResponse::GiftCard(_)
                | PaymentMethodDataResponse::PayLater(_)
                | PaymentMethodDataResponse::RealTimePayment(_)
                | PaymentMethodDataResponse::MobilePayment(_)
                | PaymentMethodDataResponse::Upi(_)
                | PaymentMethodDataResponse::Wallet(_)
                | PaymentMethodDataResponse::BankTransfer(_)
                | PaymentMethodDataResponse::OpenBanking(_)
                | PaymentMethodDataResponse::Voucher(_) => {
                    payment_method_data_response.serialize(serializer)
                }
            }
        } else {
            // Can serialize directly because there is no `payment_method_data`
            payment_method_data_response.serialize(serializer)
        }
    } else {
        serializer.serialize_none()
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum PaymentMethodDataResponse {
    #[smithy(value_type = "CardResponse")]
    Card(Box<CardResponse>),
    #[smithy(value_type = "BankTransferResponse")]
    BankTransfer(Box<BankTransferResponse>),
    #[smithy(value_type = "WalletResponse")]
    Wallet(Box<WalletResponse>),
    #[smithy(value_type = "PaylaterResponse")]
    PayLater(Box<PaylaterResponse>),
    #[smithy(value_type = "BankRedirectResponse")]
    BankRedirect(Box<BankRedirectResponse>),
    #[smithy(value_type = "CryptoResponse")]
    Crypto(Box<CryptoResponse>),
    #[smithy(value_type = "BankDebitResponse")]
    BankDebit(Box<BankDebitResponse>),
    #[smithy(nested_value_type)]
    MandatePayment {},
    #[smithy(nested_value_type)]
    Reward {},
    #[smithy(value_type = "RealTimePaymentDataResponse")]
    RealTimePayment(Box<RealTimePaymentDataResponse>),
    #[smithy(value_type = "UpiResponse")]
    Upi(Box<UpiResponse>),
    #[smithy(value_type = "VoucherResponse")]
    Voucher(Box<VoucherResponse>),
    #[smithy(value_type = "GiftCardResponse")]
    GiftCard(Box<GiftCardResponse>),
    #[smithy(value_type = "CardRedirectResponse")]
    CardRedirect(Box<CardRedirectResponse>),
    #[smithy(value_type = "CardTokenResponse")]
    CardToken(Box<CardTokenResponse>),
    #[smithy(value_type = "OpenBankingResponse")]
    OpenBanking(Box<OpenBankingResponse>),
    #[smithy(value_type = "MobilePaymentResponse")]
    MobilePayment(Box<MobilePaymentResponse>),
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankDebitResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<BankDebitAdditionalData>)]
    #[smithy(value_type = "Option<BankDebitAdditionalData>")]
    details: Option<BankDebitAdditionalData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case", tag = "type")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankRedirectResponse {
    /// Name of the bank
    #[schema(value_type = Option<BankNames>)]
    #[smithy(value_type = "Option<BankNames>")]
    pub bank_name: Option<common_enums::BankNames>,
    #[serde(flatten)]
    #[schema(value_type = Option<BankRedirectDetails>)]
    #[smithy(value_type = "Option<BankRedirectDetails>")]
    pub details: Option<BankRedirectDetails>,
    /// customer info for interac payment method
    #[schema(value_type = Option<InteracPaymentMethod>)]
    pub interac: Option<InteracPaymentMethod>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankTransferResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<BankTransferAdditionalData>)]
    #[smithy(value_type = "Option<BankTransferAdditionalData>")]
    details: Option<BankTransferAdditionalData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardRedirectResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<CardRedirectData>")]
    details: Option<CardRedirectData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardTokenResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<CardTokenAdditionalData>)]
    #[smithy(value_type = "Option<CardTokenAdditionalData>")]
    details: Option<CardTokenAdditionalData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CryptoResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<CryptoData>")]
    details: Option<CryptoData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GiftCardResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<GiftCardAdditionalData>)]
    #[smithy(value_type = "Option<GiftCardAdditionalData>")]
    details: Option<GiftCardAdditionalData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct OpenBankingResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<OpenBankingData>")]
    details: Option<OpenBankingData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MobilePaymentResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<MobilePaymentData>")]
    details: Option<MobilePaymentData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RealTimePaymentDataResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<RealTimePaymentData>")]
    details: Option<RealTimePaymentData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct UpiResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<UpiAdditionalData>)]
    #[smithy(value_type = "Option<UpiAdditionalData>")]
    details: Option<UpiAdditionalData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct VoucherResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<VoucherData>")]
    details: Option<VoucherData>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaylaterResponse {
    #[smithy(value_type = "Option<KlarnaSdkPaymentMethodResponse>")]
    klarna_sdk: Option<KlarnaSdkPaymentMethodResponse>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct WalletResponse {
    #[serde(flatten)]
    #[smithy(value_type = "Option<WalletResponseData>")]
    details: Option<WalletResponseData>,
}

/// Hyperswitch supports SDK integration with Apple Pay and Google Pay wallets. For other wallets, we integrate with their respective connectors, redirecting the customer to the connector for wallet payments. As a result, we don’t receive any payment method data in the confirm call for payments made through other wallets.
#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum WalletResponseData {
    #[schema(value_type = WalletAdditionalDataForCard)]
    #[smithy(value_type = "Option<WalletAdditionalDataForCard>")]
    ApplePay(Box<WalletAdditionalDataForCard>),
    #[schema(value_type = WalletAdditionalDataForCard)]
    #[smithy(value_type = "Option<WalletAdditionalDataForCard>")]
    GooglePay(Box<WalletAdditionalDataForCard>),
    #[schema(value_type = WalletAdditionalDataForCard)]
    #[smithy(value_type = "Option<WalletAdditionalDataForCard>")]
    SamsungPay(Box<WalletAdditionalDataForCard>),
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct KlarnaSdkPaymentMethodResponse {
    #[smithy(value_type = "Option<String>")]
    pub payment_type: Option<String>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, ToSchema, serde::Serialize, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentMethodDataResponseWithBilling {
    // The struct is flattened in order to provide backwards compatibility
    #[serde(flatten)]
    #[smithy(value_type = "Option<PaymentMethodDataResponse>")]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub billing: Option<Address>,
}

impl PaymentMethodDataResponseWithBilling {
    pub fn get_card_network(&self) -> Option<common_enums::CardNetwork> {
        match self {
            Self {
                payment_method_data: Some(PaymentMethodDataResponse::Card(card)),
                ..
            } => card.card_network.clone(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, ToSchema, serde::Serialize)]
pub struct CustomRecoveryPaymentMethodData {
    /// Primary payment method token at payment processor end.
    #[schema(value_type = String, example = "token_1234")]
    pub primary_processor_payment_method_token: Secret<String>,

    /// AdditionalCardInfo for the primary token.
    pub additional_payment_method_info: AdditionalCardInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
#[cfg(feature = "v1")]
pub enum PaymentIdType {
    /// The identifier for payment intent
    PaymentIntentId(id_type::PaymentId),
    /// The identifier for connector transaction
    ConnectorTransactionId(String),
    /// The identifier for payment attempt
    PaymentAttemptId(String),
    /// The identifier for preprocessing step
    PreprocessingId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
#[cfg(feature = "v2")]
pub enum PaymentIdType {
    /// The identifier for payment intent
    PaymentIntentId(id_type::GlobalPaymentId),
    /// The identifier for connector transaction
    ConnectorTransactionId(String),
    /// The identifier for payment attempt
    PaymentAttemptId(String),
    /// The identifier for preprocessing step
    PreprocessingId(String),
}

#[cfg(feature = "v1")]
impl fmt::Display for PaymentIdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaymentIntentId(payment_id) => {
                write!(
                    f,
                    "payment_intent_id = \"{}\"",
                    payment_id.get_string_repr()
                )
            }
            Self::ConnectorTransactionId(connector_transaction_id) => write!(
                f,
                "connector_transaction_id = \"{connector_transaction_id}\""
            ),
            Self::PaymentAttemptId(payment_attempt_id) => {
                write!(f, "payment_attempt_id = \"{payment_attempt_id}\"")
            }
            Self::PreprocessingId(preprocessing_id) => {
                write!(f, "preprocessing_id = \"{preprocessing_id}\"")
            }
        }
    }
}

#[cfg(feature = "v1")]
impl Default for PaymentIdType {
    fn default() -> Self {
        Self::PaymentIntentId(Default::default())
    }
}

#[derive(
    Default,
    Clone,
    Debug,
    Eq,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    SmithyModel,
)]
// #[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct Address {
    /// Provide the address details
    #[smithy(value_type = "Option<AddressDetails>")]
    pub address: Option<AddressDetails>,

    #[smithy(value_type = "Option<PhoneDetails>")]
    pub phone: Option<PhoneDetails>,

    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
}

impl masking::SerializableSecret for Address {}

impl Address {
    /// Unify the address, giving priority to `self` when details are present in both
    pub fn unify_address(self, other: Option<&Self>) -> Self {
        let other_address_details = other.and_then(|address| address.address.as_ref());
        Self {
            address: self
                .address
                .map(|address| address.unify_address_details(other_address_details))
                .or(other_address_details.cloned()),
            email: self.email.or(other.and_then(|other| other.email.clone())),
            phone: self.phone.or(other.and_then(|other| other.phone.clone())),
        }
    }
}

// used by customers also, could be moved outside
/// Address details
#[derive(
    Clone,
    Default,
    Debug,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    ToSchema,
    SmithyModel,
)]
// #[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AddressDetails {
    /// The city, district, suburb, town, or village of the address.
    #[schema(max_length = 50, example = "New York")]
    #[smithy(value_type = "Option<String>")]
    pub city: Option<String>,

    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    #[smithy(value_type = "Option<CountryAlpha2>")]
    pub country: Option<api_enums::CountryAlpha2>,

    /// The first line of the street address or P.O. Box.
    #[schema(value_type = Option<String>, max_length = 200, example = "123, King Street")]
    #[smithy(value_type = "Option<String>")]
    pub line1: Option<Secret<String>>,

    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    #[schema(value_type = Option<String>, max_length = 50, example = "Powelson Avenue")]
    #[smithy(value_type = "Option<String>")]
    pub line2: Option<Secret<String>>,

    /// The third line of the street address, if applicable.
    #[schema(value_type = Option<String>, max_length = 50, example = "Bridgewater")]
    #[smithy(value_type = "Option<String>")]
    pub line3: Option<Secret<String>>,

    /// The zip/postal code for the address
    #[schema(value_type = Option<String>, max_length = 50, example = "08807")]
    #[smithy(value_type = "Option<String>")]
    pub zip: Option<Secret<String>>,

    /// The address state
    #[schema(value_type = Option<String>, example = "New York")]
    #[smithy(value_type = "Option<String>")]
    pub state: Option<Secret<String>>,

    /// The first name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "John")]
    #[smithy(value_type = "Option<String>")]
    pub first_name: Option<Secret<String>>,

    /// The last name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "Doe")]
    #[smithy(value_type = "Option<String>")]
    pub last_name: Option<Secret<String>>,

    /// The zip/postal code of the origin
    #[schema(value_type = Option<String>, max_length = 50, example = "08807")]
    #[smithy(value_type = "Option<String>")]
    pub origin_zip: Option<Secret<String>>,
}

impl AddressDetails {
    pub fn get_optional_full_name(&self) -> Option<Secret<String>> {
        match (self.first_name.as_ref(), self.last_name.as_ref()) {
            (Some(first_name), Some(last_name)) => Some(Secret::new(format!(
                "{} {}",
                first_name.peek(),
                last_name.peek()
            ))),
            (Some(name), None) | (None, Some(name)) => Some(name.to_owned()),
            _ => None,
        }
    }

    pub fn unify_address_details(self, other: Option<&Self>) -> Self {
        if let Some(other) = other {
            let (first_name, last_name) = if self
                .first_name
                .as_ref()
                .is_some_and(|first_name| !first_name.is_empty_after_trim())
            {
                (self.first_name, self.last_name)
            } else {
                (other.first_name.clone(), other.last_name.clone())
            };

            Self {
                first_name,
                last_name,
                city: self.city.or(other.city.clone()),
                country: self.country.or(other.country),
                line1: self.line1.or(other.line1.clone()),
                line2: self.line2.or(other.line2.clone()),
                line3: self.line3.or(other.line3.clone()),
                zip: self.zip.or(other.zip.clone()),
                state: self.state.or(other.state.clone()),
                origin_zip: self.origin_zip.or(other.origin_zip.clone()),
            }
        } else {
            self
        }
    }
}

pub struct AddressDetailsWithPhone {
    pub address: Option<AddressDetails>,
    pub phone_number: Option<Secret<String>>,
    pub email: Option<Email>,
}

pub struct EncryptableAddressDetails {
    pub line1: crypto::OptionalEncryptableSecretString,
    pub line2: crypto::OptionalEncryptableSecretString,
    pub line3: crypto::OptionalEncryptableSecretString,
    pub state: crypto::OptionalEncryptableSecretString,
    pub zip: crypto::OptionalEncryptableSecretString,
    pub first_name: crypto::OptionalEncryptableSecretString,
    pub last_name: crypto::OptionalEncryptableSecretString,
    pub phone_number: crypto::OptionalEncryptableSecretString,
    pub email: crypto::OptionalEncryptableEmail,
}

#[derive(
    Debug,
    Clone,
    Default,
    Eq,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PhoneDetails {
    /// The contact number
    #[schema(value_type = Option<String>, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub number: Option<Secret<String>>,
    /// The country code attached to the number
    #[schema(example = "+1")]
    #[smithy(value_type = "Option<String>")]
    pub country_code: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(
    Debug,
    Clone,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    ToSchema,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentsCaptureRequest {
    /// The unique identifier for the payment being captured. This is taken from the path parameter.
    #[serde(skip_deserializing)]
    pub payment_id: id_type::PaymentId,
    /// The unique identifier for the merchant. This is usually inferred from the API key.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub merchant_id: Option<id_type::MerchantId>,
    /// The amount to capture, in the lowest denomination of the currency. If omitted, the entire `amount_capturable` of the payment will be captured. Must be less than or equal to the current `amount_capturable`.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount_to_capture: Option<MinorUnit>,
    /// Decider to refund the uncaptured amount. (Currently not fully supported or behavior may vary by connector).
    #[smithy(value_type = "Option<bool>")]
    pub refund_uncaptured_amount: Option<bool>,
    /// A dynamic suffix that appears on your customer's credit card statement. This is concatenated with the (shortened) descriptor prefix set on your account to form the complete statement descriptor. The combined length should not exceed connector-specific limits (typically 22 characters).
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_suffix: Option<String>,
    /// An optional prefix for the statement descriptor that appears on your customer's credit card statement. This can override the default prefix set on your merchant account. The combined length of prefix and suffix should not exceed connector-specific limits (typically 22 characters).
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_prefix: Option<String>,
    /// Merchant connector details used to make payments. (Deprecated)
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>, deprecated)]
    #[smithy(value_type = "Option<MerchantConnectorDetailsWrap>")]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
    /// If true, returns stringified connector raw response body
    pub all_keys_required: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentsCaptureRequest {
    /// The Amount to be captured/ debited from the user's payment method. If not passed the full amount will be captured.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub amount_to_capture: Option<MinorUnit>,
    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct PaymentsCaptureResponse {
    /// The unique identifier for the payment
    pub id: id_type::GlobalPaymentId,

    /// Status of the payment
    #[schema(value_type = IntentStatus, example = "succeeded")]
    pub status: common_enums::IntentStatus,

    /// Amount details related to the payment
    pub amount: PaymentAmountDetailsResponse,
}

#[cfg(feature = "v2")]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsCancelRequest {
    /// The reason for the payment cancel
    pub cancellation_reason: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct PaymentsCancelResponse {
    /// The unique identifier for the payment
    pub id: id_type::GlobalPaymentId,

    /// Status of the payment
    #[schema(value_type = IntentStatus, example = "cancelled")]
    pub status: common_enums::IntentStatus,

    /// Cancellation reason for the payment cancellation
    #[schema(example = "Requested by merchant")]
    pub cancellation_reason: Option<String>,

    /// Amount details related to the payment
    pub amount: PaymentAmountDetailsResponse,

    /// The unique identifier for the customer associated with the payment
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// The connector used for the payment
    #[schema(example = "stripe")]
    pub connector: Option<api_enums::Connector>,

    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The payment method type for this payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: Option<api_enums::PaymentMethod>,

    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,

    /// List of payment attempts associated with payment intent
    pub attempts: Option<Vec<PaymentAttemptResponse>>,

    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<common_utils::types::Url>,

    /// Error details for the payment
    pub error: Option<ErrorDetails>,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct UrlDetails {
    pub url: String,
    pub method: String,
}
#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct AuthenticationForStartResponse {
    pub authentication: UrlDetails,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionType {
    RedirectToUrl,
    DisplayQrCode,
    InvokeSdkClient,
    TriggerApi,
    DisplayBankTransferInformation,
    DisplayWaitScreen,
    CollectOtp,
    RedirectInsidePopup,
    InvokeUpiIntentSdk,
    InvokeUpiQrFlow,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum NextActionData {
    /// Contains the url for redirection flow
    #[cfg(feature = "v1")]
    RedirectToUrl {
        #[smithy(value_type = "String")]
        redirect_to_url: String,
    },
    #[cfg(feature = "v1")]
    RedirectInsidePopup {
        #[smithy(value_type = "String")]
        popup_url: String,
        #[smithy(value_type = "String")]
        redirect_response_url: String,
    },
    /// Contains the url for redirection flow
    #[cfg(feature = "v2")]
    RedirectToUrl {
        #[schema(value_type = String)]
        redirect_to_url: Url,
    },
    /// Informs the next steps for bank transfer and also contains the charges details (ex: amount received, amount charged etc)
    DisplayBankTransferInformation {
        #[smithy(value_type = "BankTransferNextStepsData")]
        bank_transfer_steps_and_charges_details: BankTransferNextStepsData,
    },
    /// Contains third party sdk session token response
    ThirdPartySdkSessionToken {
        #[smithy(value_type = "Option<Object>")]
        session_token: Option<SessionToken>,
    },
    /// Contains url for Qr code image, this qr code has to be shown in sdk
    QrCodeInformation {
        #[schema(value_type = String)]
        #[smithy(value_type = "String")]
        /// Hyperswitch generated image data source url
        image_data_url: Option<Url>,
        #[smithy(value_type = "Option<i64>")]
        display_to_timestamp: Option<i64>,
        #[schema(value_type = String)]
        #[smithy(value_type = "String")]
        /// The url for Qr code given by the connector
        qr_code_url: Option<Url>,
        #[smithy(value_type = "Option<String>")]
        display_text: Option<String>,
        #[smithy(value_type = "Option<String>")]
        border_color: Option<String>,
    },
    /// Contains url to fetch Qr code data
    FetchQrCodeInformation {
        #[schema(value_type = String)]
        #[smithy(value_type = "String")]
        qr_code_fetch_url: Url,
    },
    InvokeUpiIntentSdk {
        #[schema(value_type = String)]
        sdk_uri: Url,
        #[smithy(value_type = "i128")]
        display_from_timestamp: i128,
        #[smithy(value_type = "Option<i128>")]
        display_to_timestamp: Option<i128>,
        #[smithy(value_type = "Option<PollConfig>")]
        poll_config: Option<PollConfig>,
    },
    InvokeUpiQrFlow {
        #[schema(value_type = String)]
        qr_code_url: Url,
        #[smithy(value_type = "i128")]
        display_from_timestamp: i128,
        #[smithy(value_type = "Option<i128>")]
        display_to_timestamp: Option<i128>,
        #[smithy(value_type = "Option<PollConfig>")]
        poll_config: Option<PollConfig>,
    },
    /// Contains the download url and the reference number for transaction
    DisplayVoucherInformation {
        #[schema(value_type = String)]
        #[smithy(value_type = "VoucherNextStepData")]
        voucher_details: VoucherNextStepData,
    },
    /// Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk
    WaitScreenInformation {
        #[smithy(value_type = "i128")]
        display_from_timestamp: i128,
        #[smithy(value_type = "Option<i128>")]
        display_to_timestamp: Option<i128>,
        #[smithy(value_type = "Option<PollConfig>")]
        poll_config: Option<PollConfig>,
    },
    /// Contains the information regarding three_ds_method_data submission, three_ds authentication, and authorization flows
    ThreeDsInvoke {
        #[smithy(value_type = "ThreeDsData")]
        three_ds_data: ThreeDsData,
    },
    InvokeSdkClient {
        #[smithy(value_type = "SdkNextActionData")]
        next_action_data: SdkNextActionData,
    },
    /// Contains consent to collect otp for mobile payment
    CollectOtp {
        #[smithy(value_type = "MobilePaymentConsent")]
        consent_data_required: MobilePaymentConsent,
    },
    /// Contains data required to invoke hidden iframe
    InvokeHiddenIframe {
        #[smithy(value_type = "IframeData")]
        iframe_data: IframeData,
    },
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(tag = "method_key")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum IframeData {
    #[serde(rename = "threeDSMethodData")]
    #[smithy(nested_value_type)]
    ThreedsInvokeAndCompleteAutorize {
        /// ThreeDS method url
        #[smithy(value_type = "String")]
        three_ds_method_url: String,
        /// Whether ThreeDS method data submission is required
        #[smithy(value_type = "bool")]
        three_ds_method_data_submission: bool,
        /// ThreeDS method data
        #[smithy(value_type = "Option<String>")]
        three_ds_method_data: Option<String>,
        /// ThreeDS Server ID
        #[smithy(value_type = "String")]
        directory_server_id: String,
        /// ThreeDS Protocol version
        #[smithy(value_type = "Option<String>")]
        message_version: Option<String>,
    },
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ThreeDsData {
    /// ThreeDS authentication url - to initiate authentication
    #[smithy(value_type = "String")]
    pub three_ds_authentication_url: String,
    /// ThreeDS authorize url - to complete the payment authorization after authentication
    #[smithy(value_type = "String")]
    pub three_ds_authorize_url: String,
    /// ThreeDS method details
    #[smithy(value_type = "ThreeDsMethodData")]
    pub three_ds_method_details: ThreeDsMethodData,
    /// Poll config for a connector
    #[smithy(value_type = "PollConfigResponse")]
    pub poll_config: PollConfigResponse,
    /// Message Version
    #[smithy(value_type = "Option<String>")]
    pub message_version: Option<String>,
    /// Directory Server ID
    #[smithy(value_type = "Option<String>")]
    pub directory_server_id: Option<String>,
    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,
    /// Prefered 3ds Connector
    #[smithy(value_type = "Option<String>")]
    pub three_ds_connector: Option<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ThreeDsMethodData {
    AcsThreeDsMethodData {
        /// Whether ThreeDS method data submission is required
        #[smithy(value_type = "bool")]
        three_ds_method_data_submission: bool,
        /// ThreeDS method data
        #[smithy(value_type = "Option<String>")]
        three_ds_method_data: Option<String>,
        /// ThreeDS method url
        #[smithy(value_type = "Option<String>")]
        three_ds_method_url: Option<String>,
        /// Three DS Method Key
        #[smithy(value_type = "Option<ThreeDsMethodKey>")]
        three_ds_method_key: Option<ThreeDsMethodKey>,
        /// Indicates whethere to wait for Post message after 3DS method data submission
        #[smithy(value_type = "bool")]
        consume_post_message_for_three_ds_method_completion: bool,
    },
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ThreeDsMethodKey {
    #[serde(rename = "threeDSMethodData")]
    ThreeDsMethodData,
    #[serde(rename = "JWT")]
    JWT,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PollConfigResponse {
    /// Poll Id
    #[smithy(value_type = "String")]
    pub poll_id: String,
    /// Interval of the poll
    #[smithy(value_type = "i8")]
    pub delay_in_secs: i8,
    /// Frequency of the poll
    #[smithy(value_type = "i8")]
    pub frequency: i8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
// the enum order shouldn't be changed as this is being used during serialization and deserialization
pub enum QrCodeInformation {
    QrCodeUrl {
        image_data_url: Url,
        qr_code_url: Url,
        display_to_timestamp: Option<i64>,
    },
    QrDataUrl {
        image_data_url: Url,
        display_to_timestamp: Option<i64>,
    },
    QrCodeImageUrl {
        qr_code_url: Url,
        display_to_timestamp: Option<i64>,
    },
    QrColorDataUrl {
        color_image_data_url: Url,
        display_to_timestamp: Option<i64>,
        display_text: Option<String>,
        border_color: Option<String>,
    },
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SdkNextActionData {
    #[smithy(value_type = "NextActionCall")]
    pub next_action: NextActionCall,
    #[smithy(value_type = "Option<String>")]
    pub order_id: Option<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct FetchQrCodeInformation {
    #[smithy(value_type = "String")]
    pub qr_code_fetch_url: Url,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BankTransferNextStepsData {
    /// The instructions for performing a bank transfer
    #[serde(flatten)]
    #[smithy(value_type = "BankTransferInstructions")]
    pub bank_transfer_instructions: BankTransferInstructions,
    /// The details received by the receiver
    #[smithy(value_type = "Option<ReceiverDetails>")]
    pub receiver: Option<ReceiverDetails>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct VoucherNextStepData {
    /// Voucher entry date
    #[smithy(value_type = "Option<String>")]
    pub entry_date: Option<String>,
    /// Voucher expiry date and time
    #[smithy(value_type = "Option<i64>")]
    pub expires_at: Option<i64>,
    /// Reference number required for the transaction
    #[smithy(value_type = "String")]
    pub reference: String,
    /// Url to download the payment instruction
    #[smithy(value_type = "Option<String>")]
    pub download_url: Option<Url>,
    /// Url to payment instruction page
    #[smithy(value_type = "Option<String>")]
    pub instructions_url: Option<Url>,
    /// Human-readable numeric version of the barcode.
    #[smithy(value_type = "Option<String>")]
    pub digitable_line: Option<Secret<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MobilePaymentNextStepData {
    /// is consent details required to be shown by sdk
    pub consent_data_required: MobilePaymentConsent,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum MobilePaymentConsent {
    ConsentRequired,
    ConsentNotRequired,
    ConsentOptional,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct QrCodeNextStepsInstruction {
    pub image_data_url: Url,
    pub display_to_timestamp: Option<i64>,
    pub qr_code_url: Option<Url>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WaitScreenInstructions {
    pub display_from_timestamp: i128,
    pub display_to_timestamp: Option<i128>,
    pub poll_config: Option<PollConfig>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkUpiUriInformation {
    pub sdk_uri: String,
}

impl NextActionData {
    pub fn from_upi_intent(sdk_uri: Url, wait_info: WaitScreenInstructions) -> Self {
        Self::InvokeUpiIntentSdk {
            sdk_uri,
            display_from_timestamp: wait_info.display_from_timestamp,
            display_to_timestamp: wait_info.display_to_timestamp,
            poll_config: wait_info.poll_config,
        }
    }

    pub fn from_upi_qr(qr_code_url: Url, wait_info: WaitScreenInstructions) -> Self {
        Self::InvokeUpiQrFlow {
            qr_code_url,
            display_from_timestamp: wait_info.display_from_timestamp,
            display_to_timestamp: wait_info.display_to_timestamp,
            poll_config: wait_info.poll_config,
        }
    }

    pub fn from_wait_screen(wait_info: WaitScreenInstructions) -> Self {
        Self::WaitScreenInformation {
            display_from_timestamp: wait_info.display_from_timestamp,
            display_to_timestamp: wait_info.display_to_timestamp,
            poll_config: wait_info.poll_config,
        }
    }
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PollConfig {
    /// Interval of the poll
    #[smithy(value_type = "u16")]
    pub delay_in_secs: u16,
    /// Frequency of the poll
    #[smithy(value_type = "u16")]
    pub frequency: u16,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankTransferInstructions {
    /// The instructions for Doku bank transactions
    #[smithy(value_type = "DokuBankTransferInstructions")]
    DokuBankTransferInstructions(Box<DokuBankTransferInstructions>),
    /// The credit transfer for ACH transactions
    #[smithy(value_type = "AchTransfer")]
    AchCreditTransfer(Box<AchTransfer>),
    /// The instructions for SEPA bank transactions
    #[smithy(value_type = "SepaBankTransferInstructions")]
    SepaBankInstructions(Box<SepaBankTransferInstructions>),
    /// The instructions for BACS bank transactions
    #[smithy(value_type = "BacsBankTransferInstructions")]
    BacsBankInstructions(Box<BacsBankTransferInstructions>),
    /// The instructions for Multibanco bank transactions
    #[smithy(value_type = "MultibancoTransferInstructions")]
    Multibanco(Box<MultibancoTransferInstructions>),
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SepaBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    #[smithy(value_type = "String")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "9123456789")]
    #[smithy(value_type = "String")]
    pub bic: Secret<String>,
    #[smithy(value_type = "String")]
    pub country: String,
    #[schema(value_type = String, example = "123456789")]
    #[smithy(value_type = "String")]
    pub iban: Secret<String>,
    #[schema(value_type = String, example = "U2PVVSEV4V9Y")]
    #[smithy(value_type = "String")]
    pub reference: Secret<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct PaymentsConnectorThreeDsInvokeData {
    pub directory_server_id: String,
    pub three_ds_method_url: String,
    pub three_ds_method_data: String,
    pub message_version: Option<String>,
    pub three_ds_method_data_submission: bool,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BacsBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    #[smithy(value_type = "String")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "10244123908")]
    #[smithy(value_type = "String")]
    pub account_number: Secret<String>,
    #[schema(value_type = String, example = "012")]
    #[smithy(value_type = "String")]
    pub sort_code: Secret<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MultibancoTransferInstructions {
    #[schema(value_type = String, example = "122385736258")]
    #[smithy(value_type = "String")]
    pub reference: Secret<String>,
    #[schema(value_type = String, example = "12345")]
    #[smithy(value_type = "String")]
    pub entity: String,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct DokuBankTransferInstructions {
    #[schema(value_type = String, example = "1707091200000")]
    #[smithy(value_type = "String")]
    pub expires_at: Option<i64>,
    #[schema(value_type = String, example = "122385736258")]
    #[smithy(value_type = "String")]
    pub reference: Secret<String>,
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub instructions_url: Option<Url>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AchTransfer {
    #[schema(value_type = String, example = "122385736258")]
    #[smithy(value_type = "String")]
    pub account_number: Secret<String>,
    #[smithy(value_type = "String")]
    pub bank_name: String,
    #[schema(value_type = String, example = "012")]
    #[smithy(value_type = "String")]
    pub routing_number: Secret<String>,
    #[schema(value_type = String, example = "234")]
    #[smithy(value_type = "String")]
    pub swift_code: Secret<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ReceiverDetails {
    /// The amount received by receiver
    #[smithy(value_type = "i64")]
    amount_received: i64,
    /// The amount charged by ACH
    #[smithy(value_type = "Option<i64>")]
    amount_charged: Option<i64>,
    /// The amount remaining to be sent via ACH
    #[smithy(value_type = "Option<i64>")]
    amount_remaining: Option<i64>,
}

#[cfg(feature = "v1")]
#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    ToSchema,
    router_derive::PolymorphicSchema,
    SmithyModel,
)]
#[generate_schemas(PaymentsCreateResponseOpenApi)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentsResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    #[smithy(value_type = "String")]
    pub payment_id: id_type::PaymentId,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825", value_type = String)]
    #[smithy(value_type = "String")]
    pub merchant_id: id_type::MerchantId,

    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    #[smithy(value_type = "IntentStatus")]
    pub status: api_enums::IntentStatus,

    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,

    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount,
    /// If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub net_amount: MinorUnit,

    /// The shipping cost for the payment.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub shipping_cost: Option<MinorUnit>,

    /// The amount (in minor units) that can still be captured for this payment. This is relevant when `capture_method` is `manual`. Once fully captured, or if `capture_method` is `automatic` and payment succeeded, this will be 0.
    #[schema(value_type = i64, minimum = 100, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount_capturable: MinorUnit,

    /// The total amount (in minor units) that has been captured for this payment. For `fauxpay` sandbox connector, this might reflect the authorized amount if `status` is `succeeded` even if `capture_method` was `manual`.
    #[schema(value_type = Option<i64>, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount_received: Option<MinorUnit>,

    /// The name of the payment connector (e.g., 'stripe', 'adyen') that processed or is processing this payment.
    #[schema(example = "stripe")]
    #[smithy(value_type = "Option<String>")]
    pub connector: Option<String>,

    /// A secret token unique to this payment intent. It is primarily used by client-side applications (e.g., Hyperswitch SDKs) to authenticate actions like confirming the payment or handling next actions. This secret should be handled carefully and not exposed publicly beyond its intended client-side use.
    #[schema(value_type = Option<String>, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    #[smithy(value_type = "Option<String>")]
    pub client_secret: Option<Secret<String>>,

    /// Timestamp indicating when this payment intent was created, in ISO 8601 format.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub created: Option<PrimitiveDateTime>,

    /// Three-letter ISO currency code (e.g., USD, EUR) for the payment amount.
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency: String,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    /// This field will be deprecated soon. Please refer to `customer.id`
    #[schema(
        max_length = 64,
        min_length = 1,
        example = "cus_y3oqhf46pyzuxjbcn2giaqnb44",
        deprecated,
        value_type = Option<String>,
    )]
    #[smithy(value_type = "Option<String>")]
    pub customer_id: Option<id_type::CustomerId>,

    #[smithy(value_type = "Option<CustomerDetailsResponse>")]
    pub customer: Option<CustomerDetailsResponse>,

    /// An arbitrary string providing a description for the payment, often useful for display or internal record-keeping.
    #[schema(example = "It's my first payment request")]
    #[smithy(value_type = "Option<String>")]
    pub description: Option<String>,

    /// An array of refund objects associated with this payment. Empty or null if no refunds have been processed.
    #[schema(value_type = Option<Vec<RefundResponse>>)]
    #[smithy(value_type = "Option<Vec<RefundResponse>>")]
    pub refunds: Option<Vec<refunds::RefundResponse>>,

    /// List of disputes that happened on this intent
    #[schema(value_type = Option<Vec<DisputeResponsePaymentsRetrieve>>)]
    #[smithy(value_type = "Option<Vec<DisputeResponsePaymentsRetrieve>>")]
    pub disputes: Option<Vec<disputes::DisputeResponsePaymentsRetrieve>>,

    /// List of attempts that happened on this intent
    #[schema(value_type = Option<Vec<PaymentAttemptResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<Vec<PaymentAttemptResponse>>")]
    pub attempts: Option<Vec<PaymentAttemptResponse>>,

    /// List of captures done on latest attempt
    #[schema(value_type = Option<Vec<CaptureResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<Vec<CaptureResponse>>")]
    pub captures: Option<Vec<CaptureResponse>>,

    /// A unique identifier to link the payment to a mandate, can be used instead of payment_method_data, in case of setting up recurring payments
    #[schema(max_length = 255, example = "mandate_iwer89rnjef349dni3")]
    #[smithy(value_type = "Option<String>")]
    pub mandate_id: Option<String>,

    /// Provided mandate information for creating a mandate
    #[smithy(value_type = "Option<MandateData>")]
    pub mandate_data: Option<MandateData>,

    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    #[smithy(value_type = "Option<FutureUsage>")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    #[schema(example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub off_session: Option<bool>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[remove_in(PaymentsCreateResponseOpenApi)]
    #[smithy(value_type = "Option<String>")]
    pub capture_on: Option<PrimitiveDateTime>,

    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    #[smithy(value_type = "Option<CaptureMethod>")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    /// The payment method that is to be used
    #[schema(value_type = PaymentMethod, example = "bank_transfer")]
    #[smithy(value_type = "Option<PaymentMethod>")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethodDataResponseWithBilling>, example = "bank_transfer")]
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    #[smithy(value_type = "Option<PaymentMethodDataResponseWithBilling>")]
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,

    /// Provide a reference to a stored payment method
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    #[smithy(value_type = "Option<String>")]
    pub payment_token: Option<String>,

    /// The shipping address for the payment
    #[smithy(value_type = "Option<Address>")]
    pub shipping: Option<Address>,

    /// The billing address for the payment
    #[smithy(value_type = "Option<Address>")]
    pub billing: Option<Address>,

    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "gillete creme",
        "quantity": 15,
        "amount" : 900
    }]"#)]
    #[smithy(value_type = "Option<Vec<OrderDetailsWithAmount>>")]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,

    /// description: The customer's email address
    /// This field will be deprecated soon. Please refer to `customer.email` object
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub email: crypto::OptionalEncryptableEmail,

    /// description: The customer's name
    /// This field will be deprecated soon. Please refer to `customer.name` object
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub name: crypto::OptionalEncryptableName,

    /// The customer's phone number
    /// This field will be deprecated soon. Please refer to `customer.phone` object
    #[schema(value_type = Option<String>, max_length = 255, example = "9123456789", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub phone: crypto::OptionalEncryptablePhone,

    /// The URL to redirect after the completion of the operation
    #[schema(example = "https://hyperswitch.io")]
    #[smithy(value_type = "Option<String>")]
    pub return_url: Option<String>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS, as the 3DS method helps with more robust payer authentication
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    #[smithy(value_type = "Option<AuthenticationType>")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 255, example = "Hyperswitch Router")]
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor.
    #[schema(max_length = 255, example = "Payment for shoes purchase")]
    #[smithy(value_type = "Option<String>")]
    pub statement_descriptor_suffix: Option<String>,

    /// If the payment requires further action from the customer (e.g., 3DS authentication, redirect to a bank page), this object will contain the necessary information for the client to proceed. Null if no further action is needed from the customer at this stage.
    #[smithy(value_type = "Option<NextActionData>")]
    pub next_action: Option<NextActionData>,

    /// If the payment intent was cancelled, this field provides a textual reason for the cancellation (e.g., "requested_by_customer", "abandoned").
    #[smithy(value_type = "Option<String>")]
    pub cancellation_reason: Option<String>,

    /// The connector-specific error code from the last failed payment attempt associated with this payment intent.
    #[schema(example = "E0001")]
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,

    /// A human-readable error message from the last failed payment attempt associated with this payment intent.
    #[schema(example = "Failed while verifying the card")]
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,

    #[schema(example = "Insufficient Funds")]
    #[smithy(value_type = "Option<String>")]
    pub error_reason: Option<String>,

    /// error code unified across the connectors is received here if there was an error while calling connector
    #[remove_in(PaymentsCreateResponseOpenApi)]
    #[smithy(value_type = "Option<String>")]
    pub unified_code: Option<String>,

    /// error message unified across the connectors is received here if there was an error while calling connector
    #[remove_in(PaymentsCreateResponseOpenApi)]
    #[smithy(value_type = "Option<String>")]
    pub unified_message: Option<String>,

    /// Describes the type of payment flow experienced by the customer (e.g., 'redirect_to_url', 'invoke_sdk', 'display_qr_code').
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    #[smithy(value_type = "Option<PaymentExperience>")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// The specific payment method subtype used for this payment (e.g., 'credit_card', 'klarna', 'gpay'). This provides more granularity than the 'payment_method' field.
    #[schema(value_type = Option<PaymentMethodType>, example = "gpay")]
    #[smithy(value_type = "Option<PaymentMethodType>")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// A label identifying the specific merchant connector account (MCA) used for this payment. This often combines the connector name, business country, and a custom label (e.g., "stripe_US_primary").
    #[schema(example = "stripe_US_food")]
    #[smithy(value_type = "Option<String>")]
    pub connector_label: Option<String>,

    /// The two-letter ISO country code (e.g., US, GB) of the business unit or profile under which this payment was processed.
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    #[smithy(value_type = "Option<CountryAlpha2>")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// The label identifying the specific business unit or profile under which this payment was processed by the merchant.
    #[smithy(value_type = "Option<String>")]
    pub business_label: Option<String>,

    /// An optional sub-label for further categorization of the business unit or profile used for this payment.
    #[smithy(value_type = "Option<String>")]
    pub business_sub_label: Option<String>,

    /// Allowed Payment Method Types for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    #[smithy(value_type = "Option<Vec<PaymentMethodType>>")]
    pub allowed_payment_method_types: Option<serde_json::Value>,

    /// ephemeral_key for the customer_id mentioned
    #[smithy(value_type = "Option<EphemeralKeyCreateResponse>")]
    pub ephemeral_key: Option<EphemeralKeyCreateResponse>,

    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    #[smithy(value_type = "Option<bool>")]
    pub manual_retry_allowed: Option<bool>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    #[smithy(value_type = "Option<String>")]
    pub connector_transaction_id: Option<String>,

    /// Frm message contains information about the frm response
    #[smithy(value_type = "Option<FrmMessage>")]
    pub frm_message: Option<FrmMessage>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<serde_json::Value>,

    /// Additional data related to some connectors
    #[schema(value_type = Option<ConnectorMetadata>)]
    #[smithy(value_type = "Option<ConnectorMetadata>")]
    pub connector_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    #[schema(value_type = Option<FeatureMetadata>)]
    #[smithy(value_type = "Option<FeatureMetadata>")]
    pub feature_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    #[smithy(value_type = "Option<String>")]
    pub reference_id: Option<String>,

    /// Details for Payment link
    pub payment_link: Option<PaymentLinkResponse>,
    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub profile_id: Option<id_type::ProfileId>,

    /// Details of surcharge applied on this payment
    #[smithy(value_type = "Option<RequestSurchargeDetails>")]
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// Total number of attempts associated with this payment
    #[smithy(value_type = "i16")]
    pub attempt_count: i16,

    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    #[smithy(value_type = "Option<String>")]
    pub merchant_decision: Option<String>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// If true, incremental authorization can be performed on this payment, in case the funds authorized initially fall short.
    #[smithy(value_type = "Option<bool>")]
    pub incremental_authorization_allowed: Option<bool>,

    /// Total number of authorizations happened in an incremental_authorization payment
    #[smithy(value_type = "Option<i32>")]
    pub authorization_count: Option<i32>,

    /// List of incremental authorizations happened to the payment
    #[smithy(value_type = "Option<Vec<IncrementalAuthorizationResponse>>")]
    pub incremental_authorizations: Option<Vec<IncrementalAuthorizationResponse>>,

    /// Details of external authentication
    #[smithy(value_type = "Option<ExternalAuthenticationDetailsResponse>")]
    pub external_authentication_details: Option<ExternalAuthenticationDetailsResponse>,

    /// Flag indicating if external 3ds authentication is made or not
    #[smithy(value_type = "Option<bool>")]
    pub external_3ds_authentication_attempted: Option<bool>,

    /// Date Time for expiry of the payment
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub expires_on: Option<PrimitiveDateTime>,

    /// Payment Fingerprint, to identify a particular card.
    /// It is a 20 character long alphanumeric code.
    #[smithy(value_type = "Option<String>")]
    pub fingerprint: Option<String>,

    #[schema(value_type = Option<BrowserInformation>)]
    /// The browser information used for this payment
    #[smithy(value_type = "Option<BrowserInformation>")]
    pub browser_info: Option<serde_json::Value>,

    /// Indicates how the payment was initiated (e.g., ecommerce, mail, or telephone).
    #[schema(value_type = Option<PaymentChannel>)]
    #[smithy(value_type = "Option<PaymentChannel>")]
    pub payment_channel: Option<common_enums::PaymentChannel>,

    /// A unique identifier for the payment method used in this payment. If the payment method was saved or tokenized, this ID can be used to reference it for future transactions or recurring payments.
    #[smithy(value_type = "Option<String>")]
    pub payment_method_id: Option<String>,

    /// The network transaction ID is a unique identifier for the transaction as recognized by the payment network (e.g., Visa, Mastercard), this ID can be used to reference it for future transactions or recurring payments.
    #[smithy(value_type = "Option<String>")]
    pub network_transaction_id: Option<String>,

    /// Payment Method Status, refers to the status of the payment method used for this payment.
    #[schema(value_type = Option<PaymentMethodStatus>)]
    #[smithy(value_type = "Option<PaymentMethodStatus>")]
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,

    /// Date time at which payment was updated
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub updated: Option<PrimitiveDateTime>,

    /// Fee information to be charged on the payment being collected
    #[schema(value_type = Option<ConnectorChargeResponseData>)]
    #[smithy(value_type = "Option<ConnectorChargeResponseData>")]
    pub split_payments: Option<common_types::payments::ConnectorChargeResponseData>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. FRM Metadata is useful for storing additional, structured information on an object related to FRM.
    #[schema(value_type = Option<Object>, example = r#"{ "fulfillment_method" : "deliver", "coverage_request" : "fraud" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// flag that indicates if extended authorization is applied on this payment or not
    #[schema(value_type = Option<bool>)]
    #[smithy(value_type = "Option<bool>")]
    pub extended_authorization_applied: Option<ExtendedAuthorizationAppliedBool>,

    /// date and time at which extended authorization was last applied on this payment
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub extended_authorization_last_applied_at: Option<PrimitiveDateTime>,

    /// Optional boolean value to extent authorization period of this payment
    ///
    /// capture method must be manual or manual_multiple
    #[schema(value_type = Option<bool>, default = false)]
    #[smithy(value_type = "Option<bool>")]
    pub request_extended_authorization: Option<RequestExtendedAuthorizationBool>,

    /// date and time after which this payment cannot be captured
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub capture_before: Option<PrimitiveDateTime>,

    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    #[schema(
        value_type = Option<String>,
        max_length = 255,
        example = "Custom_Order_id_123"
    )]
    #[smithy(value_type = "Option<String>")]
    pub merchant_order_reference_id: Option<String>,
    /// order tax amount calculated by tax connectors
    #[smithy(value_type = "Option<i64>")]
    pub order_tax_amount: Option<MinorUnit>,

    /// Connector Identifier for the payment method
    #[smithy(value_type = "Option<String>")]
    pub connector_mandate_id: Option<String>,

    /// Method through which card was discovered
    #[schema(value_type = Option<CardDiscovery>, example = "manual")]
    #[smithy(value_type = "Option<CardDiscovery>")]
    pub card_discovery: Option<enums::CardDiscovery>,

    /// Indicates if 3ds challenge is forced
    #[smithy(value_type = "Option<bool>")]
    pub force_3ds_challenge: Option<bool>,

    /// Indicates if 3ds challenge is triggered
    #[smithy(value_type = "Option<bool>")]
    pub force_3ds_challenge_trigger: Option<bool>,

    /// Error code received from the issuer in case of failed payments
    #[smithy(value_type = "Option<String>")]
    pub issuer_error_code: Option<String>,

    /// Error message received from the issuer in case of failed payments
    #[smithy(value_type = "Option<String>")]
    pub issuer_error_message: Option<String>,

    /// Indicates if the redirection has to open in the iframe
    #[smithy(value_type = "Option<bool>")]
    pub is_iframe_redirection_enabled: Option<bool>,

    /// Contains whole connector response
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub whole_connector_response: Option<Secret<String>>,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    #[smithy(value_type = "Option<bool>")]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,

    /// Bool indicating if overcapture  must be requested for this payment
    #[schema(value_type = Option<bool>)]
    #[smithy(value_type = "Option<bool>")]
    pub enable_overcapture: Option<primitive_wrappers::EnableOvercaptureBool>,

    /// Boolean indicating whether overcapture is effectively enabled for this payment
    #[schema(value_type = Option<bool>)]
    #[smithy(value_type = "Option<bool>")]
    pub is_overcapture_enabled: Option<primitive_wrappers::OvercaptureEnabledBool>,

    /// Contains card network response details (e.g., Visa/Mastercard advice codes).
    #[schema(value_type = Option<NetworkDetails>)]
    #[smithy(value_type = "Option<NetworkDetails>")]
    pub network_details: Option<NetworkDetails>,

    /// Boolean flag indicating whether this payment method is stored and has been previously used for payments
    #[schema(value_type = Option<bool>, example = true)]
    #[smithy(value_type = "Option<bool>")]
    pub is_stored_credential: Option<bool>,

    /// The category of the MIT transaction
    #[schema(value_type = Option<MitCategory>, example = "recurring")]
    #[smithy(value_type = "Option<MitCategory>")]
    pub mit_category: Option<api_enums::MitCategory>,

    /// Billing descriptor information for the payment
    #[schema(value_type = Option<BillingDescriptor>)]
    pub billing_descriptor: Option<common_types::payments::BillingDescriptor>,

    /// The tokenization preference for the payment method. This is used to control whether a PSP token is created or not.
    #[schema(value_type = Option<Tokenization>,example="skip_psp")]
    pub tokenization: Option<enums::Tokenization>,

    /// Information identifying partner and merchant details
    #[schema(value_type = Option<PartnerMerchantIdentifierDetails>)]
    pub partner_merchant_identifier_details:
        Option<common_types::payments::PartnerMerchantIdentifierDetails>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentsListResponseItem {
    /// Unique identifier for the payment
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_pay_01926c58bc6e77c09e809964e72af8c8",
        value_type = String,
    )]
    pub id: id_type::GlobalPaymentId,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The business profile that is associated with this payment
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = Option<String>
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Identifier for Payment Method used for the payment
    #[schema(value_type = Option<String>)]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    /// Status of the payment
    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    pub status: api_enums::IntentStatus,

    /// Amount related information for this payment and attempt
    pub amount: PaymentAmountDetailsResponse,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The payment method type for this payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: Option<api_enums::PaymentMethod>,

    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,

    /// The connector used for the payment
    #[schema(value_type = Option<Connector>, example = "stripe")]
    pub connector: Option<String>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Details of the customer
    pub customer: Option<CustomerDetailsResponse>,

    /// The reference id for the order in the merchant's system. This value can be passed by the merchant.
    #[schema(value_type = Option<String>)]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_payment_id: Option<String>,

    /// Reference to the capture at connector side
    pub connector_response_reference_id: Option<String>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<Secret<serde_json::Value>>,

    /// A description of the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Total number of attempts associated with this payment
    pub attempt_count: i16,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// If the payment was cancelled the reason will be provided here
    pub cancellation_reason: Option<String>,

    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "gillete creme",
        "quantity": 15,
        "amount" : 900
    }]"#)]
    pub order_details: Option<Vec<Secret<OrderDetailsWithAmount>>>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(value_type = Option<String>, max_length = 255, example = "Hyperswitch Router")]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Allowed Payment Method Types for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<common_enums::PaymentMethodType>>,

    /// Total number of authorizations happened in an incremental_authorization payment
    pub authorization_count: Option<i32>,

    /// Date time at which payment was updated
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,

    /// Indicates if the payment amount is split across multiple payment methods
    pub is_split_payment: bool,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct RecoveryPaymentsListResponseItem {
    /// Unique identifier for the payment
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_pay_01926c58bc6e77c09e809964e72af8c8",
        value_type = String,
    )]
    pub id: id_type::GlobalPaymentId,

    /// This is an identifier for the merchant account
    #[schema(max_length = 255, example = "merchant_1668273825", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The business profile that is associated with this payment
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = Option<String>
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Status of the payment
    #[schema(value_type = RecoveryStatus, example = "failed", default = "requires_confirmation")]
    pub status: api_enums::RecoveryStatus,

    /// Amount related information for this payment and attempt
    pub amount: PaymentAmountDetailsResponse,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The payment method type for this payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: Option<api_enums::PaymentMethod>,

    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,

    /// The connector used for the payment
    #[schema(value_type = Option<Connector>, example = "stripe")]
    pub connector: Option<String>,

    /// Identifier of the connector which was chosen to make the payment
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Details of the customer
    pub customer: Option<CustomerDetailsResponse>,

    /// The reference id for the order in the merchant's system. This value can be passed by the merchant.
    #[schema(value_type = Option<String>)]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// A description of the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// Total number of attempts associated with this payment
    pub attempt_count: i16,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// If the payment was cancelled the reason will be provided here
    pub cancellation_reason: Option<String>,

    /// Date time at which payment was updated
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,

    /// Date time at which payment last attempt was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_attempt_at: Option<PrimitiveDateTime>,
}

// Serialize is implemented because, this will be serialized in the api events.
// Usually request types should not have serialize implemented.
//
/// Request for Payment Intent Confirm
#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentsConfirmIntentRequest {
    /// The URL to which you want the user to be redirected after the completion of the payment operation
    /// If this url is not passed, the url configured in the business profile will be used
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    /// The payment instrument data to be used for the payment
    pub payment_method_data: PaymentMethodDataRequest,

    /// The payment instrument data to be used for the payment in case of split payments
    pub split_payment_method_data: Option<Vec<SplitPaymentMethodDataRequest>>,

    /// The payment method type to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method_type: api_enums::PaymentMethod,

    /// The payment method subtype to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// The shipping address for the payment. This will override the shipping address provided in the create-intent request
    pub shipping: Option<Address>,

    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// The payment_method_id to be associated with the payment
    #[schema(value_type = Option<String>)]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,

    /// The webhook endpoint URL to receive payment status notifications
    #[schema(value_type = Option<String>, example = "https://merchant.example.com/webhooks/payment")]
    pub webhook_url: Option<common_utils::types::Url>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProxyPaymentsRequest {
    /// The URL to which you want the user to be redirected after the completion of the payment operation
    /// If this url is not passed, the url configured in the business profile will be used
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    pub amount: AmountDetails,

    pub recurring_details: mandates::ProcessorPaymentToken,

    pub shipping: Option<Address>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    #[schema(example = "stripe")]
    pub connector: String,

    #[schema(value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct ExternalVaultProxyPaymentsRequest {
    /// The URL to which you want the user to be redirected after the completion of the payment operation
    /// If this url is not passed, the url configured in the business profile will be used
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    /// The payment instrument data to be used for the payment
    pub payment_method_data: ProxyPaymentMethodDataRequest,

    /// The payment method type to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method_type: api_enums::PaymentMethod,

    /// The payment method subtype to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// The shipping address for the payment. This will override the shipping address provided in the create-intent request
    pub shipping: Option<Address>,

    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// The payment_method_id to be associated with the payment
    #[schema(value_type = Option<String>)]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

// This struct contains the union of fields in `PaymentsCreateIntentRequest` and
// `PaymentsConfirmIntentRequest`
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct PaymentsRequest {
    /// The amount details for the payment
    pub amount_details: AmountDetails,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// The routing algorithm id to be used for the payment
    #[schema(value_type = Option<String>)]
    pub routing_algorithm_id: Option<id_type::RoutingId>,

    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    pub billing: Option<Address>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
    #[schema(example = "present", value_type = Option<PresenceOfCustomerDuringPayment>)]
    pub customer_present: Option<common_enums::PresenceOfCustomerDuringPayment>,

    /// A description for the payment
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Apply MIT exemption for a payment
    #[schema(value_type = Option<MitExemptionRequest>)]
    pub apply_mit_exemption: Option<common_enums::MitExemptionRequest>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 22, example = "Hyperswitch Router", value_type = Option<String>)]
    pub statement_descriptor: Option<common_utils::types::StatementDescriptor>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(value_type = Option<EnablePaymentLinkRequest>)]
    pub payment_link_enabled: Option<common_enums::EnablePaymentLinkRequest>,

    /// Configure a custom payment link for the particular payment
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    pub payment_link_config: Option<admin::PaymentLinkConfigRequest>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    #[schema(value_type = Option<RequestIncrementalAuthorization>)]
    pub request_incremental_authorization: Option<common_enums::RequestIncrementalAuthorization>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds, if not sent it will be taken from profile config
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = Option<External3dsAuthenticationRequest>)]
    pub request_external_three_ds_authentication:
        Option<common_enums::External3dsAuthenticationRequest>,

    /// The payment instrument data to be used for the payment
    pub payment_method_data: PaymentMethodDataRequest,

    /// The payment method type to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method_type: api_enums::PaymentMethod,

    /// The payment method subtype to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// The payment_method_id to be associated with the payment
    #[schema(value_type = Option<String>)]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    /// Indicates if 3ds challenge is forced
    pub force_3ds_challenge: Option<bool>,

    /// Indicates if the redirection has to open in the iframe
    pub is_iframe_redirection_enabled: Option<bool>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// Stringified connector raw response body. Only returned if `return_raw_connector_response` is true
    pub return_raw_connector_response: Option<bool>,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,

    /// The webhook endpoint URL to receive payment status notifications
    #[schema(value_type = Option<String>, example = "https://merchant.example.com/webhooks/payment")]
    pub webhook_url: Option<common_utils::types::Url>,
}

#[cfg(feature = "v2")]
impl From<&PaymentsRequest> for PaymentsCreateIntentRequest {
    fn from(request: &PaymentsRequest) -> Self {
        Self {
            amount_details: request.amount_details.clone(),
            merchant_reference_id: request.merchant_reference_id.clone(),
            routing_algorithm_id: request.routing_algorithm_id.clone(),
            capture_method: request.capture_method,
            authentication_type: request.authentication_type,
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
            customer_id: request.customer_id.clone(),
            customer_present: request.customer_present,
            description: request.description.clone(),
            return_url: request.return_url.clone(),
            setup_future_usage: request.setup_future_usage,
            apply_mit_exemption: request.apply_mit_exemption,
            statement_descriptor: request.statement_descriptor.clone(),
            order_details: request.order_details.clone(),
            allowed_payment_method_types: request.allowed_payment_method_types.clone(),
            metadata: request.metadata.clone(),
            connector_metadata: request.connector_metadata.clone(),
            feature_metadata: request.feature_metadata.clone(),
            payment_link_enabled: request.payment_link_enabled,
            payment_link_config: request.payment_link_config.clone(),
            request_incremental_authorization: request.request_incremental_authorization,
            session_expiry: request.session_expiry,
            frm_metadata: request.frm_metadata.clone(),
            request_external_three_ds_authentication: request
                .request_external_three_ds_authentication,
            force_3ds_challenge: request.force_3ds_challenge,
            merchant_connector_details: request.merchant_connector_details.clone(),
            enable_partial_authorization: request.enable_partial_authorization,
        }
    }
}

#[cfg(feature = "v2")]
impl From<&PaymentsRequest> for PaymentsConfirmIntentRequest {
    fn from(request: &PaymentsRequest) -> Self {
        Self {
            return_url: request.return_url.clone(),
            payment_method_data: request.payment_method_data.clone(),
            payment_method_type: request.payment_method_type,
            payment_method_subtype: request.payment_method_subtype,
            shipping: request.shipping.clone(),
            customer_acceptance: request.customer_acceptance.clone(),
            browser_info: request.browser_info.clone(),
            payment_method_id: request.payment_method_id.clone(),
            payment_token: None,
            merchant_connector_details: request.merchant_connector_details.clone(),
            return_raw_connector_response: request.return_raw_connector_response,
            split_payment_method_data: None,
            webhook_url: request.webhook_url.clone(),
        }
    }
}

// Serialize is implemented because, this will be serialized in the api events.
// Usually request types should not have serialize implemented.
//
/// Request body for Payment Status
#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentsRetrieveRequest {
    /// A boolean used to indicate if the payment status should be fetched from the connector
    /// If this is set to true, the status will be fetched from the connector
    #[serde(default)]
    pub force_sync: bool,
    /// A boolean used to indicate if all the attempts needs to be fetched for the intent.
    /// If this is set to true, attempts list will be available in the response.
    #[serde(default)]
    pub expand_attempts: bool,
    /// These are the query params that are sent in case of redirect response.
    /// These can be ingested by the connector to take necessary actions.
    pub param: Option<String>,
    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
/// Request for Payment Status
pub struct PaymentsStatusRequest {
    /// A boolean used to indicate if the payment status should be fetched from the connector
    /// If this is set to true, the status will be fetched from the connector
    #[serde(default)]
    pub force_sync: bool,
    /// A boolean used to indicate if all the attempts needs to be fetched for the intent.
    /// If this is set to true, attempts list will be available in the response.
    #[serde(default)]
    pub expand_attempts: bool,
    /// These are the query params that are sent in case of redirect response.
    /// These can be ingested by the connector to take necessary actions.
    pub param: Option<String>,
    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

/// Error details for the payment
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, ToSchema)]
pub struct ErrorDetails {
    /// The error code
    pub code: String,
    /// The error message
    pub message: String,
    /// The detailed error reason that was returned by the connector.
    pub reason: Option<String>,
    /// The unified error code across all connectors.
    /// This can be relied upon for taking decisions based on the error.
    pub unified_code: Option<String>,
    /// The unified error message across all connectors.
    /// If there is a translation available, this will have the translated message
    pub unified_message: Option<String>,
    /// This field can be returned for both approved and refused Mastercard payments.
    /// This code provides additional information about the type of transaction or the reason why the payment failed.
    /// If the payment failed, the network advice code gives guidance on if and when you can retry the payment.
    pub network_advice_code: Option<String>,
    /// For card errors resulting from a card issuer decline, a brand specific 2, 3, or 4 digit code which indicates the reason the authorization failed.
    pub network_decline_code: Option<String>,
    /// A string indicating how to proceed with an network error if payment gateway provide one. This is used to understand the network error code better.
    pub network_error_message: Option<String>,
}

/// Token information that can be used to initiate transactions by the merchant.
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConnectorTokenDetails {
    /// A token that can be used to make payments directly with the connector.
    #[schema(example = "pm_9UhMqBMEOooRIvJFFdeW")]
    pub token: String,

    /// The reference id sent to the connector when creating the token
    pub connector_token_request_reference_id: Option<String>,
}

/// Response for Payment Intent Confirm
/// Few fields should be expandable, we need not return these in the normal response
/// But when explicitly requested for expanded objects, these can be returned
/// For example
/// shipping, billing, customer, payment_method
#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct PaymentsResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_pay_01926c58bc6e77c09e809964e72af8c8",
        value_type = String,
    )]
    pub id: id_type::GlobalPaymentId,

    #[schema(value_type = IntentStatus, example = "succeeded")]
    pub status: api_enums::IntentStatus,

    /// Amount related information for this payment and attempt
    pub amount: PaymentAmountDetailsResponse,

    /// The identifier for the customer
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// The connector used for the payment
    #[schema(example = "stripe")]
    pub connector: Option<String>,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// Time when the payment was last modified
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethodDataResponseWithBilling>)]
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,

    /// The payment method type for this payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: Option<api_enums::PaymentMethod>,

    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<String>,

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_reference_id: Option<String>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// The browser information used for this payment
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// The shipping address associated with the payment intent
    pub shipping: Option<Address>,

    /// The billing address associated with the payment intent
    pub billing: Option<Address>,

    /// List of payment attempts associated with payment intent
    pub attempts: Option<Vec<PaymentAttemptResponse>>,

    /// Connector token information that can be used to make payments directly by the merchant.
    pub connector_token_details: Option<ConnectorTokenDetails>,

    /// The payment_method_id associated with the payment
    #[schema(value_type = Option<String>)]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,

    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<common_utils::types::Url>,

    /// The authentication type that was requested for this order
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The authentication type that was appliced for this order
    /// This depeneds on the 3DS rules configured, If not a default authentication type will be applied
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "no_three_ds")]
    pub authentication_type_applied: Option<api_enums::AuthenticationType>,

    /// Indicates if the redirection has to open in the iframe
    pub is_iframe_redirection_enabled: Option<bool>,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,

    /// Stringified connector raw response body. Only returned if `return_raw_connector_response` is true
    #[schema(value_type = Option<String>)]
    pub raw_connector_response: Option<Secret<String>>,

    /// Additional data that might be required by hyperswitch based on the additional features.
    pub feature_metadata: Option<FeatureMetadata>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v2")]
impl PaymentAttemptListResponse {
    pub fn find_attempt_in_attempts_list_using_connector_transaction_id(
        &self,
        connector_transaction_id: &common_utils::types::ConnectorTransactionId,
    ) -> Option<PaymentAttemptResponse> {
        self.payment_attempt_list.iter().find_map(|attempt| {
            attempt
                .connector_payment_id
                .as_ref()
                .filter(|txn_id| *txn_id == connector_transaction_id)
                .map(|_| attempt.clone())
        })
    }
    pub fn find_attempt_in_attempts_list_using_charge_id(
        &self,
        charge_id: String,
    ) -> Option<PaymentAttemptResponse> {
        self.payment_attempt_list.iter().find_map(|attempt| {
            attempt.feature_metadata.as_ref().and_then(|metadata| {
                metadata.revenue_recovery.as_ref().and_then(|recovery| {
                    recovery
                        .charge_id
                        .as_ref()
                        .filter(|id| **id == charge_id)
                        .map(|_| attempt.clone())
                })
            })
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[cfg(feature = "v2")]
pub struct PaymentStartRedirectionRequest {
    /// Global Payment ID
    pub id: id_type::GlobalPaymentId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[cfg(feature = "v2")]
pub struct PaymentStartRedirectionParams {
    /// The identifier for the Merchant Account.
    pub publishable_key: String,
    /// The identifier for business profile
    pub profile_id: id_type::ProfileId,
}

/// Details of external authentication
#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ExternalAuthenticationDetailsResponse {
    /// Authentication Type - Challenge / Frictionless
    #[schema(value_type = Option<DecoupledAuthenticationType>)]
    #[smithy(value_type = "Option<DecoupledAuthenticationType>")]
    pub authentication_flow: Option<enums::DecoupledAuthenticationType>,
    /// Electronic Commerce Indicator (eci)
    #[smithy(value_type = "Option<String>")]
    pub electronic_commerce_indicator: Option<String>,
    /// Authentication Status
    #[schema(value_type = AuthenticationStatus)]
    #[smithy(value_type = "AuthenticationStatus")]
    pub status: enums::AuthenticationStatus,
    /// DS Transaction ID
    #[smithy(value_type = "Option<String>")]
    pub ds_transaction_id: Option<String>,
    /// Message Version
    #[smithy(value_type = "Option<String>")]
    pub version: Option<String>,
    /// Error Code
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// Error Message
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Deserialize, ToSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentListConstraints {
    /// The identifier for customer
    #[schema(
        max_length = 64,
        min_length = 1,
        example = "cus_y3oqhf46pyzuxjbcn2giaqnb44",
        value_type = Option<String>,
    )]
    pub customer_id: Option<id_type::CustomerId>,

    /// A cursor for use in pagination, fetch the next list after some object
    #[schema(example = "pay_fafa124123", value_type = Option<String>)]
    pub starting_after: Option<id_type::PaymentId>,

    /// A cursor for use in pagination, fetch the previous list before some object
    #[schema(example = "pay_fafa124123", value_type = Option<String>)]
    pub ending_before: Option<id_type::PaymentId>,

    /// limit on the number of objects to return
    #[schema(default = 10, maximum = 100)]
    #[serde(default = "default_payments_list_limit")]
    pub limit: u32,

    /// The time at which payment is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// Time less than the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lt"
    )]
    pub created_lt: Option<PrimitiveDateTime>,

    /// Time greater than the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.gt"
    )]
    pub created_gt: Option<PrimitiveDateTime>,

    /// Time less than or equals to the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lte"
    )]
    pub created_lte: Option<PrimitiveDateTime>,

    /// Time greater than or equals to the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gte")]
    pub created_gte: Option<PrimitiveDateTime>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct PaymentListConstraints {
    /// The identifier for payment
    #[param(example = "pay_fafa124123", value_type = Option<String>)]
    pub payment_id: Option<id_type::GlobalPaymentId>,

    /// The identifier for business profile
    #[param(example = "pay_fafa124123", value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// The identifier for customer
    #[param(
        max_length = 64,
        min_length = 1,
        example = "cus_y3oqhf46pyzuxjbcn2giaqnb44",
        value_type = Option<String>,
    )]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    /// A cursor for use in pagination, fetch the next list after some object
    #[param(example = "pay_fafa124123", value_type = Option<String>)]
    pub starting_after: Option<id_type::GlobalPaymentId>,

    /// A cursor for use in pagination, fetch the previous list before some object
    #[param(example = "pay_fafa124123", value_type = Option<String>)]
    pub ending_before: Option<id_type::GlobalPaymentId>,

    /// limit on the number of objects to return
    #[param(default = 10, maximum = 100)]
    #[serde(default = "default_payments_list_limit")]
    pub limit: u32,

    /// The starting point within a list of objects
    pub offset: Option<u32>,

    /// The time at which payment is created
    #[param(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// Time less than the payment created time
    #[param(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lt"
    )]
    pub created_lt: Option<PrimitiveDateTime>,

    /// Time greater than the payment created time
    #[param(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.gt"
    )]
    pub created_gt: Option<PrimitiveDateTime>,

    /// Time less than or equals to the payment created time
    #[param(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lte"
    )]
    pub created_lte: Option<PrimitiveDateTime>,

    /// Time greater than or equals to the payment created time
    #[param(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gte")]
    pub created_gte: Option<PrimitiveDateTime>,

    /// The start amount to filter list of transactions which are greater than or equal to the start amount
    pub start_amount: Option<i64>,
    /// The end amount to filter list of transactions which are less than or equal to the end amount
    pub end_amount: Option<i64>,
    /// The connector to filter payments list
    #[param(value_type = Option<Vec<Connector>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub connector: Option<Vec<api_enums::Connector>>,
    /// The currency to filter payments list
    #[param(value_type = Option<Vec<Currency>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub currency: Option<Vec<enums::Currency>>,
    /// The payment status to filter payments list
    #[param(value_type = Option<Vec<IntentStatus>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub status: Option<Vec<enums::IntentStatus>>,
    /// The payment method type to filter payments list
    #[param(value_type = Option<Vec<PaymentMethod>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub payment_method_type: Option<Vec<enums::PaymentMethod>>,
    /// The payment method subtype to filter payments list
    #[param(value_type = Option<Vec<PaymentMethodType>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub payment_method_subtype: Option<Vec<enums::PaymentMethodType>>,
    /// The authentication type to filter payments list
    #[param(value_type = Option<Vec<AuthenticationType>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub authentication_type: Option<Vec<enums::AuthenticationType>>,
    /// The merchant connector id to filter payments list
    #[param(value_type = Option<Vec<String>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
    /// The field on which the payments list should be sorted
    #[serde(default)]
    pub order_on: SortOn,
    /// The order in which payments list should be sorted
    #[serde(default)]
    pub order_by: SortBy,
    /// The card networks to filter payments list
    #[param(value_type = Option<Vec<CardNetwork>>)]
    #[serde(deserialize_with = "parse_comma_separated", default)]
    pub card_network: Option<Vec<enums::CardNetwork>>,
    /// The identifier for merchant order reference id
    pub merchant_order_reference_id: Option<String>,
}

#[cfg(feature = "v2")]
impl PaymentListConstraints {
    pub fn has_no_attempt_filters(&self) -> bool {
        self.connector.is_none()
            && self.payment_method_type.is_none()
            && self.payment_method_subtype.is_none()
            && self.authentication_type.is_none()
            && self.merchant_connector_id.is_none()
            && self.card_network.is_none()
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentListResponse {
    /// The number of payments included in the list
    pub size: usize,
    // The list of payments response objects
    pub data: Vec<PaymentsResponse>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentListResponse {
    /// The number of payments included in the current response
    pub count: usize,
    /// The total number of available payments for given constraints
    pub total_count: i64,
    /// The list of payments response objects
    pub data: Vec<PaymentsListResponseItem>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct RecoveryPaymentListResponse {
    /// The number of payments included in the current response
    pub count: usize,
    /// The total number of available payments for given constraints
    pub total_count: i64,
    /// The list of payments response objects
    pub data: Vec<RecoveryPaymentsListResponseItem>,
}

#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct IncrementalAuthorizationResponse {
    /// The unique identifier of authorization
    #[smithy(value_type = "String")]
    pub authorization_id: String,
    /// Amount the authorization has been made for
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    #[schema(value_type= AuthorizationStatus)]
    #[smithy(value_type = "AuthorizationStatus")]
    /// The status of the authorization
    pub status: common_enums::AuthorizationStatus,
    /// Error code sent by the connector for authorization
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// Error message sent by the connector for authorization
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
    /// Previously authorized amount for the payment
    #[smithy(value_type = "i64")]
    pub previously_authorized_amount: MinorUnit,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListResponseV2 {
    /// The number of payments included in the list for given constraints
    pub count: usize,
    /// The total number of available payments for given constraints
    pub total_count: i64,
    /// The list of payments response objects
    pub data: Vec<PaymentsResponse>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentListFilterConstraints {
    /// The identifier for payment
    pub payment_id: Option<id_type::PaymentId>,
    /// The identifier for business profile
    pub profile_id: Option<id_type::ProfileId>,
    /// The identifier for customer
    pub customer_id: Option<id_type::CustomerId>,
    /// The limit on the number of objects. The default limit is 10 and max limit is 20
    #[serde(default = "default_payments_list_limit")]
    pub limit: u32,
    /// The starting point within a list of objects
    pub offset: Option<u32>,
    /// The amount to filter payments list
    pub amount_filter: Option<AmountFilter>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).
    #[serde(flatten)]
    pub time_range: Option<common_utils::types::TimeRange>,
    /// The list of connectors to filter payments list
    pub connector: Option<Vec<api_enums::Connector>>,
    /// The list of currencies to filter payments list
    pub currency: Option<Vec<enums::Currency>>,
    /// The list of payment status to filter payments list
    pub status: Option<Vec<enums::IntentStatus>>,
    /// The list of payment methods to filter payments list
    pub payment_method: Option<Vec<enums::PaymentMethod>>,
    /// The list of payment method types to filter payments list
    pub payment_method_type: Option<Vec<enums::PaymentMethodType>>,
    /// The list of authentication types to filter payments list
    pub authentication_type: Option<Vec<enums::AuthenticationType>>,
    /// The list of merchant connector ids to filter payments list for selected label
    pub merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
    /// The order in which payments list should be sorted
    #[serde(default)]
    pub order: Order,
    /// The List of all the card networks to filter payments list
    pub card_network: Option<Vec<enums::CardNetwork>>,
    /// The identifier for merchant order reference id
    pub merchant_order_reference_id: Option<String>,
    /// Indicates the method by which a card is discovered during a payment
    pub card_discovery: Option<Vec<enums::CardDiscovery>>,
}

#[cfg(feature = "v1")]
impl PaymentListFilterConstraints {
    pub fn has_no_attempt_filters(&self) -> bool {
        self.connector.is_none()
            && self.payment_method.is_none()
            && self.payment_method_type.is_none()
            && self.authentication_type.is_none()
            && self.merchant_connector_id.is_none()
            && self.card_network.is_none()
            && self.card_discovery.is_none()
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListFilters {
    /// The list of available connector filters
    pub connector: Vec<String>,
    /// The list of available currency filters
    pub currency: Vec<enums::Currency>,
    /// The list of available payment status filters
    pub status: Vec<enums::IntentStatus>,
    /// The list of available payment method filters
    pub payment_method: Vec<enums::PaymentMethod>,
    /// The list of available payment method types
    pub payment_method_type: Vec<enums::PaymentMethodType>,
    /// The list of available authentication types
    pub authentication_type: Vec<enums::AuthenticationType>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListFiltersV2 {
    /// The list of available connector filters
    pub connector: HashMap<String, Vec<MerchantConnectorInfo>>,
    /// The list of available currency filters
    pub currency: Vec<enums::Currency>,
    /// The list of available payment status filters
    pub status: Vec<enums::IntentStatus>,
    /// The list payment method and their corresponding types
    pub payment_method: HashMap<enums::PaymentMethod, HashSet<enums::PaymentMethodType>>,
    /// The list of available authentication types
    pub authentication_type: Vec<enums::AuthenticationType>,
    /// The list of available card networks
    pub card_network: Vec<enums::CardNetwork>,
    /// The list of available Card discovery methods
    pub card_discovery: Vec<enums::CardDiscovery>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentsAggregateResponse {
    /// The list of intent status with their count
    pub status_with_count: HashMap<enums::IntentStatus, i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AmountFilter {
    /// The start amount to filter list of transactions which are greater than or equal to the start amount
    pub start_amount: Option<i64>,
    /// The end amount to filter list of transactions which are less than or equal to the end amount
    pub end_amount: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Order {
    /// The field to sort, such as Amount or Created etc.
    pub on: SortOn,
    /// The order in which to sort the items, either Ascending or Descending
    pub by: SortBy,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortOn {
    /// Sort by the amount field
    Amount,
    /// Sort by the created_at field
    #[default]
    Created,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    /// Sort in ascending order
    Asc,
    /// Sort in descending order
    #[default]
    Desc,
}

#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize)]
pub struct VerifyResponse {
    pub verify_id: Option<id_type::PaymentId>,
    pub merchant_id: Option<id_type::MerchantId>,
    // pub status: enums::VerifyStatus,
    pub client_secret: Option<Secret<String>>,
    pub customer_id: Option<id_type::CustomerId>,
    pub email: crypto::OptionalEncryptableEmail,
    pub name: crypto::OptionalEncryptableName,
    pub phone: crypto::OptionalEncryptablePhone,
    pub mandate_id: Option<String>,
    #[auth_based]
    pub payment_method: Option<api_enums::PaymentMethod>,
    #[auth_based]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentsRedirectionResponse {
    pub redirect_url: String,
}

pub struct MandateValidationFields {
    pub recurring_details: Option<RecurringDetails>,
    pub confirm: Option<bool>,
    pub customer_id: Option<id_type::CustomerId>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
}

#[cfg(feature = "v1")]
impl From<&PaymentsRequest> for MandateValidationFields {
    fn from(req: &PaymentsRequest) -> Self {
        let recurring_details = req
            .mandate_id
            .clone()
            .map(RecurringDetails::MandateId)
            .or(req.recurring_details.clone());

        Self {
            recurring_details,
            confirm: req.confirm,
            customer_id: req
                .customer
                .as_ref()
                .map(|customer_details| &customer_details.id)
                .or(req.customer_id.as_ref())
                .map(ToOwned::to_owned),
            mandate_data: req.mandate_data.clone(),
            setup_future_usage: req.setup_future_usage,
            off_session: req.off_session,
        }
    }
}

impl From<&VerifyRequest> for MandateValidationFields {
    fn from(req: &VerifyRequest) -> Self {
        Self {
            recurring_details: None,
            confirm: Some(true),
            customer_id: req.customer_id.clone(),
            mandate_data: req.mandate_data.clone(),
            off_session: req.off_session,
            setup_future_usage: req.setup_future_usage,
        }
    }
}

// #[cfg(all(feature = "v2", feature = "payment_v2"))]
// impl From<PaymentsSessionRequest> for PaymentsSessionResponse {
//     fn from(item: PaymentsSessionRequest) -> Self {
//         let client_secret: Secret<String, pii::ClientSecret> = Secret::new(item.client_secret);
//         Self {
//             session_token: vec![],
//             payment_id: item.payment_id,
//             client_secret,
//         }
//     }
// }

#[cfg(feature = "v1")]
impl From<PaymentsSessionRequest> for PaymentsSessionResponse {
    fn from(item: PaymentsSessionRequest) -> Self {
        let client_secret: Secret<String, pii::ClientSecret> = Secret::new(item.client_secret);
        Self {
            session_token: vec![],
            payment_id: item.payment_id,
            client_secret,
        }
    }
}

#[cfg(feature = "v1")]
impl From<PaymentsStartRequest> for PaymentsRequest {
    fn from(item: PaymentsStartRequest) -> Self {
        Self {
            payment_id: Some(PaymentIdType::PaymentIntentId(item.payment_id)),
            merchant_id: Some(item.merchant_id),
            ..Default::default()
        }
    }
}

impl From<AdditionalCardInfo> for CardResponse {
    fn from(card: AdditionalCardInfo) -> Self {
        Self {
            last4: card.last4,
            card_type: card.card_type,
            card_network: card.card_network,
            card_issuer: card.card_issuer,
            card_issuing_country: card.card_issuing_country,
            card_isin: card.card_isin,
            card_extended_bin: card.card_extended_bin,
            card_exp_month: card.card_exp_month,
            card_exp_year: card.card_exp_year,
            card_holder_name: card.card_holder_name,
            payment_checks: card.payment_checks,
            authentication_data: card.authentication_data,
        }
    }
}

impl From<KlarnaSdkPaymentMethod> for PaylaterResponse {
    fn from(klarna_sdk: KlarnaSdkPaymentMethod) -> Self {
        Self {
            klarna_sdk: Some(KlarnaSdkPaymentMethodResponse {
                payment_type: klarna_sdk.payment_type,
            }),
        }
    }
}

impl From<AdditionalPaymentData> for PaymentMethodDataResponse {
    fn from(payment_method_data: AdditionalPaymentData) -> Self {
        match payment_method_data {
            AdditionalPaymentData::Card(card) => Self::Card(Box::new(CardResponse::from(*card))),
            AdditionalPaymentData::PayLater { klarna_sdk } => match klarna_sdk {
                Some(sdk) => Self::PayLater(Box::new(PaylaterResponse::from(sdk))),
                None => Self::PayLater(Box::new(PaylaterResponse { klarna_sdk: None })),
            },
            AdditionalPaymentData::Wallet {
                apple_pay,
                google_pay,
                samsung_pay,
            } => match (apple_pay, google_pay, samsung_pay) {
                (Some(apple_pay_pm), _, _) => Self::Wallet(Box::new(WalletResponse {
                    details: Some(WalletResponseData::ApplePay(Box::new(
                        WalletAdditionalDataForCard {
                            last4: apple_pay_pm
                                .display_name
                                .clone()
                                .chars()
                                .rev()
                                .take(4)
                                .collect::<String>()
                                .chars()
                                .rev()
                                .collect::<String>(),
                            card_network: apple_pay_pm.network.clone(),
                            card_type: Some(apple_pay_pm.pm_type.clone()),
                            card_exp_month: apple_pay_pm.card_exp_month,
                            card_exp_year: apple_pay_pm.card_exp_year,
                        },
                    ))),
                })),
                (_, Some(google_pay_pm), _) => Self::Wallet(Box::new(WalletResponse {
                    details: Some(WalletResponseData::GooglePay(google_pay_pm)),
                })),
                (_, _, Some(samsung_pay_pm)) => Self::Wallet(Box::new(WalletResponse {
                    details: Some(WalletResponseData::SamsungPay(samsung_pay_pm)),
                })),
                _ => Self::Wallet(Box::new(WalletResponse { details: None })),
            },
            AdditionalPaymentData::BankRedirect {
                bank_name,
                details,
                interac,
            } => Self::BankRedirect(Box::new(BankRedirectResponse {
                bank_name,
                details,
                interac,
            })),
            AdditionalPaymentData::Crypto { details } => {
                Self::Crypto(Box::new(CryptoResponse { details }))
            }
            AdditionalPaymentData::BankDebit { details } => {
                Self::BankDebit(Box::new(BankDebitResponse { details }))
            }
            AdditionalPaymentData::MandatePayment {} => Self::MandatePayment {},
            AdditionalPaymentData::Reward {} => Self::Reward {},
            AdditionalPaymentData::RealTimePayment { details } => {
                Self::RealTimePayment(Box::new(RealTimePaymentDataResponse { details }))
            }
            AdditionalPaymentData::Upi { details } => Self::Upi(Box::new(UpiResponse { details })),
            AdditionalPaymentData::BankTransfer { details } => {
                Self::BankTransfer(Box::new(BankTransferResponse { details }))
            }
            AdditionalPaymentData::Voucher { details } => {
                Self::Voucher(Box::new(VoucherResponse { details }))
            }
            AdditionalPaymentData::GiftCard { details } => {
                Self::GiftCard(Box::new(GiftCardResponse { details }))
            }
            AdditionalPaymentData::CardRedirect { details } => {
                Self::CardRedirect(Box::new(CardRedirectResponse { details }))
            }
            AdditionalPaymentData::CardToken { details } => {
                Self::CardToken(Box::new(CardTokenResponse { details }))
            }
            AdditionalPaymentData::OpenBanking { details } => {
                Self::OpenBanking(Box::new(OpenBankingResponse { details }))
            }
            AdditionalPaymentData::MobilePayment { details } => {
                Self::MobilePayment(Box::new(MobilePaymentResponse { details }))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PgRedirectResponse {
    pub payment_id: id_type::PaymentId,
    pub status: api_enums::IntentStatus,
    pub gateway_id: String,
    pub customer_id: Option<id_type::CustomerId>,
    pub amount: Option<MinorUnit>,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct RedirectionResponse {
    pub return_url: String,
    pub params: Vec<(String, String)>,
    pub return_url_with_query_params: String,
    pub http_method: String,
    pub headers: Vec<(String, String)>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct RedirectionResponse {
    pub return_url_with_query_params: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct PaymentsResponseForm {
    pub transaction_id: String,
    // pub transaction_reference_id: String,
    pub merchant_id: id_type::MerchantId,
    pub order_id: String,
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsRetrieveRequest {
    /// The type of ID (ex: payment intent id, payment attempt id or connector txn id)
    #[schema(value_type = String)]
    pub resource_id: PaymentIdType,
    /// The identifier for the Merchant Account.
    #[schema(value_type = Option<String>)]
    pub merchant_id: Option<id_type::MerchantId>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: bool,
    /// Optional query parameters that might be specific to a connector or flow, passed through during the retrieve operation. Use with caution and refer to specific connector documentation if applicable.
    pub param: Option<String>,
    /// Optionally specifies the connector to be used for a 'force_sync' retrieve operation. If provided, Hyperswitch will attempt to sync the payment status from this specific connector.
    pub connector: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<String>,
    /// If enabled provides list of captures linked to latest attempt
    pub expand_captures: Option<bool>,
    /// If enabled provides list of attempts linked to payment intent
    pub expand_attempts: Option<bool>,
    /// If enabled, provides whole connector response
    pub all_keys_required: Option<bool>,
}

#[derive(
    Debug, Default, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    #[schema(max_length = 255, example = "shirt")]
    #[smithy(value_type = "String")]
    pub product_name: String,
    /// The quantity of the product to be purchased
    #[schema(example = 1)]
    #[smithy(value_type = "u16")]
    pub quantity: u16,
    /// the amount per quantity of product
    #[schema(value_type = i64)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    /// tax rate applicable to the product
    #[smithy(value_type = "Option<f64>")]
    pub tax_rate: Option<f64>,
    /// total tax amount applicable to the product
    #[schema(value_type = Option<i64>)]
    #[smithy(value_type = "Option<i64>")]
    pub total_tax_amount: Option<MinorUnit>,
    // Does the order includes shipping
    #[smithy(value_type = "Option<bool>")]
    pub requires_shipping: Option<bool>,
    /// The image URL of the product
    #[smithy(value_type = "Option<String>")]
    pub product_img_link: Option<String>,
    /// ID of the product that is being purchased
    #[smithy(value_type = "Option<String>")]
    pub product_id: Option<String>,
    /// Category of the product that is being purchased
    #[smithy(value_type = "Option<String>")]
    pub category: Option<String>,
    /// Sub category of the product that is being purchased
    #[smithy(value_type = "Option<String>")]
    pub sub_category: Option<String>,
    /// Brand of the product that is being purchased
    #[smithy(value_type = "Option<String>")]
    pub brand: Option<String>,
    /// Type of the product that is being purchased
    pub product_type: Option<ProductType>,
    /// The tax code for the product
    #[smithy(value_type = "Option<String>")]
    pub product_tax_code: Option<String>,
    /// Description for the item
    #[smithy(value_type = "Option<String>")]
    pub description: Option<String>,
    /// Stock Keeping Unit (SKU) or the item identifier for this item.
    #[smithy(value_type = "Option<String>")]
    pub sku: Option<String>,
    /// Universal Product Code for the item.
    #[smithy(value_type = "Option<String>")]
    pub upc: Option<String>,
    /// Code describing a commodity or a group of commodities pertaining to goods classification.
    #[smithy(value_type = "Option<String>")]
    pub commodity_code: Option<String>,
    /// Unit of measure used for the item quantity.
    #[smithy(value_type = "Option<String>")]
    pub unit_of_measure: Option<String>,
    /// Total amount for the item.
    #[schema(value_type = Option<i64>)]
    #[smithy(value_type = "Option<i64>")]
    pub total_amount: Option<MinorUnit>, // total_amount,
    /// Discount amount applied to this item.
    #[schema(value_type = Option<i64>)]
    #[smithy(value_type = "Option<i64>")]
    pub unit_discount_amount: Option<MinorUnit>,
}

impl masking::SerializableSecret for OrderDetailsWithAmount {}

#[derive(
    Default,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    ToSchema,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RedirectResponse {
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub param: Option<Secret<String>>,
    #[schema(value_type = Option<Object>)]
    #[smithy(value_type = "Option<Object>")]
    pub json_payload: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionRequest {}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionRequest {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: String,
    /// The list of the supported wallets
    #[schema(value_type = Vec<PaymentMethodType>)]
    pub wallets: Vec<api_enums::PaymentMethodType>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentsUpdateMetadataRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Object, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: pii::SecretSerdeValue,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsUpdateMetadataResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsPostSessionTokensRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// It's a token used for client side verification.
    #[schema(value_type = String)]
    pub client_secret: Secret<String>,
    /// Payment method type
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: api_enums::PaymentMethodType,
    /// The payment method that is to be used for the payment
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method: api_enums::PaymentMethod,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsPostSessionTokensResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,
    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    pub status: api_enums::IntentStatus,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsDynamicTaxCalculationRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// The shipping address for the payment
    pub shipping: Address,
    /// Client Secret
    #[schema(value_type = String)]
    pub client_secret: Secret<String>,
    /// Payment method type
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: api_enums::PaymentMethodType,
    /// Session Id
    pub session_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsDynamicTaxCalculationResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// net amount = amount + order_tax_amount + shipping_cost
    pub net_amount: MinorUnit,
    /// order tax amount calculated by tax connectors
    pub order_tax_amount: Option<MinorUnit>,
    /// shipping cost for the order
    pub shipping_cost: Option<MinorUnit>,
    /// amount in Base Unit display format
    pub display_amount: DisplayAmountOnSdk,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct DisplayAmountOnSdk {
    /// net amount = amount + order_tax_amount + shipping_cost
    #[schema(value_type = String)]
    pub net_amount: StringMajorUnit,
    /// order tax amount calculated by tax connectors
    #[schema(value_type = String)]
    pub order_tax_amount: Option<StringMajorUnit>,
    /// shipping cost for the order
    #[schema(value_type = String)]
    pub shipping_cost: Option<StringMajorUnit>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayAllowedMethodsParameters {
    /// The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
    #[smithy(value_type = "Vec<String>")]
    pub allowed_auth_methods: Vec<String>,
    /// The list of allowed card networks (ex: AMEX,JCB etc)
    #[smithy(value_type = "Vec<String>")]
    pub allowed_card_networks: Vec<String>,
    /// Is billing address required
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<bool>")]
    pub billing_address_required: Option<bool>,
    /// Billing address parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<GpayBillingAddressParameters>")]
    pub billing_address_parameters: Option<GpayBillingAddressParameters>,
    /// Whether assurance details are required
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<bool>")]
    pub assurance_details_required: Option<bool>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayBillingAddressParameters {
    /// Is billing phone number required
    #[smithy(value_type = "bool")]
    pub phone_number_required: bool,
    /// Billing address format
    #[smithy(value_type = "GpayBillingAddressFormat")]
    pub format: GpayBillingAddressFormat,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum GpayBillingAddressFormat {
    FULL,
    MIN,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayTokenParameters {
    /// The name of the connector
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<String>")]
    pub gateway: Option<String>,
    /// The merchant ID registered in the connector associated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<String>")]
    pub gateway_merchant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stripe:version")]
    #[smithy(value_type = "Option<String>")]
    pub stripe_version: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "stripe:publishableKey"
    )]
    #[smithy(value_type = "Option<String>")]
    pub stripe_publishable_key: Option<String>,
    /// The protocol version for encryption
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<String>")]
    pub protocol_version: Option<String>,
    /// The public key provided by the merchant
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub public_key: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayTokenizationSpecification {
    /// The token specification type(ex: PAYMENT_GATEWAY)
    #[serde(rename = "type")]
    #[smithy(value_type = "String")]
    pub token_specification_type: String,
    /// The parameters for the token specification Google Pay
    #[smithy(value_type = "GpayTokenParameters")]
    pub parameters: GpayTokenParameters,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayAllowedPaymentMethods {
    /// The type of payment method
    #[serde(rename = "type")]
    #[smithy(value_type = "String")]
    pub payment_method_type: String,
    /// The parameters Google Pay requires
    #[smithy(value_type = "GpayAllowedMethodsParameters")]
    pub parameters: GpayAllowedMethodsParameters,
    /// The tokenization specification for Google Pay
    #[smithy(value_type = "GpayTokenizationSpecification")]
    pub tokenization_specification: GpayTokenizationSpecification,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayTransactionInfo {
    /// The country code
    #[schema(value_type = CountryAlpha2, example = "US")]
    #[smithy(value_type = "CountryAlpha2")]
    pub country_code: api_enums::CountryAlpha2,
    /// The currency code
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency_code: api_enums::Currency,
    /// The total price status (ex: 'FINAL')
    #[smithy(value_type = "String")]
    pub total_price_status: String,
    /// The total price
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub total_price: StringMajorUnit,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayMerchantInfo {
    /// The merchant Identifier that needs to be passed while invoking Gpay SDK
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<String>")]
    pub merchant_id: Option<String>,
    /// The name of the merchant that needs to be displayed on Gpay PopUp
    #[smithy(value_type = "String")]
    pub merchant_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayMetaData {
    pub merchant_info: GpayMerchantInfo,
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpaySessionTokenData {
    #[serde(rename = "google_pay")]
    pub data: GpayMetaData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PazeSessionTokenData {
    #[serde(rename = "paze")]
    pub data: PazeMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PazeMetadata {
    pub client_id: String,
    pub client_name: String,
    pub client_profile_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamsungPayCombinedMetadata {
    // This is to support the Samsung Pay decryption flow with application credentials,
    // where the private key, certificates, or any other information required for decryption
    // will be obtained from the application configuration.
    ApplicationCredentials(SamsungPayApplicationCredentials),
    MerchantCredentials(SamsungPayMerchantCredentials),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SamsungPaySessionTokenData {
    #[serde(rename = "samsung_pay")]
    pub data: SamsungPayCombinedMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SamsungPayMerchantCredentials {
    pub service_id: String,
    pub merchant_display_name: String,
    pub merchant_business_country: api_enums::CountryAlpha2,
    pub allowed_brands: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SamsungPayApplicationCredentials {
    pub merchant_display_name: String,
    pub merchant_business_country: api_enums::CountryAlpha2,
    pub allowed_brands: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaypalSdkMetaData {
    pub client_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaypalSdkSessionTokenData {
    #[serde(rename = "paypal_sdk")]
    pub data: PaypalSdkMetaData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepaySessionRequest {
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: String,
    pub initiative_context: String,
}

/// Some connectors like Apple Pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ConnectorMetadata {
    #[smithy(value_type = "Option<ApplepayConnectorMetadataRequest>")]
    pub apple_pay: Option<ApplepayConnectorMetadataRequest>,
    #[smithy(value_type = "Option<AirwallexData>")]
    pub airwallex: Option<AirwallexData>,
    #[smithy(value_type = "Option<NoonData>")]
    pub noon: Option<NoonData>,
    #[smithy(value_type = "Option<BraintreeData>")]
    pub braintree: Option<BraintreeData>,
    #[smithy(value_type = "Option<AdyenConnectorMetadata>")]
    pub adyen: Option<AdyenConnectorMetadata>,
}

impl ConnectorMetadata {
    pub fn from_value(
        value: pii::SecretSerdeValue,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ParsingError> {
        value
            .parse_value::<Self>("ConnectorMetadata")
            .change_context(common_utils::errors::ParsingError::StructParseFailure(
                "Metadata",
            ))
    }
    pub fn get_apple_pay_certificates(self) -> Option<(Secret<String>, Secret<String>)> {
        self.apple_pay.and_then(|applepay_metadata| {
            applepay_metadata
                .session_token_data
                .map(|session_token_data| {
                    let SessionTokenInfo {
                        certificate,
                        certificate_keys,
                        ..
                    } = session_token_data;
                    (certificate, certificate_keys)
                })
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AirwallexData {
    /// payload required by airwallex
    #[smithy(value_type = "Option<String>")]
    payload: Option<String>,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NoonData {
    /// Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by the merchant in Noon's Dashboard)
    #[smithy(value_type = "Option<String>")]
    pub order_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BraintreeData {
    /// Information about the merchant_account_id that merchant wants to specify at connector level.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub merchant_account_id: Option<Secret<String>>,
    /// Information about the merchant_config_currency that merchant wants to specify at connector level.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub merchant_config_currency: Option<api_enums::Currency>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AdyenConnectorMetadata {
    #[smithy(value_type = "AdyenTestingData")]
    pub testing: AdyenTestingData,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AdyenTestingData {
    /// Holder name to be sent to Adyen for a card payment(CIT) or a generic payment(MIT). This value overrides the values for card.card_holder_name and applies during both CIT and MIT payment transactions.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplepayConnectorMetadataRequest {
    #[smithy(value_type = "Option<SessionTokenInfo>")]
    pub session_token_data: Option<SessionTokenInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplepaySessionTokenData {
    pub apple_pay: ApplePayMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplepayCombinedSessionTokenData {
    pub apple_pay_combined: ApplePayCombinedMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplepaySessionTokenMetadata {
    ApplePayCombined(ApplePayCombinedMetadata),
    ApplePay(ApplePayMetadata),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplePayMetadata {
    pub payment_request_data: PaymentRequestMetadata,
    pub session_token_data: SessionTokenInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplePayCombinedMetadata {
    Simplified {
        payment_request_data: PaymentRequestMetadata,
        session_token_data: SessionTokenForSimplifiedApplePay,
    },
    Manual {
        payment_request_data: PaymentRequestMetadata,
        session_token_data: SessionTokenInfo,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentRequestMetadata {
    pub supported_networks: Vec<String>,
    pub merchant_capabilities: Vec<String>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SessionTokenInfo {
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub certificate: Secret<String>,
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub certificate_keys: Secret<String>,
    #[smithy(value_type = "String")]
    pub merchant_identifier: String,
    #[smithy(value_type = "String")]
    pub display_name: String,
    #[smithy(value_type = "ApplepayInitiative")]
    pub initiative: ApplepayInitiative,
    #[smithy(value_type = "Option<String>")]
    pub initiative_context: Option<String>,
    #[schema(value_type = Option<CountryAlpha2>)]
    #[smithy(value_type = "Option<CountryAlpha2>")]
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    #[serde(flatten)]
    pub payment_processing_details_at: Option<PaymentProcessingDetailsAt>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Display, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ApplepayInitiative {
    Web,
    Ios,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel)]
#[serde(tag = "payment_processing_details_at")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum PaymentProcessingDetailsAt {
    #[smithy(value_type = "PaymentProcessingDetails")]
    Hyperswitch(PaymentProcessingDetails),
    #[smithy(value_type = "smithy.api#Unit")]
    Connector,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentProcessingDetails {
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub payment_processing_certificate: Secret<String>,
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub payment_processing_certificate_key: Secret<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SessionTokenForSimplifiedApplePay {
    pub initiative_context: String,
    #[schema(value_type = Option<CountryAlpha2>)]
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayWalletDetails {
    pub google_pay: GooglePayDetails,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayDetails {
    pub provider_details: GooglePayProviderDetails,
    pub cards: GpayAllowedMethodsParameters,
}

// Google Pay Provider Details can of two types: GooglePayMerchantDetails or GooglePayHyperSwitchDetails
// GooglePayHyperSwitchDetails is not implemented yet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GooglePayProviderDetails {
    GooglePayMerchantDetails(GooglePayMerchantDetails),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayMerchantDetails {
    pub merchant_info: GooglePayMerchantInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayMerchantInfo {
    pub merchant_name: String,
    pub merchant_id: Option<String>,
    pub tokenization_specification: GooglePayTokenizationSpecification,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayTokenizationSpecification {
    #[serde(rename = "type")]
    pub tokenization_type: GooglePayTokenizationType,
    pub parameters: GooglePayTokenizationParameters,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum GooglePayTokenizationType {
    PaymentGateway,
    Direct,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayTokenizationParameters {
    pub gateway: Option<String>,
    pub public_key: Option<Secret<String>>,
    pub private_key: Option<Secret<String>>,
    pub recipient_id: Option<Secret<String>>,
    pub gateway_merchant_id: Option<Secret<String>>,
    pub stripe_publishable_key: Option<Secret<String>>,
    pub stripe_version: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(tag = "wallet_name")]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum SessionToken {
    /// The session response structure for Google Pay
    #[smithy(value_type = "GpaySessionTokenResponse")]
    GooglePay(Box<GpaySessionTokenResponse>),
    /// The session response structure for Samsung Pay
    #[smithy(value_type = "SamsungPaySessionTokenResponse")]
    SamsungPay(Box<SamsungPaySessionTokenResponse>),
    /// The session response structure for Klarna
    #[smithy(value_type = "KlarnaSessionTokenResponse")]
    Klarna(Box<KlarnaSessionTokenResponse>),
    /// The session response structure for PayPal
    #[smithy(value_type = "PaypalSessionTokenResponse")]
    Paypal(Box<PaypalSessionTokenResponse>),
    /// The session response structure for Apple Pay
    #[smithy(value_type = "ApplepaySessionTokenResponse")]
    ApplePay(Box<ApplepaySessionTokenResponse>),
    /// Session token for OpenBanking PIS flow
    #[smithy(value_type = "OpenBankingSessionToken")]
    OpenBanking(OpenBankingSessionToken),
    /// The session response structure for Paze
    #[smithy(value_type = "PazeSessionTokenResponse")]
    Paze(Box<PazeSessionTokenResponse>),
    /// The sessions response structure for ClickToPay
    #[smithy(value_type = "ClickToPaySessionResponse")]
    ClickToPay(Box<ClickToPaySessionResponse>),
    /// The session response structure for Amazon Pay
    #[smithy(value_type = "AmazonPaySessionTokenResponse")]
    AmazonPay(Box<AmazonPaySessionTokenResponse>),
    /// Whenever there is no session token response or an error in session response
    #[smithy(value_type = "smithy.api#Unit")]
    NoSessionTokenReceived,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VaultSessionDetails {
    Vgs(VgsSessionDetails),
    HyperswitchVault(HyperswitchVaultSessionDetails),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct VgsSessionDetails {
    /// The identifier of the external vault
    #[schema(value_type = String)]
    pub external_vault_id: Secret<String>,
    /// The environment for the external vault initiation
    pub sdk_env: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct HyperswitchVaultSessionDetails {
    /// Session ID for Hyperswitch Vault
    #[schema(value_type = String)]
    pub payment_method_session_id: Secret<String>,
    /// Client secret for Hyperswitch Vault
    #[schema(value_type = String)]
    pub client_secret: Secret<String>,
    /// Publishable key for Hyperswitch Vault
    #[schema(value_type = String)]
    pub publishable_key: Secret<String>,
    /// Profile ID for Hyperswitch Vault
    #[schema(value_type = String)]
    pub profile_id: Secret<String>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PazeSessionTokenResponse {
    /// Paze Client ID
    #[smithy(value_type = "String")]
    pub client_id: String,
    /// Client Name to be displayed on the Paze screen
    #[smithy(value_type = "String")]
    pub client_name: String,
    /// Paze Client Profile ID
    #[smithy(value_type = "String")]
    pub client_profile_id: String,
    /// The transaction currency code
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub transaction_currency_code: api_enums::Currency,
    /// The transaction amount
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub transaction_amount: StringMajorUnit,
    /// Email Address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    #[smithy(value_type = "Option<String>")]
    pub email_address: Option<Email>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum GpaySessionTokenResponse {
    /// Google pay response involving third party sdk
    #[smithy(value_type = "GooglePayThirdPartySdk")]
    ThirdPartyResponse(GooglePayThirdPartySdk),
    /// Google pay session response for non third party sdk
    #[smithy(value_type = "GooglePaySessionResponse")]
    GooglePaySession(GooglePaySessionResponse),
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePayThirdPartySdk {
    /// Identifier for the delayed session response
    #[smithy(value_type = "bool")]
    pub delayed_session_token: bool,
    /// The name of the connector
    #[smithy(value_type = "String")]
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    #[smithy(value_type = "SdkNextAction")]
    pub sdk_next_action: SdkNextAction,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GooglePaySessionResponse {
    /// The merchant info
    #[smithy(value_type = "GpayMerchantInfo")]
    pub merchant_info: GpayMerchantInfo,
    /// Is shipping address required
    #[smithy(value_type = "bool")]
    pub shipping_address_required: bool,
    /// Is email required
    #[smithy(value_type = "bool")]
    pub email_required: bool,
    /// Shipping address parameters
    #[smithy(value_type = "GpayShippingAddressParameters")]
    pub shipping_address_parameters: GpayShippingAddressParameters,
    /// List of the allowed payment methods
    #[smithy(value_type = "Vec<GpayAllowedPaymentMethods>")]
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
    /// The transaction info Google Pay requires
    #[smithy(value_type = "GpayTransactionInfo")]
    pub transaction_info: GpayTransactionInfo,
    /// Identifier for the delayed session response
    #[smithy(value_type = "bool")]
    pub delayed_session_token: bool,
    /// The name of the connector
    #[smithy(value_type = "String")]
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    #[smithy(value_type = "SdkNextAction")]
    pub sdk_next_action: SdkNextAction,
    /// Secrets for sdk display and payment
    #[smithy(value_type = " Option<SecretInfoToInitiateSdk>")]
    pub secrets: Option<SecretInfoToInitiateSdk>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPaySessionTokenResponse {
    /// Samsung Pay API version
    #[smithy(value_type = "String")]
    pub version: String,
    /// Samsung Pay service ID to which session call needs to be made
    #[smithy(value_type = "String")]
    pub service_id: String,
    /// Order number of the transaction
    #[smithy(value_type = "String")]
    pub order_number: String,
    /// Field containing merchant information
    #[serde(rename = "merchant")]
    #[smithy(value_type = "SamsungPayMerchantPaymentInformation")]
    pub merchant_payment_information: SamsungPayMerchantPaymentInformation,
    /// Field containing the payment amount
    #[smithy(value_type = "SamsungPayAmountDetails")]
    pub amount: SamsungPayAmountDetails,
    /// Payment protocol type
    #[smithy(value_type = "SamsungPayProtocolType")]
    pub protocol: SamsungPayProtocolType,
    /// List of supported card brands
    #[schema(value_type = Vec<String>)]
    #[smithy(value_type = "Vec<String>")]
    pub allowed_brands: Vec<String>,
    /// Is billing address required to be collected from wallet
    #[smithy(value_type = "bool")]
    pub billing_address_required: bool,
    /// Is shipping address required to be collected from wallet
    #[smithy(value_type = "bool")]
    pub shipping_address_required: bool,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum SamsungPayProtocolType {
    Protocol3ds,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayMerchantPaymentInformation {
    /// Merchant name, this will be displayed on the Samsung Pay screen
    #[smithy(value_type = "String")]
    pub name: String,
    /// Merchant domain that process payments, required for web payments
    #[smithy(value_type = "Option<String>")]
    pub url: Option<String>,
    /// Merchant country code
    #[schema(value_type = CountryAlpha2, example = "US")]
    #[smithy(value_type = "CountryAlpha2")]
    pub country_code: api_enums::CountryAlpha2,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SamsungPayAmountDetails {
    #[serde(rename = "option")]
    #[smithy(value_type = "SamsungPayAmountFormat")]
    /// Amount format to be displayed
    pub amount_format: SamsungPayAmountFormat,
    /// The currency code
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency_code: api_enums::Currency,
    /// The total amount of the transaction
    #[serde(rename = "total")]
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub total_amount: StringMajorUnit,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum SamsungPayAmountFormat {
    /// Display the total amount only
    FormatTotalPriceOnly,
    /// Display "Total (Estimated amount)" and total amount
    FormatTotalEstimatedAmount,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GpayShippingAddressParameters {
    /// Is shipping phone number required
    #[smithy(value_type = "bool")]
    pub phone_number_required: bool,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct KlarnaSessionTokenResponse {
    /// The session token for Klarna
    #[smithy(value_type = "String")]
    pub session_token: String,
    /// The identifier for the session
    #[smithy(value_type = "String")]
    pub session_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaypalFlow {
    Checkout,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaypalTransactionInfo {
    /// Paypal flow type
    #[schema(value_type = PaypalFlow, example = "checkout")]
    pub flow: PaypalFlow,
    /// Currency code
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// Total price
    #[schema(value_type = String, example = "38.02")]
    pub total_price: StringMajorUnit,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaypalSessionTokenResponse {
    /// Name of the connector
    #[smithy(value_type = "String")]
    pub connector: String,
    /// The session token for PayPal
    #[smithy(value_type = "String")]
    pub session_token: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    #[smithy(value_type = "SdkNextAction")]
    pub sdk_next_action: SdkNextAction,
    /// Authorization token used by client to initiate sdk
    pub client_token: Option<String>,
    /// The transaction info Paypal requires
    pub transaction_info: Option<PaypalTransactionInfo>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct OpenBankingSessionToken {
    /// The session token for OpenBanking Connectors
    #[smithy(value_type = "String")]
    pub open_banking_session_token: String,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplepaySessionTokenResponse {
    /// Session object for Apple Pay
    /// The session_token_data will be null for iOS devices because the Apple Pay session call is skipped, as there is no web domain involved
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<ApplePaySessionResponse>")]
    pub session_token_data: Option<ApplePaySessionResponse>,
    /// Payment request object for Apple Pay
    #[smithy(value_type = "Option<ApplePayPaymentRequest>")]
    pub payment_request_data: Option<ApplePayPaymentRequest>,
    /// The session token is w.r.t this connector
    #[smithy(value_type = "String")]
    pub connector: String,
    /// Identifier for the delayed session response
    #[smithy(value_type = "bool")]
    pub delayed_session_token: bool,
    /// The next action for the sdk (ex: calling confirm or sync call)
    #[smithy(value_type = "SdkNextAction")]
    pub sdk_next_action: SdkNextAction,
    /// The connector transaction id
    #[smithy(value_type = "Option<String>")]
    pub connector_reference_id: Option<String>,
    /// The public key id is to invoke third party sdk
    #[smithy(value_type = "Option<String>")]
    pub connector_sdk_public_key: Option<String>,
    /// The connector merchant id
    #[smithy(value_type = "Option<String>")]
    pub connector_merchant_id: Option<String>,
}

#[derive(
    Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Clone, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SdkNextAction {
    /// The type of next action
    #[smithy(value_type = "NextActionCall")]
    pub next_action: NextActionCall,
}

#[derive(
    Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Clone, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum NextActionCall {
    /// The next action call is Post Session Tokens
    PostSessionTokens,
    /// The next action call is confirm
    Confirm,
    /// The next action call is sync
    Sync,
    /// The next action call is Complete Authorize
    CompleteAuthorize,
    /// The next action is to await for a merchant callback
    AwaitMerchantCallback,
    /// The next action is to deny the payment with an error message
    Deny { message: String },
    /// The next action is to perform eligibility check
    EligibilityCheck,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[serde(untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ApplePaySessionResponse {
    ///  We get this session response, when third party sdk is involved
    #[smithy(value_type = "ThirdPartySdkSessionResponse")]
    ThirdPartySdk(ThirdPartySdkSessionResponse),
    ///  We get this session response, when there is no involvement of third party sdk
    /// This is the common response most of the times
    #[smithy(value_type = "NoThirdPartySdkSessionResponse")]
    NoThirdPartySdk(NoThirdPartySdkSessionResponse),
    /// This is for the empty session response
    #[smithy(value_type = "smithy.api#Unit")]
    NoSessionResponse(NullObject),
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize, SmithyModel,
)]
#[serde(rename_all(deserialize = "camelCase"))]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NoThirdPartySdkSessionResponse {
    /// Timestamp at which session is requested
    #[smithy(value_type = "u64")]
    pub epoch_timestamp: u64,
    /// Timestamp at which session expires
    #[smithy(value_type = "u64")]
    pub expires_at: u64,
    /// The identifier for the merchant session
    #[smithy(value_type = "String")]
    pub merchant_session_identifier: String,
    /// Apple pay generated unique ID (UUID) value
    #[smithy(value_type = "String")]
    pub nonce: String,
    /// The identifier for the merchant
    #[smithy(value_type = "String")]
    pub merchant_identifier: String,
    /// The domain name of the merchant which is registered in Apple Pay
    #[smithy(value_type = "String")]
    pub domain_name: String,
    /// The name to be displayed on Apple Pay button
    #[smithy(value_type = "String")]
    pub display_name: String,
    /// A string which represents the properties of a payment
    #[smithy(value_type = "String")]
    pub signature: String,
    /// The identifier for the operational analytics
    #[smithy(value_type = "String")]
    pub operational_analytics_identifier: String,
    /// The number of retries to get the session response
    #[smithy(value_type = "u8")]
    pub retries: u8,
    /// The identifier for the connector transaction
    #[smithy(value_type = "String")]
    pub psp_id: String,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ThirdPartySdkSessionResponse {
    #[smithy(value_type = "SecretInfoToInitiateSdk")]
    pub secrets: SecretInfoToInitiateSdk,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SecretInfoToInitiateSdk {
    // Authorization secrets used by client to initiate sdk
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub display: Secret<String>,
    // Authorization secrets used by client for payment
    #[schema(value_type = String)]
    #[smithy(value_type = "Option<String>")]
    pub payment: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayPaymentRequest {
    /// The code for country
    #[schema(value_type = CountryAlpha2, example = "US")]
    #[smithy(value_type = "CountryAlpha2")]
    pub country_code: api_enums::CountryAlpha2,
    /// The code for currency
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency_code: api_enums::Currency,
    /// Represents the total for the payment.
    #[smithy(value_type = "AmountInfo")]
    pub total: AmountInfo,
    /// The list of merchant capabilities(ex: whether capable of 3ds or no-3ds)
    #[smithy(value_type = "Option<Vec<String>>")]
    pub merchant_capabilities: Option<Vec<String>>,
    /// The list of supported networks
    #[smithy(value_type = "Option<Vec<String>>")]
    pub supported_networks: Option<Vec<String>>,
    #[smithy(value_type = "Option<String>")]
    pub merchant_identifier: Option<String>,
    /// The required billing contact fields for connector
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<Vec<ApplePayAddressParameters>>")]
    pub required_billing_contact_fields: Option<ApplePayBillingContactFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The required shipping contacht fields for connector
    #[smithy(value_type = "Option<Vec<ApplePayAddressParameters>>")]
    pub required_shipping_contact_fields: Option<ApplePayShippingContactFields>,
    /// Recurring payment request for apple pay Merchant Token
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<ApplePayRecurringPaymentRequest>")]
    pub recurring_payment_request: Option<ApplePayRecurringPaymentRequest>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayRecurringPaymentRequest {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    #[smithy(value_type = "String")]
    pub payment_description: String,
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    #[smithy(value_type = "ApplePayRegularBillingRequest")]
    pub regular_billing: ApplePayRegularBillingRequest,
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<String>")]
    pub billing_agreement: Option<String>,
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    #[smithy(value_type = "String")]
    pub management_u_r_l: common_utils::types::Url,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayRegularBillingRequest {
    /// The amount of the recurring payment
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub amount: StringMajorUnit,
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    #[smithy(value_type = "String")]
    pub label: String,
    /// The time that the payment occurs as part of a successful transaction
    #[smithy(value_type = "ApplePayPaymentTiming")]
    pub payment_timing: ApplePayPaymentTiming,
    /// The date of the first payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub recurring_payment_start_date: Option<PrimitiveDateTime>,
    /// The date of the final payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub recurring_payment_end_date: Option<PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<RecurringPaymentIntervalUnit>")]
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    #[serde(skip_serializing_if = "Option::is_none")]
    #[smithy(value_type = "Option<i32>")]
    pub recurring_payment_interval_count: Option<i32>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ApplePayPaymentTiming {
    /// A value that specifies that the payment occurs when the transaction is complete
    Immediate,
    /// A value that specifies that the payment occurs on a regular basis
    Recurring,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct ApplePayBillingContactFields(pub Vec<ApplePayAddressParameters>);
#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct ApplePayShippingContactFields(pub Vec<ApplePayAddressParameters>);

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize, SmithyModel,
)]
#[serde(rename_all = "camelCase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ApplePayAddressParameters {
    PostalAddress,
    Phone,
    Email,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmountInfo {
    /// The label must be the name of the merchant.
    #[smithy(value_type = "String")]
    pub label: String,
    /// A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending.
    #[serde(rename = "type")]
    #[smithy(value_type = "Option<String>")]
    pub total_type: Option<String>,
    /// The total amount for the payment in majot unit string (Ex: 38.02)
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub amount: StringMajorUnit,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayErrorResponse {
    pub status_code: String,
    pub status_message: String,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPaySessionTokenResponse {
    /// Amazon Pay merchant account identifier
    #[smithy(value_type = "String")]
    pub merchant_id: String,
    /// Ledger currency provided during registration for the given merchant identifier
    #[schema(example = "USD", value_type = Currency)]
    #[smithy(value_type = "Currency")]
    pub ledger_currency: common_enums::Currency,
    /// Amazon Pay store ID
    #[smithy(value_type = "String")]
    pub store_id: String,
    /// Payment flow for charging the buyer
    #[smithy(value_type = "AmazonPayPaymentIntent")]
    pub payment_intent: AmazonPayPaymentIntent,
    /// The total shipping costs
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub total_shipping_amount: StringMajorUnit,
    /// The total tax amount for the order
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub total_tax_amount: StringMajorUnit,
    /// The total amount for items in the cart
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub total_base_amount: StringMajorUnit,
    /// The delivery options available for the provided address
    #[smithy(value_type = "Vec<AmazonPayDeliveryOptions>")]
    pub delivery_options: Vec<AmazonPayDeliveryOptions>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum AmazonPayPaymentIntent {
    /// Create a Charge Permission to authorize and capture funds at a later time
    Confirm,
    /// Authorize funds immediately and capture at a later time
    Authorize,
    /// Authorize and capture funds immediately
    AuthorizeWithCapture,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPayDeliveryOptions {
    /// Delivery Option identifier
    #[smithy(value_type = "String")]
    pub id: String,
    /// Total delivery cost
    #[smithy(value_type = "AmazonPayDeliveryPrice")]
    pub price: AmazonPayDeliveryPrice,
    /// Shipping method details
    #[smithy(value_type = "AmazonPayShippingMethod")]
    pub shipping_method: AmazonPayShippingMethod,
    /// Specifies if this delivery option is the default
    #[smithy(value_type = "bool")]
    pub is_default: bool,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPayDeliveryPrice {
    /// Transaction amount in MinorUnit
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    #[serde(skip_deserializing)]
    /// Transaction amount in StringMajorUnit
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub display_amount: StringMajorUnit,
    /// Transaction currency code in ISO 4217 format
    #[schema(example = "USD", value_type = Currency)]
    #[smithy(value_type = "Currency")]
    pub currency_code: common_enums::Currency,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AmazonPayShippingMethod {
    /// Name of the shipping method
    #[smithy(value_type = "String")]
    pub shipping_method_name: String,
    /// Code of the shipping method
    #[smithy(value_type = "String")]
    pub shipping_method_code: String,
}

impl AmazonPayDeliveryOptions {
    pub fn parse_delivery_options_request(
        delivery_options_request: &[serde_json::Value],
    ) -> Result<Vec<Self>, common_utils::errors::ParsingError> {
        delivery_options_request
            .iter()
            .map(|option| {
                serde_json::from_value(option.clone()).map_err(|_| {
                    common_utils::errors::ParsingError::StructParseFailure(
                        "AmazonPayDeliveryOptions",
                    )
                })
            })
            .collect()
    }

    pub fn get_default_delivery_amount(
        delivery_options: Vec<Self>,
    ) -> Result<MinorUnit, error_stack::Report<ValidationError>> {
        let mut default_options = delivery_options
            .into_iter()
            .filter(|delivery_option| delivery_option.is_default);

        match (default_options.next(), default_options.next()) {
            (Some(default_option), None) => Ok(default_option.price.amount),
            _ => Err(ValidationError::InvalidValue {
                message: "Amazon Pay Delivery Option".to_string(),
            })
            .attach_printable("Expected exactly one default Amazon Pay Delivery Option"),
        }
    }

    pub fn validate_currency(
        currency_code: common_enums::Currency,
        amazonpay_supported_currencies: HashSet<common_enums::Currency>,
    ) -> Result<(), ValidationError> {
        if !amazonpay_supported_currencies.contains(&currency_code) {
            return Err(ValidationError::InvalidValue {
                message: format!("{currency_code:?} is not a supported currency."),
            });
        }

        Ok(())
    }

    pub fn insert_display_amount(
        delivery_options: &mut Vec<Self>,
        currency_code: common_enums::Currency,
    ) -> Result<(), error_stack::Report<common_utils::errors::ParsingError>> {
        let required_amount_type = common_utils::types::StringMajorUnitForCore;
        for option in delivery_options {
            let display_amount = required_amount_type
                .convert(option.price.amount, currency_code)
                .change_context(common_utils::errors::ParsingError::I64ToStringConversionFailure)?;

            option.price.display_amount = display_amount;
        }

        Ok(())
    }
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    #[schema(value_type = String)]
    pub client_secret: Secret<String, pii::ClientSecret>,
    /// The list of session token object
    pub session_token: Vec<SessionToken>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::GlobalPaymentId,
    /// The list of session token object
    pub session_token: Vec<SessionToken>,
    /// External vault session details
    pub vault_details: Option<VaultSessionDetails>,
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentRetrieveBody {
    /// The identifier for the Merchant Account.
    #[schema(value_type = Option<String>)]
    pub merchant_id: Option<id_type::MerchantId>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: Option<bool>,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<String>,
    /// If enabled provides list of captures linked to latest attempt
    pub expand_captures: Option<bool>,
    /// If enabled provides list of attempts linked to payment intent
    pub expand_attempts: Option<bool>,
    /// If enabled, provides whole connector response
    pub all_keys_required: Option<bool>,
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentRetrieveBodyWithCredentials {
    /// The identifier for payment.
    pub payment_id: id_type::PaymentId,
    /// The identifier for the Merchant Account.
    #[schema(value_type = Option<String>)]
    pub merchant_id: Option<id_type::MerchantId>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: Option<bool>,
    /// Merchant connector details used to make payments.
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsCompleteAuthorizeRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    pub payment_id: id_type::PaymentId,
    /// The shipping address for the payment
    pub shipping: Option<Address>,
    /// Client Secret
    #[schema(value_type = String)]
    pub client_secret: Secret<String>,
    /// Indicates if 3DS method data was successfully completed or not
    pub threeds_method_comp_ind: Option<ThreeDsCompletionIndicator>,
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PaymentsCancelRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// The reason for the payment cancel
    #[smithy(value_type = "Option<String>")]
    pub cancellation_reason: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>, deprecated)]
    #[smithy(value_type = "Option<MerchantConnectorDetailsWrap>")]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

/// Request to cancel a payment when the payment is already captured
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsCancelPostCaptureRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// The reason for the payment cancel
    pub cancellation_reason: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
/// Request constructed internally for extending authorization
pub struct PaymentsExtendAuthorizationRequest {
    /// The identifier for the payment
    pub payment_id: id_type::PaymentId,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsIncrementalAuthorizationRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// The total amount including previously authorized amount and additional amount
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// Reason for incremental authorization
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsExternalAuthenticationRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// Client Secret
    #[schema(value_type = String)]
    pub client_secret: Secret<String>,
    /// SDK Information if request is from SDK
    pub sdk_information: Option<SdkInformation>,
    /// Device Channel indicating whether request is coming from App or Browser
    pub device_channel: DeviceChannel,
    /// Indicates if 3DS method data was successfully completed or not
    pub threeds_method_comp_ind: ThreeDsCompletionIndicator,
}

/// Indicates if 3DS method data was successfully completed or not
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsManualUpdateRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// The identifier for the payment attempt
    pub attempt_id: String,
    /// Merchant ID
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// The status of the attempt
    pub attempt_status: Option<enums::AttemptStatus>,
    /// Error code of the connector
    pub error_code: Option<String>,
    /// Error message of the connector
    pub error_message: Option<String>,
    /// Error reason of the connector
    pub error_reason: Option<String>,
    /// A unique identifier for a payment provided by the connector
    pub connector_transaction_id: Option<String>,
    /// The amount that can be captured on the payment.
    pub amount_capturable: Option<MinorUnit>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsManualUpdateResponse {
    /// The identifier for the payment
    pub payment_id: id_type::PaymentId,
    /// The identifier for the payment attempt
    pub attempt_id: String,
    /// Merchant ID
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// The status of the attempt
    pub attempt_status: enums::AttemptStatus,
    /// Error code of the connector
    pub error_code: Option<String>,
    /// Error message of the connector
    pub error_message: Option<String>,
    /// Error reason of the connector
    pub error_reason: Option<String>,
    /// A unique identifier for a payment provided by the connector
    pub connector_transaction_id: Option<String>,
    /// The amount that can be captured on the payment.
    pub amount_capturable: Option<MinorUnit>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Indicates if 3DS method data was successfully completed or not
pub enum ThreeDsCompletionIndicator {
    /// 3DS method successfully completed
    #[serde(rename = "Y")]
    Success,
    /// 3DS method was not successful
    #[serde(rename = "N")]
    Failure,
    /// 3DS method URL was unavailable
    #[serde(rename = "U")]
    NotAvailable,
}

/// Device Channel indicating whether request is coming from App or Browser
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema, Eq, PartialEq)]
pub enum DeviceChannel {
    #[serde(rename = "APP")]
    App,
    #[serde(rename = "BRW")]
    Browser,
}

/// SDK Information if request is from SDK
#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct SdkInformation {
    /// Unique ID created on installations of the 3DS Requestor App on a Consumer Device
    pub sdk_app_id: String,
    /// JWE Object containing data encrypted by the SDK for the DS to decrypt
    pub sdk_enc_data: String,
    /// Public key component of the ephemeral key pair generated by the 3DS SDK
    pub sdk_ephem_pub_key: HashMap<String, String>,
    /// Unique transaction identifier assigned by the 3DS SDK
    pub sdk_trans_id: String,
    /// Identifies the vendor and version for the 3DS SDK that is integrated in a 3DS Requestor App
    pub sdk_reference_number: String,
    /// Indicates maximum amount of time in minutes
    pub sdk_max_timeout: u8,
    /// Indicates the type of 3DS SDK
    pub sdk_type: Option<SdkType>,
}

/// Enum representing the type of 3DS SDK.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub enum SdkType {
    #[serde(rename = "01")]
    DefaultSdk,
    #[serde(rename = "02")]
    SplitSdk,
    #[serde(rename = "03")]
    LimitedSdk,
    #[serde(rename = "04")]
    BrowserSdk,
    #[serde(rename = "05")]
    ShellSdk,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct ListMethodsForPaymentsRequest {
    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    #[schema(max_length = 30, min_length = 30, example = "secret_k2uj3he2893eiu2d")]
    pub client_secret: Option<String>,

    /// The two-letter ISO currency code
    #[serde(deserialize_with = "parse_comma_separated", default)]
    #[schema(value_type = Option<Vec<CountryAlpha2>>, example = json!(["US", "UK", "IN"]))]
    pub accepted_countries: Option<Vec<api_enums::CountryAlpha2>>,

    /// Filter by amount
    #[schema(example = 60)]
    pub amount: Option<MinorUnit>,

    /// The three-letter ISO currency code
    #[serde(deserialize_with = "parse_comma_separated", default)]
    #[schema(value_type = Option<Vec<Currency>>,example = json!(["USD", "EUR"]))]
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,

    /// Indicates whether the payment method supports recurring payments. Optional.
    #[schema(example = true)]
    pub recurring_enabled: Option<bool>,

    /// Indicates whether the payment method is eligible for card networks
    #[serde(deserialize_with = "parse_comma_separated", default)]
    #[schema(value_type = Option<Vec<CardNetwork>>, example = json!(["visa", "mastercard"]))]
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,

    /// Indicates the limit of last used payment methods
    #[schema(example = 1)]
    pub limit: Option<i64>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodListResponseForPayments {
    /// The list of payment methods that are enabled for the business profile
    pub payment_methods_enabled: Vec<ResponsePaymentMethodTypesForPayments>,

    /// The list of payment methods that are saved by the given customer
    /// This field is only returned if the customer_id is provided in the request
    #[schema(value_type = Option<Vec<CustomerPaymentMethodResponseItem>>)]
    pub customer_payment_methods: Option<Vec<payment_methods::CustomerPaymentMethodResponseItem>>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, ToSchema, PartialEq)]
pub struct ResponsePaymentMethodTypesForPayments {
    /// The payment method type enabled
    #[schema(example = "pay_later", value_type = PaymentMethod)]
    pub payment_method_type: common_enums::PaymentMethod,

    /// The payment method subtype enabled
    #[schema(example = "klarna", value_type = PaymentMethodType)]
    pub payment_method_subtype: common_enums::PaymentMethodType,

    /// The payment experience for the payment method
    #[schema(value_type = Option<Vec<PaymentExperience>>)]
    pub payment_experience: Option<Vec<common_enums::PaymentExperience>>,

    /// payment method subtype specific information
    #[serde(flatten)]
    #[schema(value_type = Option<PaymentMethodSubtypeSpecificData>)]
    pub extra_information: Option<payment_methods::PaymentMethodSubtypeSpecificData>,

    /// Required fields for the payment_method_type.
    /// This is the union of all the required fields for the payment method type enabled in all the connectors.
    #[schema(value_type = RequiredFieldInfo)]
    pub required_fields: Vec<payment_methods::RequiredFieldInfo>,

    /// surcharge details for this payment method type if exists
    #[schema(value_type = Option<SurchargeDetailsResponse>)]
    pub surcharge_details: Option<payment_methods::SurchargeDetailsResponse>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsExternalAuthenticationResponse {
    /// Indicates the transaction status
    #[serde(rename = "trans_status")]
    #[schema(value_type = TransactionStatus)]
    pub transaction_status: common_enums::TransactionStatus,
    /// Access Server URL to be used for challenge submission
    pub acs_url: Option<String>,
    /// Challenge request which should be sent to acs_url
    pub challenge_request: Option<String>,
    /// Challenge request key which should be set as form field name for creq
    pub challenge_request_key: Option<String>,
    /// Unique identifier assigned by the EMVCo(Europay, Mastercard and Visa)
    pub acs_reference_number: Option<String>,
    /// Unique identifier assigned by the ACS to identify a single transaction
    pub acs_trans_id: Option<String>,
    /// Unique identifier assigned by the 3DS Server to identify a single transaction
    pub three_dsserver_trans_id: Option<String>,
    /// Contains the JWS object created by the ACS for the ARes(Authentication Response) message
    pub acs_signed_content: Option<String>,
    /// Three DS Requestor URL
    pub three_ds_requestor_url: String,
    /// Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred
    pub three_ds_requestor_app_url: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsApproveRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsRejectRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentsStartRequest {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    pub payment_id: id_type::PaymentId,
    /// The identifier for the Merchant Account.
    pub merchant_id: id_type::MerchantId,
    /// The identifier for the payment transaction
    pub attempt_id: String,
}

/// additional data that might be required by hyperswitch
#[cfg(feature = "v2")]
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    #[schema(value_type = Option<RedirectResponse>)]
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    #[schema(value_type = Option<Vec<String>>)]
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
    /// revenue recovery data for payment intent
    pub revenue_recovery: Option<PaymentRevenueRecoveryMetadata>,
}

#[cfg(feature = "v2")]
impl FeatureMetadata {
    pub fn get_retry_count(&self) -> Option<u16> {
        self.revenue_recovery
            .as_ref()
            .map(|metadata| metadata.total_retry_count)
    }

    pub fn set_payment_revenue_recovery_metadata_using_api(
        self,
        payment_revenue_recovery_metadata: PaymentRevenueRecoveryMetadata,
    ) -> Self {
        Self {
            redirect_response: self.redirect_response,
            search_tags: self.search_tags,
            apple_pay_recurring_details: self.apple_pay_recurring_details,
            revenue_recovery: Some(payment_revenue_recovery_metadata),
        }
    }
}

/// additional data that might be required by hyperswitch
#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    #[schema(value_type = Option<RedirectResponse>)]
    #[smithy(value_type = "Option<RedirectResponse>")]
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    #[schema(value_type = Option<Vec<String>>)]
    #[smithy(value_type = "Option<Vec<String>>")]
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    #[smithy(value_type = "Option<ApplePayRecurringDetails>")]
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayRecurringDetails {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    #[smithy(value_type = "String")]
    pub payment_description: String,
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    #[smithy(value_type = "ApplePayRegularBillingDetails")]
    pub regular_billing: ApplePayRegularBillingDetails,
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    #[smithy(value_type = "Option<String>")]
    pub billing_agreement: Option<String>,
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    #[smithy(value_type = "String")]
    pub management_url: common_utils::types::Url,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ApplePayRegularBillingDetails {
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    pub label: String,
    /// The date of the first payment
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub recurring_payment_start_date: Option<PrimitiveDateTime>,
    /// The date of the final payment
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub recurring_payment_end_date: Option<PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    #[smithy(value_type = "Option<RecurringPaymentIntervalUnit>")]
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    #[smithy(value_type = "Option<i32>")]
    pub recurring_payment_interval_count: Option<i32>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum RecurringPaymentIntervalUnit {
    Year,
    Month,
    Day,
    Hour,
    Minute,
}

///frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct FrmMessage {
    #[smithy(value_type = "String")]
    pub frm_name: String,
    #[smithy(value_type = "Option<String>")]
    pub frm_transaction_id: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub frm_transaction_type: Option<String>,
    #[smithy(value_type = "Option<String>")]
    pub frm_status: Option<String>,
    #[smithy(value_type = "Option<i32>")]
    pub frm_score: Option<i32>,
    #[smithy(value_type = "Option<Object>")]
    pub frm_reason: Option<serde_json::Value>,
    #[smithy(value_type = "Option<String>")]
    pub frm_error: Option<String>,
}

#[cfg(feature = "v2")]
mod payment_id_type {
    use std::{borrow::Cow, fmt};

    use serde::{
        de::{self, Visitor},
        Deserializer,
    };

    use super::PaymentIdType;

    struct PaymentIdVisitor;
    struct OptionalPaymentIdVisitor;

    impl Visitor<'_> for PaymentIdVisitor {
        type Value = PaymentIdType;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            common_utils::id_type::GlobalPaymentId::try_from(Cow::Owned(value.to_string()))
                .map_err(de::Error::custom)
                .map(PaymentIdType::PaymentIntentId)
        }
    }

    impl<'de> Visitor<'de> for OptionalPaymentIdVisitor {
        type Value = Option<PaymentIdType>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(PaymentIdVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<PaymentIdType, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(PaymentIdVisitor)
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize_option<'a, D>(
        deserializer: D,
    ) -> Result<Option<PaymentIdType>, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_option(OptionalPaymentIdVisitor)
    }
}

#[cfg(feature = "v1")]
mod payment_id_type {
    use std::{borrow::Cow, fmt};

    use serde::{
        de::{self, Visitor},
        Deserializer,
    };

    use super::PaymentIdType;

    struct PaymentIdVisitor;
    struct OptionalPaymentIdVisitor;

    impl Visitor<'_> for PaymentIdVisitor {
        type Value = PaymentIdType;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            common_utils::id_type::PaymentId::try_from(Cow::Owned(value.to_string()))
                .map_err(de::Error::custom)
                .map(PaymentIdType::PaymentIntentId)
        }
    }

    impl<'de> Visitor<'de> for OptionalPaymentIdVisitor {
        type Value = Option<PaymentIdType>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(PaymentIdVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<PaymentIdType, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(PaymentIdVisitor)
    }

    pub(crate) fn deserialize_option<'a, D>(
        deserializer: D,
    ) -> Result<Option<PaymentIdType>, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_option(OptionalPaymentIdVisitor)
    }
}

pub mod amount {
    use serde::de;

    use super::Amount;
    struct AmountVisitor;
    struct OptionalAmountVisitor;
    use crate::payments::MinorUnit;

    // This is defined to provide guarded deserialization of amount
    // which itself handles zero and non-zero values internally
    impl de::Visitor<'_> for AmountVisitor {
        type Value = Amount;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "amount as integer")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let v = i64::try_from(v).map_err(|_| {
                E::custom(format!(
                    "invalid value `{v}`, expected an integer between 0 and {}",
                    i64::MAX
                ))
            })?;
            self.visit_i64(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_negative() {
                return Err(E::custom(format!(
                    "invalid value `{v}`, expected a positive integer"
                )));
            }
            Ok(Amount::from(MinorUnit::new(v)))
        }
    }

    impl<'de> de::Visitor<'de> for OptionalAmountVisitor {
        type Value = Option<Amount>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "option of amount (as integer)")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_i64(AmountVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(AmountVisitor)
    }
    pub(crate) fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalAmountVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandate_type() {
        let mandate_type = MandateType::default();
        assert_eq!(
            serde_json::to_string(&mandate_type).unwrap(),
            r#"{"multi_use":null}"#
        )
    }
}

#[derive(Default, Debug, serde::Deserialize, Clone, ToSchema, serde::Serialize)]
pub struct RetrievePaymentLinkRequest {
    /// It's a token used for client side verification.
    pub client_secret: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkResponse {
    /// URL for rendering the open payment link
    pub link: String,
    /// URL for rendering the secure payment link
    pub secure_link: Option<String>,
    /// Identifier for the payment link
    pub payment_link_id: String,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct RetrievePaymentLinkResponse {
    /// Identifier for Payment Link
    pub payment_link_id: String,
    /// Identifier for Merchant
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// Open payment link (without any security checks and listing SPMs)
    pub link_to_pay: String,
    /// The payment amount. Amount for the payment in the lowest denomination of the currency
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// Date and time of Payment Link creation
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// Date and time of Expiration for Payment Link
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub expiry: Option<PrimitiveDateTime>,
    /// Description for Payment Link
    pub description: Option<String>,
    /// Status Of the Payment Link
    pub status: PaymentLinkStatus,
    #[schema(value_type = Option<Currency>)]
    pub currency: Option<api_enums::Currency>,
    /// Secure payment link (with security checks and listing saved payment methods)
    pub secure_link: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, ToSchema, serde::Serialize)]
pub struct PaymentLinkInitiateRequest {
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum PaymentLinkData {
    PaymentLinkDetails(Box<PaymentLinkDetails>),
    PaymentLinkStatusDetails(Box<PaymentLinkStatusDetails>),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PaymentLinkDetails {
    pub amount: StringMajorUnit,
    pub currency: api_enums::Currency,
    pub pub_key: String,
    pub client_secret: String,
    pub payment_id: id_type::PaymentId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: PrimitiveDateTime,
    pub merchant_logo: String,
    pub return_url: String,
    pub merchant_name: String,
    pub order_details: Option<Vec<OrderDetailsWithStringAmount>>,
    pub max_items_visible_after_collapse: i8,
    pub theme: String,
    pub merchant_description: Option<String>,
    pub sdk_layout: String,
    pub display_sdk_only: bool,
    pub hide_card_nickname_field: bool,
    pub show_card_form_by_default: bool,
    pub locale: Option<String>,
    pub transaction_details: Option<Vec<admin::PaymentLinkTransactionDetails>>,
    pub background_image: Option<admin::PaymentLinkBackgroundImageConfig>,
    pub details_layout: Option<api_enums::PaymentLinkDetailsLayout>,
    pub branding_visibility: Option<bool>,
    pub payment_button_text: Option<String>,
    pub skip_status_screen: Option<bool>,
    pub custom_message_for_card_terms: Option<String>,
    pub custom_message_for_payment_method_types:
        Option<common_enums::CustomTermsByPaymentMethodTypes>,
    pub payment_button_colour: Option<String>,
    pub payment_button_text_colour: Option<String>,
    pub background_colour: Option<String>,
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub status: api_enums::IntentStatus,
    pub enable_button_only_on_form_ready: bool,
    pub payment_form_header_text: Option<String>,
    pub payment_form_label_type: Option<api_enums::PaymentLinkSdkLabelType>,
    pub show_card_terms: Option<api_enums::PaymentLinkShowSdkTerms>,
    pub is_setup_mandate_flow: Option<bool>,
    pub capture_method: Option<common_enums::CaptureMethod>,
    pub setup_future_usage_applied: Option<common_enums::FutureUsage>,
    pub color_icon_card_cvc_error: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct SecurePaymentLinkDetails {
    pub enabled_saved_payment_method: bool,
    pub hide_card_nickname_field: bool,
    pub show_card_form_by_default: bool,
    #[serde(flatten)]
    pub payment_link_details: PaymentLinkDetails,
    pub payment_button_text: Option<String>,
    pub skip_status_screen: Option<bool>,
    pub custom_message_for_card_terms: Option<String>,
    pub custom_message_for_payment_method_types:
        Option<common_enums::CustomTermsByPaymentMethodTypes>,
    pub payment_button_colour: Option<String>,
    pub payment_button_text_colour: Option<String>,
    pub background_colour: Option<String>,
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub enable_button_only_on_form_ready: bool,
    pub payment_form_header_text: Option<String>,
    pub payment_form_label_type: Option<api_enums::PaymentLinkSdkLabelType>,
    pub show_card_terms: Option<api_enums::PaymentLinkShowSdkTerms>,
    pub color_icon_card_cvc_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentLinkStatusDetails {
    pub amount: StringMajorUnit,
    pub currency: api_enums::Currency,
    pub payment_id: id_type::PaymentId,
    pub merchant_logo: String,
    pub merchant_name: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub status: PaymentLinkStatusWrap,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub redirect: bool,
    pub theme: String,
    pub return_url: String,
    pub locale: Option<String>,
    pub transaction_details: Option<Vec<admin::PaymentLinkTransactionDetails>>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub capture_method: Option<common_enums::CaptureMethod>,
    pub setup_future_usage_applied: Option<common_enums::FutureUsage>,
}

#[derive(Clone, Debug, serde::Deserialize, ToSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLinkListConstraints {
    /// limit on the number of objects to return
    pub limit: Option<i64>,

    /// The time at which payment link is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// Time less than the payment link created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lt"
    )]
    pub created_lt: Option<PrimitiveDateTime>,

    /// Time greater than the payment link created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.gt"
    )]
    pub created_gt: Option<PrimitiveDateTime>,

    /// Time less than or equals to the payment link created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lte"
    )]
    pub created_lte: Option<PrimitiveDateTime>,

    /// Time greater than or equals to the payment link created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gte")]
    pub created_gte: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentLinkListResponse {
    /// The number of payment links included in the list
    pub size: usize,
    // The list of payment link response objects
    pub data: Vec<PaymentLinkResponse>,
}

/// Configure a custom payment link for the particular payment
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentCreatePaymentLinkConfig {
    #[serde(flatten)]
    #[schema(value_type = Option<PaymentLinkConfigRequest>)]
    /// Theme config for the particular payment
    pub theme_config: admin::PaymentLinkConfigRequest,
}

#[derive(Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct OrderDetailsWithStringAmount {
    /// Name of the product that is being purchased
    #[schema(max_length = 255, example = "shirt")]
    pub product_name: String,
    /// The quantity of the product to be purchased
    #[schema(example = 1)]
    pub quantity: u16,
    /// the amount per quantity of product
    pub amount: StringMajorUnit,
    /// Product Image link
    pub product_img_link: Option<String>,
}

/// Status Of the Payment Link
#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentLinkStatus {
    Active,
    Expired,
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum PaymentLinkStatusWrap {
    PaymentLinkStatus(PaymentLinkStatus),
    IntentStatus(api_enums::IntentStatus),
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct ExtendedCardInfoResponse {
    // Encrypted customer payment method data
    pub payload: String,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ClickToPaySessionResponse {
    #[smithy(value_type = "String")]
    pub dpa_id: String,
    #[smithy(value_type = "String")]
    pub dpa_name: String,
    #[smithy(value_type = "String")]
    pub locale: String,
    #[schema(value_type = Vec<CardNetwork>, example = "[Visa, Mastercard]")]
    #[smithy(value_type = "Vec<CardNetwork>")]
    pub card_brands: HashSet<api_enums::CardNetwork>,
    #[smithy(value_type = "String")]
    pub acquirer_bin: String,
    #[smithy(value_type = "String")]
    pub acquirer_merchant_id: String,
    #[smithy(value_type = "String")]
    pub merchant_category_code: String,
    #[smithy(value_type = "String")]
    pub merchant_country_code: String,
    #[schema(value_type = String, example = "38.02")]
    #[smithy(value_type = "String")]
    pub transaction_amount: StringMajorUnit,
    #[schema(value_type = Currency)]
    #[smithy(value_type = "Currency")]
    pub transaction_currency_code: common_enums::Currency,
    #[schema(value_type = Option<String>, max_length = 255, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub phone_number: Option<Secret<String>>,
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    #[smithy(value_type = "Option<String>")]
    pub email: Option<Email>,
    #[smithy(value_type = "Option<String>")]
    pub phone_country_code: Option<String>,
    /// provider Eg: Visa, Mastercard
    #[schema(value_type = Option<CtpServiceProvider>)]
    #[smithy(value_type = "Option<CtpServiceProvider>")]
    pub provider: Option<api_enums::CtpServiceProvider>,
    #[smithy(value_type = "Option<String>")]
    pub dpa_client_id: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, Clone, ToSchema)]
pub struct PaymentsEligibilityRequest {
    /// The identifier for the payment
    /// Added in the payload for ApiEventMetrics, populated from the path param
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// Token used for client side verification
    #[schema(value_type = String, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<Secret<String>>,
    /// The payment method to be used for the payment
    #[schema(value_type = PaymentMethod, example = "wallet")]
    pub payment_method_type: api_enums::PaymentMethod,
    /// The payment method type to be used for the payment
    #[schema(value_type = Option<PaymentMethodType>)]
    pub payment_method_subtype: Option<api_enums::PaymentMethodType>,
    /// The payment instrument data to be used for the payment
    pub payment_method_data: PaymentMethodDataRequest,
    /// The browser information for the payment
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<BrowserInformation>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsEligibilityResponse {
    /// The identifier for the payment
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
    /// Next action to be performed by the SDK
    pub sdk_next_action: SdkNextAction,
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod payments_request_api_contract {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_successful_card_deser() {
        let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method": "card",
            "payment_method_data": {
                "card": {
                    "card_number": "4242424242424242",
                    "card_exp_month": "10",
                    "card_exp_year": "25",
                    "card_holder_name": "joseph Doe",
                    "card_cvc": "123"
                }
            }
        }
        "#;

        let expected_card_number_string = "4242424242424242";
        let expected_card_number = CardNumber::from_str(expected_card_number_string).unwrap();

        let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
        assert!(payments_request.is_ok());

        if let Some(PaymentMethodData::Card(card_data)) = payments_request
            .unwrap()
            .payment_method_data
            .unwrap()
            .payment_method_data
        {
            assert_eq!(card_data.card_number, expected_card_number);
        } else {
            panic!("Received unexpected response")
        }
    }

    #[test]
    fn test_successful_payment_method_reward() {
        let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method": "reward",
            "payment_method_data": "reward",
            "payment_method_type": "evoucher"
        }
        "#;

        let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
        assert!(payments_request.is_ok());
        assert_eq!(
            payments_request
                .unwrap()
                .payment_method_data
                .unwrap()
                .payment_method_data,
            Some(PaymentMethodData::Reward)
        );
    }

    #[test]
    fn test_payment_method_data_with_payment_method_billing() {
        let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method_data": {
                "billing": {
                    "address": {
                        "line1": "1467",
                        "line2": "Harrison Street",
                        "city": "San Fransico",
                        "state": "California",
                        "zip": "94122",
                        "country": "US",
                        "first_name": "Narayan",
                        "last_name": "Bhat"
                    }
                }
            }
        }
        "#;

        let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
        assert!(payments_request.is_ok());
        assert!(payments_request
            .unwrap()
            .payment_method_data
            .unwrap()
            .billing
            .is_some());
    }
}

#[cfg(test)]
mod payments_response_api_contract {
    use super::*;

    #[derive(Debug, serde::Serialize)]
    struct TestPaymentsResponse {
        #[serde(serialize_with = "serialize_payment_method_data_response")]
        payment_method_data: Option<PaymentMethodDataResponseWithBilling>,
    }

    #[test]
    fn test_reward_payment_response() {
        let payment_method_response_with_billing = PaymentMethodDataResponseWithBilling {
            payment_method_data: Some(PaymentMethodDataResponse::Reward {}),
            billing: None,
        };

        let payments_response = TestPaymentsResponse {
            payment_method_data: Some(payment_method_response_with_billing),
        };

        let expected_response = r#"{"payment_method_data":"reward"}"#;

        let stringified_payments_response = payments_response.encode_to_string_of_json();
        assert_eq!(stringified_payments_response.unwrap(), expected_response);
    }
}

/// Set of tests to extract billing details from payment method data
/// These are required for backwards compatibility
#[cfg(test)]
mod billing_from_payment_method_data {
    use common_enums::CountryAlpha2;
    use masking::ExposeOptionInterface;

    use super::*;

    const TEST_COUNTRY: CountryAlpha2 = CountryAlpha2::US;
    const TEST_FIRST_NAME: &str = "John";
    const TEST_LAST_NAME: &str = "Wheat Dough";
    const TEST_FULL_NAME: &str = "John Wheat Dough";
    const TEST_FIRST_NAME_SINGLE: &str = "John";

    #[test]
    fn test_wallet_payment_method_data_paypal() {
        let test_email: Email = Email::try_from("example@example.com".to_string()).unwrap();

        let paypal_wallet_payment_method_data =
            PaymentMethodData::Wallet(WalletData::PaypalRedirect(PaypalRedirection {
                email: Some(test_email.clone()),
            }));

        let billing_address = paypal_wallet_payment_method_data
            .get_billing_address()
            .unwrap();

        assert_eq!(billing_address.email.unwrap(), test_email);

        assert!(billing_address.address.is_none());
        assert!(billing_address.phone.is_none());
    }

    #[test]
    fn test_bank_redirect_payment_method_data_eps() {
        let test_email = Email::try_from("example@example.com".to_string()).unwrap();
        let test_first_name = Secret::new(String::from("Chaser"));

        let bank_redirect_billing = BankRedirectBilling {
            billing_name: Some(test_first_name.clone()),
            email: Some(test_email.clone()),
        };

        let eps_bank_redirect_payment_method_data =
            PaymentMethodData::BankRedirect(BankRedirectData::Eps {
                billing_details: Some(bank_redirect_billing),
                bank_name: None,
                country: Some(TEST_COUNTRY),
            });

        let billing_address = eps_bank_redirect_payment_method_data
            .get_billing_address()
            .unwrap();

        let address_details = billing_address.address.unwrap();

        assert_eq!(billing_address.email.unwrap(), test_email);
        assert_eq!(address_details.country.unwrap(), TEST_COUNTRY);
        assert_eq!(address_details.first_name.unwrap(), test_first_name);
        assert!(billing_address.phone.is_none());
    }

    #[test]
    fn test_paylater_payment_method_data_klarna() {
        let test_email: Email = Email::try_from("example@example.com".to_string()).unwrap();

        let klarna_paylater_payment_method_data =
            PaymentMethodData::PayLater(PayLaterData::KlarnaRedirect {
                billing_email: Some(test_email.clone()),
                billing_country: Some(TEST_COUNTRY),
            });

        let billing_address = klarna_paylater_payment_method_data
            .get_billing_address()
            .unwrap();

        assert_eq!(billing_address.email.unwrap(), test_email);
        assert_eq!(
            billing_address.address.unwrap().country.unwrap(),
            TEST_COUNTRY
        );
        assert!(billing_address.phone.is_none());
    }

    #[test]
    fn test_bank_debit_payment_method_data_ach() {
        let test_email = Email::try_from("example@example.com".to_string()).unwrap();
        let test_first_name = Secret::new(String::from("Chaser"));

        let bank_redirect_billing = BankDebitBilling {
            name: Some(test_first_name.clone()),
            address: None,
            email: Some(test_email.clone()),
        };

        let ach_bank_debit_payment_method_data =
            PaymentMethodData::BankDebit(BankDebitData::AchBankDebit {
                billing_details: Some(bank_redirect_billing),
                account_number: Secret::new("1234".to_string()),
                routing_number: Secret::new("1235".to_string()),
                card_holder_name: None,
                bank_account_holder_name: None,
                bank_name: None,
                bank_type: None,
                bank_holder_type: None,
            });

        let billing_address = ach_bank_debit_payment_method_data
            .get_billing_address()
            .unwrap();

        let address_details = billing_address.address.unwrap();

        assert_eq!(billing_address.email.unwrap(), test_email);
        assert_eq!(address_details.first_name.unwrap(), test_first_name);
        assert!(billing_address.phone.is_none());
    }

    #[test]
    fn test_card_payment_method_data() {
        let card_payment_method_data = PaymentMethodData::Card(Card {
            card_holder_name: Some(Secret::new(TEST_FIRST_NAME_SINGLE.into())),
            ..Default::default()
        });

        let billing_address = card_payment_method_data.get_billing_address();

        let billing_address = billing_address.unwrap();

        assert_eq!(
            billing_address.address.unwrap().first_name.expose_option(),
            Some(TEST_FIRST_NAME_SINGLE.into())
        );
    }

    #[test]
    fn test_card_payment_method_data_empty() {
        let card_payment_method_data = PaymentMethodData::Card(Card::default());

        let billing_address = card_payment_method_data.get_billing_address();

        assert!(billing_address.is_none());
    }

    #[test]
    fn test_card_payment_method_data_full_name() {
        let card_payment_method_data = PaymentMethodData::Card(Card {
            card_holder_name: Some(Secret::new(TEST_FULL_NAME.into())),
            ..Default::default()
        });

        let billing_details = card_payment_method_data.get_billing_address().unwrap();
        let billing_address = billing_details.address.unwrap();

        assert_eq!(
            billing_address.first_name.expose_option(),
            Some(TEST_FIRST_NAME.into())
        );

        assert_eq!(
            billing_address.last_name.expose_option(),
            Some(TEST_LAST_NAME.into())
        );
    }

    #[test]
    fn test_card_payment_method_data_empty_string() {
        let card_payment_method_data = PaymentMethodData::Card(Card {
            card_holder_name: Some(Secret::new("".to_string())),
            ..Default::default()
        });

        let billing_details = card_payment_method_data.get_billing_address();

        assert!(billing_details.is_none());
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentRevenueRecoveryMetadata {
    /// Total number of billing connector + recovery retries for a payment intent.
    #[schema(value_type = u16,example = "1")]
    pub total_retry_count: u16,
    /// Flag for the payment connector's call
    pub payment_connector_transmission: Option<PaymentConnectorTransmission>,
    /// Billing Connector Id to update the invoices
    #[schema(value_type = String, example = "mca_1234567890")]
    pub billing_connector_id: id_type::MerchantConnectorAccountId,
    /// Payment Connector Id to retry the payments
    #[schema(value_type = String, example = "mca_1234567890")]
    pub active_attempt_payment_connector_id: id_type::MerchantConnectorAccountId,
    /// Billing Connector Payment Details
    #[schema(value_type = BillingConnectorPaymentDetails)]
    pub billing_connector_payment_details: BillingConnectorPaymentDetails,
    /// Payment Method Type
    #[schema(example = "pay_later", value_type = PaymentMethod)]
    pub payment_method_type: common_enums::PaymentMethod,
    /// PaymentMethod Subtype
    #[schema(example = "klarna", value_type = PaymentMethodType)]
    pub payment_method_subtype: common_enums::PaymentMethodType,
    /// The name of the payment connector through which the payment attempt was made.
    #[schema(value_type = Connector, example = "stripe")]
    pub connector: common_enums::connector_enums::Connector,
    #[schema(value_type = BillingConnectorPaymentMethodDetails)]
    /// Extra Payment Method Details that are needed to be stored
    pub billing_connector_payment_method_details: Option<BillingConnectorPaymentMethodDetails>,
    /// Invoice Next billing time
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub invoice_next_billing_time: Option<PrimitiveDateTime>,
    /// Invoice Next billing time
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub invoice_billing_started_at_time: Option<PrimitiveDateTime>,
    /// First Payment Attempt Payment Gateway Error Code
    #[schema(value_type = Option<String>, example = "card_declined")]
    pub first_payment_attempt_pg_error_code: Option<String>,
    /// First Payment Attempt Network Error Code
    #[schema(value_type = Option<String>, example = "05")]
    pub first_payment_attempt_network_decline_code: Option<String>,
    /// First Payment Attempt Network Advice Code
    #[schema(value_type = Option<String>, example = "02")]
    pub first_payment_attempt_network_advice_code: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum BillingConnectorPaymentMethodDetails {
    Card(BillingConnectorAdditionalCardInfo),
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct BillingConnectorAdditionalCardInfo {
    #[schema(value_type = CardNetwork, example = "Visa")]
    /// Card Network
    pub card_network: Option<common_enums::enums::CardNetwork>,
    #[schema(value_type = Option<String>, example = "JP MORGAN CHASE")]
    /// Card Issuer
    pub card_issuer: Option<String>,
}

#[cfg(feature = "v2")]
impl BillingConnectorPaymentMethodDetails {
    pub fn get_billing_connector_card_info(&self) -> Option<&BillingConnectorAdditionalCardInfo> {
        match self {
            Self::Card(card_details) => Some(card_details),
        }
    }
}

#[cfg(feature = "v2")]
impl PaymentRevenueRecoveryMetadata {
    pub fn set_payment_transmission_field_for_api_request(
        &mut self,
        payment_connector_transmission: PaymentConnectorTransmission,
    ) {
        self.payment_connector_transmission = Some(payment_connector_transmission);
    }
    pub fn get_payment_token_for_api_request(&self) -> mandates::ProcessorPaymentToken {
        mandates::ProcessorPaymentToken {
            processor_payment_token: self
                .billing_connector_payment_details
                .payment_processor_token
                .clone(),
            merchant_connector_id: Some(self.active_attempt_payment_connector_id.clone()),
        }
    }
    pub fn get_merchant_connector_id_for_api_request(&self) -> id_type::MerchantConnectorAccountId {
        self.active_attempt_payment_connector_id.clone()
    }

    pub fn get_connector_customer_id(&self) -> String {
        self.billing_connector_payment_details
            .connector_customer_id
            .to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg(feature = "v2")]
pub struct BillingConnectorPaymentDetails {
    /// Payment Processor Token to process the Revenue Recovery Payment
    pub payment_processor_token: String,
    /// Billing Connector's Customer Id
    pub connector_customer_id: String,
}

// Serialize is required because the api event requires Serialize to be implemented
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct PaymentsAttemptRecordRequest {
    /// The amount details for the payment attempt.
    pub amount_details: PaymentAttemptAmountDetails,

    #[schema(value_type = AttemptStatus, example = "charged")]
    pub status: enums::AttemptStatus,

    /// The billing details of the payment attempt. This address will be used for invoicing.
    pub billing: Option<Address>,

    /// The shipping address for the payment attempt.
    pub shipping: Option<Address>,

    /// Error details provided by the billing processor.
    pub error: Option<RecordAttemptErrorDetails>,

    /// A description for the payment attempt.
    #[schema(example = "It's my first payment request", value_type = Option<String>)]
    pub description: Option<common_utils::types::Description>,

    /// A unique identifier for a payment provided by the connector.
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,

    /// The payment method type used for payment attempt.
    #[schema(value_type = PaymentMethod, example = "bank_transfer")]
    pub payment_method_type: api_enums::PaymentMethod,

    /// The name of the payment connector through which the payment attempt was made.
    #[schema(value_type = Option<Connector>, example = "stripe")]
    pub connector: Option<common_enums::connector_enums::Connector>,

    /// Billing connector id to update the invoices.
    #[schema(value_type = String, example = "mca_1234567890")]
    pub billing_connector_id: id_type::MerchantConnectorAccountId,

    /// Billing connector id to update the invoices.
    #[schema(value_type = String, example = "mca_1234567890")]
    pub payment_merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// The payment method subtype to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// The additional payment data to be used for the payment attempt.
    pub payment_method_data: Option<RecordAttemptPaymentMethodDataRequest>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    pub feature_metadata: Option<PaymentAttemptFeatureMetadata>,

    /// The time at which payment attempt was created.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub transaction_created_at: Option<PrimitiveDateTime>,

    /// payment method token at payment processor end.
    #[schema(value_type = String, example = "1234567890")]
    pub processor_payment_method_token: String,

    /// customer id at payment connector for which mandate is attached.
    #[schema(value_type = String, example = "cust_12345")]
    pub connector_customer_id: String,

    /// Number of attempts made for invoice
    #[schema(value_type = Option<u16>, example = 1)]
    pub retry_count: Option<u16>,

    /// Next Billing time of the Invoice
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub invoice_next_billing_time: Option<PrimitiveDateTime>,

    /// Next Billing time of the Invoice
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub invoice_billing_started_at_time: Option<PrimitiveDateTime>,

    /// source where the payment was triggered by
    #[schema(value_type = TriggeredBy, example = "internal" )]
    pub triggered_by: common_enums::TriggeredBy,

    #[schema(value_type = CardNetwork, example = "Visa" )]
    /// card_network
    pub card_network: Option<common_enums::CardNetwork>,

    #[schema(example = "Chase")]
    /// Card Issuer
    pub card_issuer: Option<String>,
}

// Serialize is required because the api event requires Serialize to be implemented
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "v2")]
pub struct RecoveryPaymentsCreate {
    /// The amount details for the payment
    pub amount_details: AmountDetails,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub merchant_reference_id: id_type::PaymentReferenceId,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// Billing connector id to update the invoices.
    #[schema(value_type = String, example = "mca_1234567890")]
    pub billing_merchant_connector_id: id_type::MerchantConnectorAccountId,

    /// Payments connector id to update the invoices.
    #[schema(value_type = String, example = "mca_1234567890")]
    pub payment_merchant_connector_id: id_type::MerchantConnectorAccountId,

    #[schema(value_type = AttemptStatus, example = "charged")]
    pub attempt_status: enums::AttemptStatus,

    /// The billing details of the payment attempt.
    pub billing: Option<Address>,

    /// The payment method subtype to be used for the payment. This should match with the `payment_method_data` provided
    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_sub_type: api_enums::PaymentMethodType,

    /// The time at which payment attempt was created.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub transaction_created_at: Option<PrimitiveDateTime>,

    /// Payment method type for the payment attempt
    #[schema(value_type = Option<PaymentMethod>, example = "wallet")]
    pub payment_method_type: common_enums::PaymentMethod,

    /// customer id at payment connector for which mandate is attached.
    #[schema(value_type = String, example = "cust_12345")]
    pub connector_customer_id: Secret<String>,

    /// Invoice billing started at billing connector end.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub billing_started_at: Option<PrimitiveDateTime>,

    /// A unique identifier for a payment provided by the payment connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<Secret<String>>,

    /// payment method token units at payment processor end.
    pub payment_method_data: CustomRecoveryPaymentMethodData,

    /// Type of action that needs to be taken after consuming the recovery payload. For example: scheduling a failed payment or stopping the invoice.
    pub action: common_payments_types::RecoveryAction,

    /// Allow partial authorization for this payment
    #[schema(value_type = Option<bool>, default = false)]
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

/// Error details for the payment
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct RecordAttemptErrorDetails {
    /// error code sent by billing connector.
    pub code: String,
    /// error message sent by billing connector.
    pub message: String,
    /// This field can be returned for both approved and refused Mastercard payments.
    /// This code provides additional information about the type of transaction or the reason why the payment failed.
    /// If the payment failed, the network advice code gives guidance on if and when you can retry the payment.
    pub network_advice_code: Option<String>,
    /// For card errors resulting from a card issuer decline, a brand specific 2, 3, or 4 digit code which indicates the reason the authorization failed.
    pub network_decline_code: Option<String>,
    /// A string indicating how to proceed with an network error if payment gateway provide one. This is used to understand the network error code better.
    pub network_error_message: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, ToSchema)]
pub struct NullObject;

impl Serialize for NullObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

#[cfg(test)]
mod null_object_test {
    use serde_json;

    use super::*;

    #[test]
    fn test_null_object_serialization() {
        let null_object = NullObject;
        let serialized = serde_json::to_string(&null_object).unwrap();
        assert_eq!(serialized, "null");
    }
}

/// Represents external 3DS authentication data used in the payment flow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ExternalThreeDsData {
    /// Contains the authentication cryptogram data (CAVV or TAVV).
    #[schema(value_type = Cryptogram)]
    pub authentication_cryptogram: Cryptogram,
    /// Directory Server Transaction ID generated during the 3DS process.
    #[schema(value_type = String)]
    pub ds_trans_id: String,
    /// The version of the 3DS protocol used (e.g., "2.1.0" or "2.2.0").
    #[schema(value_type = String)]
    pub version: SemanticVersion,
    /// Electronic Commerce Indicator (ECI) value representing the 3DS authentication result.
    #[schema(value_type = String)]
    pub eci: String,
    /// Indicates the transaction status from the 3DS authentication flow.
    #[schema(value_type = TransactionStatus)]
    pub transaction_status: common_enums::TransactionStatus,
    /// Optional exemption indicator specifying the exemption type, if any, used in this transaction.
    #[schema(value_type = Option<ExemptionIndicator>)]
    pub exemption_indicator: Option<common_enums::ExemptionIndicator>,
    /// Optional network-specific parameters that may be required by certain card networks.
    #[schema(value_type = Option<NetworkParams>)]
    pub network_params: Option<NetworkParams>,
}

/// Represents the 3DS cryptogram data returned after authentication.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cryptogram {
    /// Cardholder Authentication Verification Value (CAVV) cryptogram.
    Cavv {
        /// The authentication cryptogram provided by the issuer or ACS.
        #[schema(value_type = Option<String>)]
        authentication_cryptogram: Secret<String>,
    },
}

/// Represents additional network-level parameters for 3DS processing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct NetworkParams {
    /// Parameters specific to Cartes Bancaires network, if applicable.
    #[schema(value_type = Option<CartesBancairesParams>)]
    pub cartes_bancaires: Option<CartesBancairesParams>,
}

/// Represents network-specific parameters for the Cartes Bancaires 3DS process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CartesBancairesParams {
    /// The algorithm used to generate the CAVV value.
    #[schema(value_type = Option<CavvAlgorithm>)]
    pub cavv_algorithm: common_enums::CavvAlgorithm,
    /// Exemption indicator specific to Cartes Bancaires network (e.g., "low_value", "trusted_merchant")
    #[schema(value_type = String)]
    pub cb_exemption: String,
    /// Cartes Bancaires risk score assigned during 3DS authentication.
    #[schema(value_type = i32)]
    pub cb_score: i32,
}
