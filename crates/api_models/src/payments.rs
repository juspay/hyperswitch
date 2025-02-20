use std::{
    collections::{HashMap, HashSet},
    fmt,
    num::NonZeroI64,
};
pub mod additional_info;
pub mod trait_impls;
use cards::CardNumber;
#[cfg(feature = "v2")]
use common_enums::enums::PaymentConnectorTransmission;
use common_enums::ProductType;
#[cfg(feature = "v2")]
use common_utils::id_type::GlobalPaymentId;
use common_utils::{
    consts::default_payments_list_limit,
    crypto,
    errors::ValidationError,
    ext_traits::{ConfigExt, Encode, ValueExt},
    hashing::HashedString,
    id_type,
    pii::{self, Email},
    types::{
        ExtendedAuthorizationAppliedBool, MinorUnit, RequestExtendedAuthorizationBool,
        StringMajorUnit,
    },
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret, WithType};
use router_derive::Setter;
use serde::{de, ser::Serializer, Deserialize, Deserializer, Serialize};
use strum::Display;
use time::{Date, PrimitiveDateTime};
use url::Url;
use utoipa::ToSchema;

#[cfg(feature = "v1")]
use crate::ephemeral_key::EphemeralKeyCreateResponse;
#[cfg(feature = "v2")]
use crate::payment_methods;
use crate::{
    admin::{self, MerchantConnectorInfo},
    disputes, enums as api_enums,
    mandates::RecurringDetails,
    refunds, ValidateFieldAndGet,
};

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
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, PartialEq)]
pub struct CustomerDetails {
    /// The identifier for the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub id: id_type::CustomerId,

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

/// Details of customer attached to this payment
#[derive(
    Debug, Default, serde::Serialize, serde::Deserialize, Clone, ToSchema, PartialEq, Setter,
)]
pub struct CustomerDetailsResponse {
    /// The identifier for the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub id: Option<id_type::CustomerId>,

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
#[derive(Debug, Clone, serde::Serialize)]
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
    pub connector_metadata: Option<pii::SecretSerdeValue>,

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
    #[schema(value_type = String, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: common_utils::types::ClientSecret,

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
    pub connector_metadata: Option<pii::SecretSerdeValue>,

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

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_on: PrimitiveDateTime,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(value_type = External3dsAuthenticationRequest)]
    pub request_external_three_ds_authentication: common_enums::External3dsAuthenticationRequest,
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]

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
)]
#[generate_schemas(PaymentsCreateRequest, PaymentsUpdateRequest, PaymentsConfirmRequest)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRequest {
    /// The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥ is a zero-decimal currency. Read more about [the Decimal and Non-Decimal Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
    #[schema(value_type = Option<u64>, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize_option")]
    #[mandatory_in(PaymentsCreateRequest = u64)]
    // Makes the field mandatory in PaymentsCreateRequest
    pub amount: Option<Amount>,

    /// Total tax amount applicable to the order
    #[schema(value_type = Option<i64>, example = 6540)]
    pub order_tax_amount: Option<MinorUnit>,

    /// The three letter ISO currency code in uppercase. Eg: 'USD' to charge US Dollars
    #[schema(example = "USD", value_type = Option<Currency>)]
    #[mandatory_in(PaymentsCreateRequest = Currency)]
    pub currency: Option<api_enums::Currency>,

    /// The Amount to be captured / debited from the users payment method. It shall be in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, the default amount_to_capture will be the payment amount. Also, it must be less than or equal to the original payment account.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub amount_to_capture: Option<MinorUnit>,

    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub shipping_cost: Option<MinorUnit>,

    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant. The value for this field can be specified in the request, it will be auto generated otherwise and returned in the API response.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    #[serde(default, deserialize_with = "payment_id_type::deserialize_option")]
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
    pub routing: Option<serde_json::Value>,

    /// This allows to manually select a connector with which the payment can go through.
    #[schema(value_type = Option<Vec<Connector>>, max_length = 255, example = json!(["stripe", "adyen"]))]
    pub connector: Option<Vec<api_enums::Connector>>,

    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The billing details of the payment. This address will be used for invoicing.
    pub billing: Option<Address>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub capture_on: Option<PrimitiveDateTime>,

    /// Whether to confirm the payment (if applicable). It can be used to completely process a payment by attaching a payment method, setting `confirm=true` and `capture_method = automatic` in the *Payments/Create API* request itself.
    #[schema(default = false, example = true)]
    pub confirm: Option<bool>,

    /// Passing this object creates a new customer or attaches an existing customer to the payment
    pub customer: Option<CustomerDetails>,

    /// The identifier for the customer
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
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
    pub off_session: Option<bool>,

    /// A description for the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// The URL to which you want the user to be redirected after the completion of the payment operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<Url>,

    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    #[schema(example = "bank_transfer")]
    #[serde(with = "payment_method_data_serde", default)]
    pub payment_method_data: Option<PaymentMethodDataRequest>,

    #[schema(value_type = Option<PaymentMethod>, example = "card")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    #[schema(value_type = Option<String>, deprecated)]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub card_cvc: Option<Secret<String>>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 255, example = "Hyperswitch Router")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    #[schema(max_length = 255, example = "Payment for shoes purchase")]
    pub statement_descriptor_suffix: Option<String>,

    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "Apple iPhone 16",
        "quantity": 1,
        "amount" : 69000
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// It's a token used for client side verification.
    #[schema(example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest)]
    pub client_secret: Option<String>,

    /// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
    pub mandate_data: Option<MandateData>,

    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<CustomerAcceptance>,

    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    #[schema(max_length = 255, example = "mandate_iwer89rnjef349dni3")]
    #[remove_in(PaymentsUpdateRequest)]
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
    pub browser_info: Option<serde_json::Value>,

    /// To indicate the type of payment experience that the payment method would go through
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// Can be used to specify the Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// Business country of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    #[schema(example = "food")]
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    pub business_label: Option<String>,

    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,

    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Business sub label for the payment
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest, PaymentsCreateRequest)]
    pub business_sub_label: Option<String>,

    /// Denotes the retry action
    #[schema(value_type = Option<RetryAction>)]
    #[remove_in(PaymentsCreateRequest)]
    pub retry_action: Option<api_enums::RetryAction>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<serde_json::Value>,

    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    pub connector_metadata: Option<ConnectorMetadata>,

    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    #[remove_in(PaymentsUpdateRequest, PaymentsCreateRequest, PaymentsConfirmRequest)]
    pub feature_metadata: Option<FeatureMetadata>,

    /// Whether to generate the payment link for this payment or not (if applicable)
    #[schema(default = false, example = true)]
    pub payment_link: Option<bool>,

    #[schema(value_type = Option<PaymentCreatePaymentLinkConfig>)]
    pub payment_link_config: Option<PaymentCreatePaymentLinkConfig>,

    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    pub payment_link_config_id: Option<String>,

    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    #[remove_in(PaymentsUpdateRequest, PaymentsConfirmRequest)]
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    #[remove_in(PaymentsConfirmRequest)]
    #[schema(value_type = Option<RequestSurchargeDetails>)]
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// The type of the payment that differentiates between normal and various types of mandate payments
    #[schema(value_type = Option<PaymentType>)]
    pub payment_type: Option<api_enums::PaymentType>,

    ///Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    pub request_incremental_authorization: Option<bool>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Additional data related to some frm(Fraud Risk Management) connectors
    #[schema(value_type = Option<Object>, example = r#"{ "coverage_request" : "fraud", "fulfillment_method" : "delivery" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// Whether to perform external authentication (if applicable)
    #[schema(example = true)]
    pub request_external_three_ds_authentication: Option<bool>,

    /// Details required for recurring payment
    pub recurring_details: Option<RecurringDetails>,

    /// Fee information to be charged on the payment being collected
    #[schema(value_type = Option<SplitPaymentsRequest>)]
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,

    /// Optional boolean value to extent authorization period of this payment
    ///
    /// capture method must be manual or manual_multiple
    #[schema(value_type = Option<bool>, default = false)]
    pub request_extended_authorization: Option<RequestExtendedAuthorizationBool>,

    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    #[schema(
        value_type = Option<String>,
        max_length = 255,
        example = "Custom_Order_id_123"
    )]
    pub merchant_order_reference_id: Option<String>,

    /// Whether to calculate tax for this payment intent
    pub skip_external_tax_calculation: Option<bool>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<api_enums::ScaExemptionType>,

    /// Service details for click to pay external authentication
    #[schema(value_type = Option<CtpServiceDetails>)]
    pub ctp_service_details: Option<CtpServiceDetails>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CtpServiceDetails {
    /// merchant transaction id
    pub merchant_transaction_id: Option<String>,
    /// network transaction correlation id
    pub correlation_id: Option<String>,
    /// session transaction flow id
    pub x_src_flow_id: Option<String>,
    /// provider Eg: Visa, Mastercard
    pub provider: Option<String>,
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
    Default, Debug, Clone, serde::Serialize, serde::Deserialize, Copy, ToSchema, PartialEq,
)]
pub struct RequestSurchargeDetails {
    #[schema(value_type = i64, example = 6540)]
    pub surcharge_amount: MinorUnit,
    pub tax_amount: Option<MinorUnit>,
}

// for v2 use the type from common_utils::types
#[cfg(feature = "v1")]
/// Browser information to be used for 3DS 2.0
#[derive(ToSchema, Debug, serde::Deserialize, serde::Serialize)]
pub struct BrowserInformation {
    /// Color depth supported by the browser
    pub color_depth: Option<u8>,

    /// Whether java is enabled in the browser
    pub java_enabled: Option<bool>,

    /// Whether javascript is enabled in the browser
    pub java_script_enabled: Option<bool>,

    /// Language supported
    pub language: Option<String>,

    /// The screen height in pixels
    pub screen_height: Option<u32>,

    /// The screen width in pixels
    pub screen_width: Option<u32>,

    /// Time zone of the client
    pub time_zone: Option<i32>,

    /// Ip address of the client
    #[schema(value_type = Option<String>)]
    pub ip_address: Option<std::net::IpAddr>,

    /// List of headers that are accepted
    #[schema(
        example = "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
    )]
    pub accept_header: Option<String>,

    /// User-agent of the browser
    pub user_agent: Option<String>,

    /// The os type of the client device
    pub os_type: Option<String>,

    /// The os version of the client device
    pub os_version: Option<String>,

    /// The device model of the client
    pub device_model: Option<String>,
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
#[derive(Debug, serde::Serialize, Clone, PartialEq, ToSchema, router_derive::PolymorphicSchema)]
pub struct PaymentAttemptResponse {
    /// Unique identifier for the attempt
    pub attempt_id: String,
    /// The status of the attempt
    #[schema(value_type = AttemptStatus, example = "charged")]
    pub status: enums::AttemptStatus,
    /// The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// The payment attempt tax_amount.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub order_tax_amount: Option<MinorUnit>,
    /// The currency of the amount of the payment attempt
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<enums::Currency>,
    /// The connector used for the payment
    pub connector: Option<String>,
    /// If there was an error while calling the connector, the error message is received here
    pub error_message: Option<String>,
    /// The payment method that is to be used
    #[schema(value_type = Option<PaymentMethod>, example = "bank_transfer")]
    pub payment_method: Option<enums::PaymentMethod>,
    /// A unique identifier for a payment provided by the connector
    pub connector_transaction_id: Option<String>,
    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "scheduled")]
    pub capture_method: Option<enums::CaptureMethod>,
    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<enums::AuthenticationType>,
    /// Time at which the payment attempt was created
    #[schema(value_type = PrimitiveDateTime, example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// Time at which the payment attempt was last modified
    #[schema(value_type = PrimitiveDateTime, example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    /// If the payment was cancelled the reason will be provided here
    pub cancellation_reason: Option<String>,
    /// A unique identifier to link the payment to a mandate, can be use instead of payment_method_data
    pub mandate_id: Option<String>,
    /// If there was an error while calling the connectors the error code is received here
    pub error_code: Option<String>,
    /// Provide a reference to a stored payment method
    pub payment_token: Option<String>,
    /// Additional data related to some connectors
    pub connector_metadata: Option<serde_json::Value>,
    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<enums::PaymentExperience>,
    /// Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    pub payment_method_type: Option<enums::PaymentMethodType>,
    /// Reference to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub reference_id: Option<String>,
    /// (This field is not live yet)Error code unified across the connectors is received here if there was an error while calling connector
    pub unified_code: Option<String>,
    /// (This field is not live yet)Error message unified across the connectors is received here if there was an error while calling connector
    pub unified_message: Option<String>,
    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    pub client_source: Option<String>,
    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
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
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_payment_id: Option<String>,

    /// Identifier for Payment Method used for the payment attempt
    #[schema(value_type = Option<String>, example = "12345_pm_01926c58bc6e77c09e809964e72af8c8")]
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,

    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    pub client_source: Option<String>,
    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
    pub client_version: Option<String>,
}

#[derive(
    Default, Debug, serde::Serialize, Clone, PartialEq, ToSchema, router_derive::PolymorphicSchema,
)]
pub struct CaptureResponse {
    /// Unique identifier for the capture
    pub capture_id: String,
    /// The status of the capture
    #[schema(value_type = CaptureStatus, example = "charged")]
    pub status: enums::CaptureStatus,
    /// The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// The currency of the amount of the capture
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<enums::Currency>,
    /// The connector used for the payment
    pub connector: String,
    /// Unique identifier for the parent attempt on which this capture is made
    pub authorized_attempt_id: String,
    /// A unique identifier for this capture provided by the connector
    pub connector_capture_id: Option<String>,
    /// Sequence number of this capture, in the series of captures made for the parent attempt
    pub capture_sequence: i16,
    /// If there was an error while calling the connector the error message is received here
    pub error_message: Option<String>,
    /// If there was an error while calling the connectors the code is received here
    pub error_code: Option<String>,
    /// If there was an error while calling the connectors the reason is received here
    pub error_reason: Option<String>,
    /// Reference to the capture at connector side
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
}

impl ConnectorMandateReferenceId {
    pub fn new(
        connector_mandate_id: Option<String>,
        payment_method_id: Option<String>,
        update_history: Option<Vec<UpdateHistory>>,
        mandate_metadata: Option<pii::SecretSerdeValue>,
        connector_mandate_request_reference_id: Option<String>,
    ) -> Self {
        Self {
            connector_mandate_id,
            payment_method_id,
            update_history,
            mandate_metadata,
            connector_mandate_request_reference_id,
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
#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MandateData {
    /// A way to update the mandate's payment method details
    pub update_mandate_id: Option<String>,
    /// A consent from the customer to store the payment method
    pub customer_acceptance: Option<CustomerAcceptance>,
    /// A way to select the type of mandate used
    pub mandate_type: Option<MandateType>,
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SingleUseMandate {
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, ToSchema, serde::Serialize, serde::Deserialize)]
pub struct MandateAmountData {
    /// The maximum amount to be debited for the mandate transaction
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    /// The currency for the transaction
    #[schema(value_type = Currency, example = "USD")]
    pub currency: api_enums::Currency,
    /// Specifying start date of the mandate
    #[schema(example = "2022-09-10T00:00:00Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_date: Option<PrimitiveDateTime>,
    /// Specifying end date of the mandate
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_date: Option<PrimitiveDateTime>,
    /// Additional details required by mandate
    #[schema(value_type = Option<Object>, example = r#"{
        "frequency": "DAILY"
    }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MandateType {
    /// If the mandate should only be valid for 1 off-session use
    SingleUse(MandateAmountData),
    /// If the mandate should be valid for multiple debits
    MultiUse(Option<MandateAmountData>),
}

impl Default for MandateType {
    fn default() -> Self {
        Self::MultiUse(None)
    }
}

/// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomerAcceptance {
    /// Type of acceptance provided by the
    #[schema(example = "online")]
    pub acceptance_type: AcceptanceType,
    /// Specifying when the customer acceptance was provided
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<PrimitiveDateTime>,
    /// Information required for online mandate generation
    pub online: Option<OnlineMandate>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
/// This is used to indicate if the mandate was accepted online or offline
pub enum AcceptanceType {
    Online,
    #[default]
    Offline,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    #[schema(value_type = String, example = "123.32.25.123")]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    /// The user-agent of the customer's browser
    pub user_agent: String,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Card {
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
    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    pub nick_name: Option<Secret<String>>,
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

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct CardToken {
    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = Option<String>)]
    pub card_cvc: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CardRedirectData {
    Knet {},
    Benefit {},
    MomoAtm {},
    CardRedirect {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayLaterData {
    /// For KlarnaRedirect as PayLater Option
    KlarnaRedirect {
        /// The billing email
        #[schema(value_type = Option<String>)]
        billing_email: Option<Email>,
        // The billing country code
        #[schema(value_type = Option<CountryAlpha2>, example = "US")]
        billing_country: Option<api_enums::CountryAlpha2>,
    },
    /// For Klarna Sdk as PayLater Option
    KlarnaSdk {
        /// The token for the sdk workflow
        token: String,
    },
    /// For Affirm redirect as PayLater Option
    AffirmRedirect {},
    /// For AfterpayClearpay redirect as PayLater Option
    AfterpayClearpayRedirect {
        /// The billing email
        #[schema(value_type = Option<String>)]
        billing_email: Option<Email>,
        /// The billing name
        #[schema(value_type = Option<String>)]
        billing_name: Option<Secret<String>>,
    },
    /// For PayBright Redirect as PayLater Option
    PayBrightRedirect {},
    /// For WalleyRedirect as PayLater Option
    WalleyRedirect {},
    /// For Alma Redirection as PayLater Option
    AlmaRedirect {},
    AtomeRedirect {},
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
            | Self::WalleyRedirect {}
            | Self::AlmaRedirect {}
            | Self::KlarnaSdk { .. }
            | Self::AffirmRedirect {}
            | Self::AtomeRedirect {} => None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitData {
    /// Payment Method data for Ach bank debit
    AchBankDebit {
        /// Billing details for bank debit
        billing_details: Option<BankDebitBilling>,
        /// Account number for ach bank debit payment
        #[schema(value_type = String, example = "000123456789")]
        account_number: Secret<String>,
        /// Routing number for ach bank debit payment
        #[schema(value_type = String, example = "110000000")]
        routing_number: Secret<String>,

        #[schema(value_type = String, example = "John Test")]
        card_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "John Doe")]
        bank_account_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "ACH")]
        bank_name: Option<common_enums::BankNames>,

        #[schema(value_type = String, example = "Checking")]
        bank_type: Option<common_enums::BankType>,

        #[schema(value_type = String, example = "Personal")]
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
    SepaBankDebit {
        /// Billing details for bank debit
        billing_details: Option<BankDebitBilling>,
        /// International bank account number (iban) for SEPA
        #[schema(value_type = String, example = "DE89370400440532013000")]
        iban: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    BecsBankDebit {
        /// Billing details for bank debit
        billing_details: Option<BankDebitBilling>,
        /// Account number for Becs payment method
        #[schema(value_type = String, example = "000123456")]
        account_number: Secret<String>,
        /// Bank-State-Branch (bsb) number
        #[schema(value_type = String, example = "000000")]
        bsb_number: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = Option<String>, example = "A. Schneider")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    BacsBankDebit {
        /// Billing details for bank debit
        billing_details: Option<BankDebitBilling>,
        /// Account number for Bacs payment method
        #[schema(value_type = String, example = "00012345")]
        account_number: Secret<String>,
        /// Sort code for Bacs payment method
        #[schema(value_type = String, example = "108800")]
        sort_code: Secret<String>,
        /// holder name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
pub struct PaymentMethodDataRequest {
    /// This field is optional because, in case of saved cards we pass the payment_token
    /// There might be cases where we don't need to pass the payment_method_data and pass only payment method billing details
    /// We have flattened it because to maintain backwards compatibility with the old API contract
    #[serde(flatten)]
    pub payment_method_data: Option<PaymentMethodData>,
    /// billing details for the payment method.
    /// This billing details will be passed to the processor as billing address.
    /// If not passed, then payment.billing will be considered
    pub billing: Option<Address>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodData {
    #[schema(title = "Card")]
    Card(Card),
    #[schema(title = "CardRedirect")]
    CardRedirect(CardRedirectData),
    #[schema(title = "Wallet")]
    Wallet(WalletData),
    #[schema(title = "PayLater")]
    PayLater(PayLaterData),
    #[schema(title = "BankRedirect")]
    BankRedirect(BankRedirectData),
    #[schema(title = "BankDebit")]
    BankDebit(BankDebitData),
    #[schema(title = "BankTransfer")]
    BankTransfer(Box<BankTransferData>),
    #[schema(title = "RealTimePayment")]
    RealTimePayment(Box<RealTimePaymentData>),
    #[schema(title = "Crypto")]
    Crypto(CryptoData),
    #[schema(title = "MandatePayment")]
    MandatePayment,
    #[schema(title = "Reward")]
    Reward,
    #[schema(title = "Upi")]
    Upi(UpiData),
    #[schema(title = "Voucher")]
    Voucher(VoucherData),
    #[schema(title = "GiftCard")]
    GiftCard(Box<GiftCardData>),
    #[schema(title = "CardToken")]
    CardToken(CardToken),
    #[schema(title = "OpenBanking")]
    OpenBanking(OpenBankingData),
    #[schema(title = "MobilePayment")]
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
            Self::AliPayQr(_) | Self::AliPayRedirect(_) => api_enums::PaymentMethodType::AliPay,
            Self::AliPayHkRedirect(_) => api_enums::PaymentMethodType::AliPayHk,
            Self::AmazonPayRedirect(_) => api_enums::PaymentMethodType::AmazonPay,
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
            Self::WalleyRedirect {} => api_enums::PaymentMethodType::Walley,
            Self::AlmaRedirect {} => api_enums::PaymentMethodType::Alma,
            Self::AtomeRedirect {} => api_enums::PaymentMethodType::Atome,
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
        }
    }
}

impl GetPaymentMethodType for BankTransferData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankTransfer { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankTransfer { .. } => api_enums::PaymentMethodType::Sepa,
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
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardData {
    Givex(GiftCardDetails),
    PaySafeCard {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GiftCardDetails {
    /// The gift card number
    #[schema(value_type = String)]
    pub number: Secret<String>,
    /// The card verification code.
    #[schema(value_type = String)]
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
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalPaymentData {
    Card(Box<AdditionalCardInfo>),
    BankRedirect {
        bank_name: Option<common_enums::BankNames>,
        #[serde(flatten)]
        details: Option<additional_info::BankRedirectDetails>,
    },
    Wallet {
        apple_pay: Option<ApplepayPaymentMethod>,
        google_pay: Option<additional_info::WalletAdditionalDataForCard>,
        samsung_pay: Option<additional_info::WalletAdditionalDataForCard>,
    },
    PayLater {
        klarna_sdk: Option<KlarnaSdkPaymentMethod>,
    },
    BankTransfer {
        #[serde(flatten)]
        details: Option<additional_info::BankTransferAdditionalData>,
    },
    Crypto {
        #[serde(flatten)]
        details: Option<CryptoData>,
    },
    BankDebit {
        #[serde(flatten)]
        details: Option<additional_info::BankDebitAdditionalData>,
    },
    MandatePayment {},
    Reward {},
    RealTimePayment {
        #[serde(flatten)]
        details: Option<RealTimePaymentData>,
    },
    Upi {
        #[serde(flatten)]
        details: Option<additional_info::UpiAdditionalData>,
    },
    GiftCard {
        #[serde(flatten)]
        details: Option<additional_info::GiftCardAdditionalData>,
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
        details: Option<additional_info::CardTokenAdditionalData>,
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct KlarnaSdkPaymentMethod {
    pub payment_type: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankRedirectData {
    BancontactCard {
        /// The card number
        #[schema(value_type = String, example = "4242424242424242")]
        card_number: Option<CardNumber>,
        /// The card's expiry month
        #[schema(value_type = String, example = "24")]
        card_exp_month: Option<Secret<String>>,

        /// The card's expiry year
        #[schema(value_type = String, example = "24")]
        card_exp_year: Option<Secret<String>>,

        /// The card holder's name
        #[schema(value_type = String, example = "John Test")]
        card_holder_name: Option<Secret<String>>,

        //Required by Stripes
        billing_details: Option<BankRedirectBilling>,
    },
    Bizum {},
    Blik {
        // Blik Code
        blik_code: Option<String>,
    },
    Eps {
        /// The billing details for bank redirection
        billing_details: Option<BankRedirectBilling>,

        /// The hyperswitch bank code for eps
        #[schema(value_type = BankNames, example = "triodos_bank")]
        bank_name: Option<common_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Giropay {
        /// The billing details for bank redirection
        billing_details: Option<BankRedirectBilling>,

        #[schema(value_type = Option<String>)]
        /// Bank account bic code
        bank_account_bic: Option<Secret<String>>,

        /// Bank account iban
        #[schema(value_type = Option<String>)]
        bank_account_iban: Option<Secret<String>>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Ideal {
        /// The billing details for bank redirection
        billing_details: Option<BankRedirectBilling>,

        /// The hyperswitch bank code for ideal
        #[schema(value_type = BankNames, example = "abn_amro")]
        bank_name: Option<common_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Interac {
        /// The country for bank payment
        #[schema(value_type = Option<CountryAlpha2>, example = "US")]
        country: Option<api_enums::CountryAlpha2>,

        #[schema(value_type = Option<String>, example = "john.doe@example.com")]
        email: Option<Email>,
    },
    OnlineBankingCzechRepublic {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: common_enums::BankNames,
    },
    OnlineBankingFinland {
        // Shopper Email
        #[schema(value_type = Option<String>)]
        email: Option<Email>,
    },
    OnlineBankingPoland {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: common_enums::BankNames,
    },
    OnlineBankingSlovakia {
        // Issuer value corresponds to the bank
        #[schema(value_type = BankNames)]
        issuer: common_enums::BankNames,
    },
    OpenBankingUk {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: Option<common_enums::BankNames>,
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Przelewy24 {
        //Issuer banks
        #[schema(value_type = Option<BankNames>)]
        bank_name: Option<common_enums::BankNames>,

        // The billing details for bank redirect
        billing_details: Option<BankRedirectBilling>,
    },
    Sofort {
        /// The billing details for bank redirection
        billing_details: Option<BankRedirectBilling>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,

        /// The preferred language
        #[schema(example = "en")]
        preferred_language: Option<String>,
    },
    Trustly {
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,
    },
    OnlineBankingFpx {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: common_enums::BankNames,
    },
    OnlineBankingThailand {
        #[schema(value_type = BankNames)]
        issuer: common_enums::BankNames,
    },
    LocalBankRedirect {},
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
            | Self::Blik { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AlfamartVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = Option<String>, example = "Jane")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Alfamart
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct IndomaretVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = Option<String>, example = "Jane")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Alfamart
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "Jane")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name Japanese convenience stores
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
    /// The telephone number for Japanese convenience stores
    #[schema(value_type = Option<String>, example = "9123456789")]
    pub phone_number: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AchBillingDetails {
    /// The Email ID for ACH billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct DokuBillingDetails {
    /// The billing first name for Doku
    #[schema(value_type = Option<String>, example = "Jane")]
    pub first_name: Option<Secret<String>>,
    /// The billing second name for Doku
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Doku billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MultibancoBillingDetails {
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    #[schema(value_type = Option<String>, example = "example@me.com")]
    pub email: Option<Email>,
    /// The billing name for SEPA and BACS billing
    #[schema(value_type = Option<String>, example = "Jane Doe")]
    pub name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpiData {
    UpiCollect(UpiCollectData),
    UpiIntent(UpiIntentData),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpiCollectData {
    #[schema(value_type = Option<String>, example = "successtest@iata")]
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct UpiIntentData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SofortBilling {
    /// The country associated with the billing
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub billing_country: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BankRedirectBilling {
    /// The name for which billing is issued
    #[schema(value_type = String, example = "John Doe")]
    pub billing_name: Option<Secret<String>>,
    /// The billing email for bank redirect
    #[schema(value_type = String, example = "example@example.com")]
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferData {
    AchBankTransfer {
        /// The billing details for ACH Bank Transfer
        billing_details: Option<AchBillingDetails>,
    },
    SepaBankTransfer {
        /// The billing details for SEPA
        billing_details: Option<SepaAndBacsBillingDetails>,

        /// The two-letter ISO country code for SEPA and BACS
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    BacsBankTransfer {
        /// The billing details for SEPA
        billing_details: Option<SepaAndBacsBillingDetails>,
    },
    MultibancoBankTransfer {
        /// The billing details for Multibanco
        billing_details: Option<MultibancoBillingDetails>,
    },
    PermataBankTransfer {
        /// The billing details for Permata Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    BcaBankTransfer {
        /// The billing details for BCA Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    BniVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    BriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    CimbVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    DanamonVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    MandiriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: Option<DokuBillingDetails>,
    },
    Pix {
        /// Unique key for pix transfer
        #[schema(value_type = Option<String>, example = "a1f4102e-a446-4a57-bcce-6fa48899c1d1")]
        pix_key: Option<Secret<String>>,
        /// CPF is a Brazilian tax identification number
        #[schema(value_type = Option<String>, example = "10599054689")]
        cpf: Option<Secret<String>>,
        /// CNPJ is a Brazilian company tax identification number
        #[schema(value_type = Option<String>, example = "74469027417312")]
        cnpj: Option<Secret<String>>,
    },
    Pse {},
    LocalBankTransfer {
        bank_code: Option<String>,
    },
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RealTimePaymentData {
    Fps {},
    DuitNow {},
    PromptPay {},
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
            Self::LocalBankTransfer { .. } | Self::Pix { .. } | Self::Pse {} => None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
pub struct BankDebitBilling {
    /// The billing name for bank debits
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub name: Option<Secret<String>>,
    /// The billing email for bank debits
    #[schema(value_type = Option<String>, example = "example@example.com")]
    pub email: Option<Email>,
    /// The billing address for bank debits
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalletData {
    /// The wallet data for Ali Pay QrCode
    AliPayQr(Box<AliPayQr>),
    /// The wallet data for Ali Pay redirect
    AliPayRedirect(AliPayRedirection),
    /// The wallet data for Ali Pay HK redirect
    AliPayHkRedirect(AliPayHkRedirection),
    /// The wallet data for Amazon Pay redirect
    AmazonPayRedirect(AmazonPayRedirectData),
    /// The wallet data for Momo redirect
    MomoRedirect(MomoRedirection),
    /// The wallet data for KakaoPay redirect
    KakaoPayRedirect(KakaoPayRedirection),
    /// The wallet data for GoPay redirect
    GoPayRedirect(GoPayRedirection),
    /// The wallet data for Gcash redirect
    GcashRedirect(GcashRedirection),
    /// The wallet data for Apple pay
    ApplePay(ApplePayWalletData),
    /// Wallet data for apple pay redirect flow
    ApplePayRedirect(Box<ApplePayRedirectData>),
    /// Wallet data for apple pay third party sdk flow
    ApplePayThirdPartySdk(Box<ApplePayThirdPartySdkData>),
    /// Wallet data for DANA redirect flow
    DanaRedirect {},
    /// The wallet data for Google pay
    GooglePay(GooglePayWalletData),
    /// Wallet data for google pay redirect flow
    GooglePayRedirect(Box<GooglePayRedirectData>),
    /// Wallet data for Google pay third party sdk flow
    GooglePayThirdPartySdk(Box<GooglePayThirdPartySdkData>),
    MbWayRedirect(Box<MbWayRedirection>),
    /// The wallet data for MobilePay redirect
    MobilePayRedirect(Box<MobilePayRedirection>),
    /// This is for paypal redirection
    PaypalRedirect(PaypalRedirection),
    /// The wallet data for Paypal
    PaypalSdk(PayPalWalletData),
    /// The wallet data for Paze
    Paze(PazeWalletData),
    /// The wallet data for Samsung Pay
    SamsungPay(Box<SamsungPayWalletData>),
    /// Wallet data for Twint Redirection
    TwintRedirect {},
    /// Wallet data for Vipps Redirection
    VippsRedirect {},
    /// The wallet data for Touch n Go Redirection
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    /// The wallet data for WeChat Pay Redirection
    WeChatPayRedirect(Box<WeChatPayRedirection>),
    /// The wallet data for WeChat Pay Display QrCode
    WeChatPayQr(Box<WeChatPayQr>),
    /// The wallet data for Cashapp Qr
    CashappQr(Box<CashappQr>),
    // The wallet data for Swish
    SwishQr(SwishQrData),
    // The wallet data for Mifinity Ewallet
    Mifinity(MifinityData),
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
            | Self::AmazonPayRedirect(_)
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
            | Self::SwishQr(_) => None,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct PazeWalletData {
    #[schema(value_type = String)]
    pub complete_response: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletData {
    pub payment_credential: SamsungPayWalletCredentials,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case", untagged)]
pub enum SamsungPayWalletCredentials {
    SamsungPayWalletDataForWeb(SamsungPayWebWalletData),
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayAppWalletData {
    /// Samsung Pay token data
    #[serde(rename = "3_d_s")]
    pub token_data: SamsungPayTokenData,
    /// Brand of the payment card
    pub payment_card_brand: SamsungPayCardBrand,
    /// Currency type of the payment
    pub payment_currency_type: String,
    /// Last 4 digits of the device specific card number
    pub payment_last4_dpan: Option<String>,
    /// Last 4 digits of the card number
    pub payment_last4_fpan: String,
    /// Merchant reference id that was passed in the session call request
    pub merchant_ref: Option<String>,
    /// Specifies authentication method used
    pub method: Option<String>,
    /// Value if credential is enabled for recurring payment
    pub recurring_payment: Option<bool>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWebWalletData {
    /// Specifies authentication method used
    pub method: Option<String>,
    /// Value if credential is enabled for recurring payment
    pub recurring_payment: Option<bool>,
    /// Brand of the payment card
    pub card_brand: SamsungPayCardBrand,
    /// Last 4 digits of the card number
    #[serde(rename = "card_last4digits")]
    pub card_last_four_digits: String,
    /// Samsung Pay token data
    #[serde(rename = "3_d_s")]
    pub token_data: SamsungPayTokenData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayTokenData {
    /// 3DS type used by Samsung Pay
    #[serde(rename = "type")]
    pub three_ds_type: Option<String>,
    /// 3DS version used by Samsung Pay
    pub version: String,
    /// Samsung Pay encrypted payment credential data
    #[schema(value_type = String)]
    pub data: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OpenBankingData {
    #[serde(rename = "open_banking_pis")]
    OpenBankingPIS {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MobilePaymentData {
    DirectCarrierBilling {
        /// The phone number of the user
        #[schema(value_type = String, example = "1234567890")]
        msisdn: String,
        /// Unique user id
        #[schema(value_type = Option<String>, example = "02iacdYXGI9CnyJdoN8c7")]
        client_uid: Option<String>,
    },
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayWalletData {
    /// The type of payment method
    #[serde(rename = "type")]
    pub pm_type: String,
    /// User-facing message to describe the payment method that funds this transaction.
    pub description: String,
    /// The information of the payment method
    pub info: GooglePayPaymentMethodInfo,
    /// The tokenization data of Google pay
    pub tokenization_data: GpayTokenizationData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AmazonPayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaypalRedirection {
    /// paypal's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    #[schema(value_type = String)]
    pub telephone_number: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
    //assurance_details of the card
    pub assurance_details: Option<GooglePayAssuranceDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayAssuranceDetails {
    ///indicates that Cardholder possession validation has been performed
    pub card_holder_authenticated: bool,
    /// indicates that identification and verifications (ID&V) was performed
    pub account_verified: bool,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MifinityData {
    #[schema(value_type = Date)]
    pub date_of_birth: Secret<Date>,
    pub language_preference: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GpayTokenizationData {
    /// The type of the token
    #[serde(rename = "type")]
    pub token_type: String,
    /// Token generated for the wallet
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: String,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplepayPaymentMethod {
    /// The name to be displayed on Apple Pay button
    pub display_name: String,
    /// The network of the Apple pay payment method
    pub network: String,
    /// The type of the payment method
    #[serde(rename = "type")]
    pub pm_type: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CardResponse {
    pub last4: Option<String>,
    pub card_type: Option<String>,
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_isin: Option<String>,
    pub card_extended_bin: Option<String>,
    #[schema(value_type = Option<String>)]
    pub card_exp_month: Option<Secret<String>>,
    #[schema(value_type = Option<String>)]
    pub card_exp_year: Option<Secret<String>>,
    #[schema(value_type = Option<String>)]
    pub card_holder_name: Option<Secret<String>>,
    pub payment_checks: Option<serde_json::Value>,
    pub authentication_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RewardData {
    /// The merchant ID with which we have to call the connector
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BoletoVoucherData {
    /// The shopper's social security number
    #[schema(value_type = Option<String>)]
    pub social_security_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoucherData {
    Boleto(Box<BoletoVoucherData>),
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart(Box<AlfamartVoucherData>),
    Indomaret(Box<IndomaretVoucherData>),
    Oxxo,
    SevenEleven(Box<JCSVoucherData>),
    Lawson(Box<JCSVoucherData>),
    MiniStop(Box<JCSVoucherData>),
    FamilyMart(Box<JCSVoucherData>),
    Seicomart(Box<JCSVoucherData>),
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodDataResponse {
    Card(Box<CardResponse>),
    BankTransfer(Box<BankTransferResponse>),
    Wallet(Box<WalletResponse>),
    PayLater(Box<PaylaterResponse>),
    BankRedirect(Box<BankRedirectResponse>),
    Crypto(Box<CryptoResponse>),
    BankDebit(Box<BankDebitResponse>),
    MandatePayment {},
    Reward {},
    RealTimePayment(Box<RealTimePaymentDataResponse>),
    Upi(Box<UpiResponse>),
    Voucher(Box<VoucherResponse>),
    GiftCard(Box<GiftCardResponse>),
    CardRedirect(Box<CardRedirectResponse>),
    CardToken(Box<CardTokenResponse>),
    OpenBanking(Box<OpenBankingResponse>),
    MobilePayment(Box<MobilePaymentResponse>),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BankDebitResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<BankDebitAdditionalData>)]
    details: Option<additional_info::BankDebitAdditionalData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct BankRedirectResponse {
    /// Name of the bank
    #[schema(value_type = Option<BankNames>)]
    pub bank_name: Option<common_enums::BankNames>,
    #[serde(flatten)]
    #[schema(value_type = Option<BankRedirectDetails>)]
    pub details: Option<additional_info::BankRedirectDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BankTransferResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<BankTransferAdditionalData>)]
    details: Option<additional_info::BankTransferAdditionalData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CardRedirectResponse {
    #[serde(flatten)]
    details: Option<CardRedirectData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CardTokenResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<CardTokenAdditionalData>)]
    details: Option<additional_info::CardTokenAdditionalData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CryptoResponse {
    #[serde(flatten)]
    details: Option<CryptoData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GiftCardResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<GiftCardAdditionalData>)]
    details: Option<additional_info::GiftCardAdditionalData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct OpenBankingResponse {
    #[serde(flatten)]
    details: Option<OpenBankingData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MobilePaymentResponse {
    #[serde(flatten)]
    details: Option<MobilePaymentData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct RealTimePaymentDataResponse {
    #[serde(flatten)]
    details: Option<RealTimePaymentData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpiResponse {
    #[serde(flatten)]
    #[schema(value_type = Option<UpiAdditionalData>)]
    details: Option<additional_info::UpiAdditionalData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct VoucherResponse {
    #[serde(flatten)]
    details: Option<VoucherData>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaylaterResponse {
    klarna_sdk: Option<KlarnaSdkPaymentMethodResponse>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct WalletResponse {
    #[serde(flatten)]
    details: Option<WalletResponseData>,
}

/// Hyperswitch supports SDK integration with Apple Pay and Google Pay wallets. For other wallets, we integrate with their respective connectors, redirecting the customer to the connector for wallet payments. As a result, we don’t receive any payment method data in the confirm call for payments made through other wallets.
#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalletResponseData {
    #[schema(value_type = WalletAdditionalDataForCard)]
    ApplePay(Box<additional_info::WalletAdditionalDataForCard>),
    #[schema(value_type = WalletAdditionalDataForCard)]
    GooglePay(Box<additional_info::WalletAdditionalDataForCard>),
    #[schema(value_type = WalletAdditionalDataForCard)]
    SamsungPay(Box<additional_info::WalletAdditionalDataForCard>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct KlarnaSdkPaymentMethodResponse {
    pub payment_type: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, ToSchema, serde::Serialize)]
pub struct PaymentMethodDataResponseWithBilling {
    // The struct is flattened in order to provide backwards compatibility
    #[serde(flatten)]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub billing: Option<Address>,
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

#[derive(Default, Clone, Debug, Eq, PartialEq, ToSchema, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    /// Provide the address details
    pub address: Option<AddressDetails>,

    pub phone: Option<PhoneDetails>,

    #[schema(value_type = Option<String>)]
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
#[derive(Clone, Default, Debug, Eq, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddressDetails {
    /// The address city
    #[schema(max_length = 50, example = "New York")]
    pub city: Option<String>,

    /// The two-letter ISO country code for the address
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub country: Option<api_enums::CountryAlpha2>,

    /// The first line of the address
    #[schema(value_type = Option<String>, max_length = 200, example = "123, King Street")]
    pub line1: Option<Secret<String>>,

    /// The second line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Powelson Avenue")]
    pub line2: Option<Secret<String>>,

    /// The third line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Bridgewater")]
    pub line3: Option<Secret<String>>,

    /// The zip/postal code for the address
    #[schema(value_type = Option<String>, max_length = 50, example = "08807")]
    pub zip: Option<Secret<String>>,

    /// The address state
    #[schema(value_type = Option<String>, example = "New York")]
    pub state: Option<Secret<String>>,

    /// The first name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "John")]
    pub first_name: Option<Secret<String>>,

    /// The last name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "Doe")]
    pub last_name: Option<Secret<String>>,
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

#[derive(Debug, Clone, Default, Eq, PartialEq, ToSchema, serde::Deserialize, serde::Serialize)]
pub struct PhoneDetails {
    /// The contact number
    #[schema(value_type = Option<String>, example = "9123456789")]
    pub number: Option<Secret<String>>,
    /// The country code attached to the number
    #[schema(example = "+1")]
    pub country_code: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentsCaptureRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    pub payment_id: id_type::PaymentId,
    /// The unique identifier for the merchant
    #[schema(value_type = Option<String>)]
    pub merchant_id: Option<id_type::MerchantId>,
    /// The Amount to be captured/ debited from the user's payment method. If not passed the full amount will be captured.
    #[schema(value_type = i64, example = 6540)]
    pub amount_to_capture: Option<MinorUnit>,
    /// Decider to refund the uncaptured amount
    pub refund_uncaptured_amount: Option<bool>,
    /// Provides information about a card payment that customers see on their statements.
    pub statement_descriptor_suffix: Option<String>,
    /// Concatenated with the statement descriptor suffix that’s set on the account to form the complete statement descriptor.
    pub statement_descriptor_prefix: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>, deprecated)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentsCaptureRequest {
    /// The Amount to be captured/ debited from the user's payment method. If not passed the full amount will be captured.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub amount_to_capture: Option<MinorUnit>,
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
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NextActionData {
    /// Contains the url for redirection flow
    #[cfg(feature = "v1")]
    RedirectToUrl {
        redirect_to_url: String,
    },
    /// Contains the url for redirection flow
    #[cfg(feature = "v2")]
    RedirectToUrl {
        #[schema(value_type = String)]
        redirect_to_url: Url,
    },
    /// Informs the next steps for bank transfer and also contains the charges details (ex: amount received, amount charged etc)
    DisplayBankTransferInformation {
        bank_transfer_steps_and_charges_details: BankTransferNextStepsData,
    },
    /// Contains third party sdk session token response
    ThirdPartySdkSessionToken {
        session_token: Option<SessionToken>,
    },
    /// Contains url for Qr code image, this qr code has to be shown in sdk
    QrCodeInformation {
        #[schema(value_type = String)]
        /// Hyperswitch generated image data source url
        image_data_url: Option<Url>,
        display_to_timestamp: Option<i64>,
        #[schema(value_type = String)]
        /// The url for Qr code given by the connector
        qr_code_url: Option<Url>,
    },
    /// Contains url to fetch Qr code data
    FetchQrCodeInformation {
        #[schema(value_type = String)]
        qr_code_fetch_url: Url,
    },
    /// Contains the download url and the reference number for transaction
    DisplayVoucherInformation {
        #[schema(value_type = String)]
        voucher_details: VoucherNextStepData,
    },
    /// Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk
    WaitScreenInformation {
        display_from_timestamp: i128,
        display_to_timestamp: Option<i128>,
    },
    /// Contains the information regarding three_ds_method_data submission, three_ds authentication, and authorization flows
    ThreeDsInvoke {
        three_ds_data: ThreeDsData,
    },
    InvokeSdkClient {
        next_action_data: SdkNextActionData,
    },
    /// Contains consent to collect otp for mobile payment
    CollectOtp {
        consent_data_required: MobilePaymentConsent,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct ThreeDsData {
    /// ThreeDS authentication url - to initiate authentication
    pub three_ds_authentication_url: String,
    /// ThreeDS authorize url - to complete the payment authorization after authentication
    pub three_ds_authorize_url: String,
    /// ThreeDS method details
    pub three_ds_method_details: ThreeDsMethodData,
    /// Poll config for a connector
    pub poll_config: PollConfigResponse,
    /// Message Version
    pub message_version: Option<String>,
    /// Directory Server ID
    pub directory_server_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(tag = "three_ds_method_key")]
pub enum ThreeDsMethodData {
    #[serde(rename = "threeDSMethodData")]
    AcsThreeDsMethodData {
        /// Whether ThreeDS method data submission is required
        three_ds_method_data_submission: bool,
        /// ThreeDS method data
        three_ds_method_data: Option<String>,
        /// ThreeDS method url
        three_ds_method_url: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct PollConfigResponse {
    /// Poll Id
    pub poll_id: String,
    /// Interval of the poll
    pub delay_in_secs: i8,
    /// Frequency of the poll
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
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SdkNextActionData {
    pub next_action: NextActionCall,
    pub order_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct FetchQrCodeInformation {
    pub qr_code_fetch_url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BankTransferNextStepsData {
    /// The instructions for performing a bank transfer
    #[serde(flatten)]
    pub bank_transfer_instructions: BankTransferInstructions,
    /// The details received by the receiver
    pub receiver: Option<ReceiverDetails>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct VoucherNextStepData {
    /// Voucher expiry date and time
    pub expires_at: Option<i64>,
    /// Reference number required for the transaction
    pub reference: String,
    /// Url to download the payment instruction
    pub download_url: Option<Url>,
    /// Url to payment instruction page
    pub instructions_url: Option<Url>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MobilePaymentNextStepData {
    /// is consent details required to be shown by sdk
    pub consent_data_required: MobilePaymentConsent,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct WaitScreenInstructions {
    pub display_from_timestamp: i128,
    pub display_to_timestamp: Option<i128>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferInstructions {
    /// The instructions for Doku bank transactions
    DokuBankTransferInstructions(Box<DokuBankTransferInstructions>),
    /// The credit transfer for ACH transactions
    AchCreditTransfer(Box<AchTransfer>),
    /// The instructions for SEPA bank transactions
    SepaBankInstructions(Box<SepaBankTransferInstructions>),
    /// The instructions for BACS bank transactions
    BacsBankInstructions(Box<BacsBankTransferInstructions>),
    /// The instructions for Multibanco bank transactions
    Multibanco(Box<MultibancoTransferInstructions>),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "9123456789")]
    pub bic: Secret<String>,
    pub country: String,
    #[schema(value_type = String, example = "123456789")]
    pub iban: Secret<String>,
    #[schema(value_type = String, example = "U2PVVSEV4V9Y")]
    pub reference: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BacsBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "10244123908")]
    pub account_number: Secret<String>,
    #[schema(value_type = String, example = "012")]
    pub sort_code: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MultibancoTransferInstructions {
    #[schema(value_type = String, example = "122385736258")]
    pub reference: Secret<String>,
    #[schema(value_type = String, example = "12345")]
    pub entity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DokuBankTransferInstructions {
    #[schema(value_type = String, example = "1707091200000")]
    pub expires_at: Option<i64>,
    #[schema(value_type = String, example = "122385736258")]
    pub reference: Secret<String>,
    #[schema(value_type = String)]
    pub instructions_url: Option<Url>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AchTransfer {
    #[schema(value_type = String, example = "122385736258")]
    pub account_number: Secret<String>,
    pub bank_name: String,
    #[schema(value_type = String, example = "012")]
    pub routing_number: Secret<String>,
    #[schema(value_type = String, example = "234")]
    pub swift_code: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReceiverDetails {
    /// The amount received by receiver
    amount_received: i64,
    /// The amount charged by ACH
    amount_charged: Option<i64>,
    /// The amount remaining to be sent via ACH
    amount_remaining: Option<i64>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema, router_derive::PolymorphicSchema)]
#[generate_schemas(PaymentsCreateResponseOpenApi)]
pub struct PaymentsResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    pub payment_id: id_type::PaymentId,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    pub status: api_enums::IntentStatus,

    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,

    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount,
    /// If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    #[schema(value_type = i64, example = 6540)]
    pub net_amount: MinorUnit,

    /// The shipping cost for the payment.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub shipping_cost: Option<MinorUnit>,

    /// The maximum amount that could be captured from the payment
    #[schema(value_type = i64, minimum = 100, example = 6540)]
    pub amount_capturable: MinorUnit,

    /// The amount which is already captured from the payment, this helps in the cases where merchants can't capture all capturable amount at once.
    #[schema(value_type = Option<i64>, example = 6540)]
    pub amount_received: Option<MinorUnit>,

    /// The connector used for the payment
    #[schema(example = "stripe")]
    pub connector: Option<String>,

    /// It's a token used for client side verification.
    #[schema(value_type = Option<String>, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<Secret<String>>,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// The currency of the amount of the payment
    #[schema(value_type = Currency, example = "USD")]
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
    pub customer_id: Option<id_type::CustomerId>,

    pub customer: Option<CustomerDetailsResponse>,

    /// A description of the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// List of refunds that happened on this intent, as same payment intent can have multiple refund requests depending on the nature of order
    #[schema(value_type = Option<Vec<RefundResponse>>)]
    pub refunds: Option<Vec<refunds::RefundResponse>>,

    /// List of disputes that happened on this intent
    #[schema(value_type = Option<Vec<DisputeResponsePaymentsRetrieve>>)]
    pub disputes: Option<Vec<disputes::DisputeResponsePaymentsRetrieve>>,

    /// List of attempts that happened on this intent
    #[schema(value_type = Option<Vec<PaymentAttemptResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<Vec<PaymentAttemptResponse>>,

    /// List of captures done on latest attempt
    #[schema(value_type = Option<Vec<CaptureResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captures: Option<Vec<CaptureResponse>>,

    /// A unique identifier to link the payment to a mandate, can be used instead of payment_method_data, in case of setting up recurring payments
    #[schema(max_length = 255, example = "mandate_iwer89rnjef349dni3")]
    pub mandate_id: Option<String>,

    /// Provided mandate information for creating a mandate
    pub mandate_data: Option<MandateData>,

    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    #[schema(example = true)]
    pub off_session: Option<bool>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[remove_in(PaymentsCreateResponseOpenApi)]
    pub capture_on: Option<PrimitiveDateTime>,

    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    /// The payment method that is to be used
    #[schema(value_type = PaymentMethod, example = "bank_transfer")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethodDataResponseWithBilling>, example = "bank_transfer")]
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,

    /// Provide a reference to a stored payment method
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// The billing address for the payment
    pub billing: Option<Address>,

    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "gillete creme",
        "quantity": 15,
        "amount" : 900
    }]"#)]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,

    /// description: The customer's email address
    /// This field will be deprecated soon. Please refer to `customer.email` object
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com", deprecated)]
    pub email: crypto::OptionalEncryptableEmail,

    /// description: The customer's name
    /// This field will be deprecated soon. Please refer to `customer.name` object
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test", deprecated)]
    pub name: crypto::OptionalEncryptableName,

    /// The customer's phone number
    /// This field will be deprecated soon. Please refer to `customer.phone` object
    #[schema(value_type = Option<String>, max_length = 255, example = "9123456789", deprecated)]
    pub phone: crypto::OptionalEncryptablePhone,

    /// The URL to redirect after the completion of the operation
    #[schema(example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS, as the 3DS method helps with more robust payer authentication
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 255, example = "Hyperswitch Router")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor.
    #[schema(max_length = 255, example = "Payment for shoes purchase")]
    pub statement_descriptor_suffix: Option<String>,

    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,

    /// If the payment was cancelled the reason will be provided here
    pub cancellation_reason: Option<String>,

    /// If there was an error while calling the connectors the code is received here
    #[schema(example = "E0001")]
    pub error_code: Option<String>,

    /// If there was an error while calling the connector the error message is received here
    #[schema(example = "Failed while verifying the card")]
    pub error_message: Option<String>,

    /// error code unified across the connectors is received here if there was an error while calling connector
    #[remove_in(PaymentsCreateResponseOpenApi)]
    pub unified_code: Option<String>,

    /// error message unified across the connectors is received here if there was an error while calling connector
    #[remove_in(PaymentsCreateResponseOpenApi)]
    pub unified_message: Option<String>,

    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// Can be used to specify the Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "gpay")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// The connector used for this payment along with the country and business details
    #[schema(example = "stripe_US_food")]
    pub connector_label: Option<String>,

    /// The business country of merchant for this payment
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// The business label of merchant for this payment
    pub business_label: Option<String>,

    /// The business_sub_label for this payment
    pub business_sub_label: Option<String>,

    /// Allowed Payment Method Types for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<serde_json::Value>,

    /// ephemeral_key for the customer_id mentioned
    pub ephemeral_key: Option<EphemeralKeyCreateResponse>,

    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    pub manual_retry_allowed: Option<bool>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<String>,

    /// Frm message contains information about the frm response
    pub frm_message: Option<FrmMessage>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<serde_json::Value>,

    /// Additional data related to some connectors
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    #[schema(value_type = Option<FeatureMetadata>)]
    pub feature_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub reference_id: Option<String>,

    /// Details for Payment link
    pub payment_link: Option<PaymentLinkResponse>,
    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// Details of surcharge applied on this payment
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// Total number of attempts associated with this payment
    pub attempt_count: i16,

    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// If true, incremental authorization can be performed on this payment, in case the funds authorized initially fall short.
    pub incremental_authorization_allowed: Option<bool>,

    /// Total number of authorizations happened in an incremental_authorization payment
    pub authorization_count: Option<i32>,

    /// List of incremental authorizations happened to the payment
    pub incremental_authorizations: Option<Vec<IncrementalAuthorizationResponse>>,

    /// Details of external authentication
    pub external_authentication_details: Option<ExternalAuthenticationDetailsResponse>,

    /// Flag indicating if external 3ds authentication is made or not
    pub external_3ds_authentication_attempted: Option<bool>,

    /// Date Time for expiry of the payment
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_on: Option<PrimitiveDateTime>,

    /// Payment Fingerprint, to identify a particular card.
    /// It is a 20 character long alphanumeric code.
    pub fingerprint: Option<String>,

    #[schema(value_type = Option<BrowserInformation>)]
    /// The browser information used for this payment
    pub browser_info: Option<serde_json::Value>,

    /// Identifier for Payment Method used for the payment
    pub payment_method_id: Option<String>,

    /// Payment Method Status, refers to the status of the payment method used for this payment.
    #[schema(value_type = Option<PaymentMethodStatus>)]
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,

    /// Date time at which payment was updated
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub updated: Option<PrimitiveDateTime>,

    /// Fee information to be charged on the payment being collected
    #[schema(value_type = Option<ConnectorChargeResponseData>)]
    pub split_payments: Option<common_types::payments::ConnectorChargeResponseData>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. FRM Metadata is useful for storing additional, structured information on an object related to FRM.
    #[schema(value_type = Option<Object>, example = r#"{ "fulfillment_method" : "deliver", "coverage_request" : "fraud" }"#)]
    pub frm_metadata: Option<pii::SecretSerdeValue>,

    /// flag that indicates if extended authorization is applied on this payment or not
    #[schema(value_type = Option<bool>)]
    pub extended_authorization_applied: Option<ExtendedAuthorizationAppliedBool>,

    /// date and time after which this payment cannot be captured
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_before: Option<PrimitiveDateTime>,

    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    #[schema(
        value_type = Option<String>,
        max_length = 255,
        example = "Custom_Order_id_123"
    )]
    pub merchant_order_reference_id: Option<String>,
    /// order tax amount calculated by tax connectors
    pub order_tax_amount: Option<MinorUnit>,

    /// Connector Identifier for the payment method
    pub connector_mandate_id: Option<String>,

    /// Method through which card was discovered
    #[schema(value_type = Option<CardDiscovery>, example = "manual")]
    pub card_discovery: Option<enums::CardDiscovery>,
}

// Serialize is implemented because, this will be serialized in the api events.
// Usually request types should not have serialize implemented.
//
/// Request for Payment Intent Confirm
#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentsConfirmIntentRequest {
    /// The URL to which you want the user to be redirected after the completion of the payment operation
    /// If this url is not passed, the url configured in the business profile will be used
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<common_utils::types::Url>,

    /// The payment instrument data to be used for the payment
    pub payment_method_data: PaymentMethodDataRequest,

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
    pub customer_acceptance: Option<CustomerAcceptance>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,
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
    pub customer_acceptance: Option<CustomerAcceptance>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,
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
            customer_present: request.customer_present.clone(),
            description: request.description.clone(),
            return_url: request.return_url.clone(),
            setup_future_usage: request.setup_future_usage,
            apply_mit_exemption: request.apply_mit_exemption.clone(),
            statement_descriptor: request.statement_descriptor.clone(),
            order_details: request.order_details.clone(),
            allowed_payment_method_types: request.allowed_payment_method_types.clone(),
            metadata: request.metadata.clone(),
            connector_metadata: request.connector_metadata.clone(),
            feature_metadata: request.feature_metadata.clone(),
            payment_link_enabled: request.payment_link_enabled.clone(),
            payment_link_config: request.payment_link_config.clone(),
            request_incremental_authorization: request.request_incremental_authorization,
            session_expiry: request.session_expiry,
            frm_metadata: request.frm_metadata.clone(),
            request_external_three_ds_authentication: request
                .request_external_three_ds_authentication
                .clone(),
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
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, ToSchema)]
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

    #[schema(value_type = IntentStatus, example = "success")]
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
    pub connector: String,

    /// It's a token used for client side verification.
    #[schema(value_type = String)]
    pub client_secret: common_utils::types::ClientSecret,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethodDataResponseWithBilling>)]
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,

    /// The payment method type for this payment attempt
    #[schema(value_type = PaymentMethod, example = "wallet")]
    pub payment_method_type: api_enums::PaymentMethod,

    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<String>,

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_reference_id: Option<String>,

    /// Connector token information that can be used to make payments directly by the merchant.
    pub connector_token_details: Option<ConnectorTokenDetails>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,

    /// The browser information used for this payment
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,
}

// Serialize is implemented because, this will be serialized in the api events.
// Usually request types should not have serialize implemented.
//
/// Request for Payment Status
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
}

/// Error details for the payment
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, PartialEq, ToSchema)]
pub struct ErrorDetails {
    /// The error code
    pub code: String,
    /// The error message
    pub message: String,
    /// The unified error code across all connectors.
    /// This can be relied upon for taking decisions based on the error.
    pub unified_code: Option<String>,
    /// The unified error message across all connectors.
    /// If there is a translation available, this will have the translated message
    pub unified_message: Option<String>,
}

/// Response for Payment Intent Confirm
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentsConfirmIntentResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        min_length = 32,
        max_length = 64,
        example = "12345_pay_01926c58bc6e77c09e809964e72af8c8",
        value_type = String,
    )]
    pub id: id_type::GlobalPaymentId,

    #[schema(value_type = IntentStatus, example = "success")]
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
    pub connector: String,

    /// It's a token used for client side verification.
    #[schema(value_type = String)]
    pub client_secret: common_utils::types::ClientSecret,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethodDataResponseWithBilling>)]
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    pub payment_method_data: Option<PaymentMethodDataResponseWithBilling>,

    /// The payment method type for this payment attempt
    #[schema(value_type = PaymentMethod, example = "wallet")]
    pub payment_method_type: api_enums::PaymentMethod,

    #[schema(value_type = PaymentMethodType, example = "apple_pay")]
    pub payment_method_subtype: api_enums::PaymentMethodType,

    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<String>,

    /// reference(Identifier) to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_reference_id: Option<String>,

    /// Connector token information that can be used to make payments directly by the merchant.
    pub connector_token_details: Option<ConnectorTokenDetails>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,

    /// The browser information used for this payment
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_info: Option<common_utils::types::BrowserInformation>,

    /// Error details for the payment if any
    pub error: Option<ErrorDetails>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The authentication type applied for the payment
    #[schema(value_type = AuthenticationType, example = "no_three_ds")]
    pub applied_authentication_type: api_enums::AuthenticationType,
}

/// Token information that can be used to initiate transactions by the merchant.
#[cfg(feature = "v2")]
#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectorTokenDetails {
    /// A token that can be used to make payments directly with the connector.
    #[schema(example = "pm_9UhMqBMEOooRIvJFFdeW")]
    pub token: String,
}

// TODO: have a separate response for detailed, summarized
/// Response for Payment Intent Confirm
#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsRetrieveResponse {
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

    /// It's a token used for client side verification.
    #[schema(value_type = String)]
    pub client_secret: common_utils::types::ClientSecret,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,

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
#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct ExternalAuthenticationDetailsResponse {
    /// Authentication Type - Challenge / Frictionless
    #[schema(value_type = Option<DecoupledAuthenticationType>)]
    pub authentication_flow: Option<enums::DecoupledAuthenticationType>,
    /// Electronic Commerce Indicator (eci)
    pub electronic_commerce_indicator: Option<String>,
    /// Authentication Status
    #[schema(value_type = AuthenticationStatus)]
    pub status: enums::AuthenticationStatus,
    /// DS Transaction ID
    pub ds_transaction_id: Option<String>,
    /// Message Version
    pub version: Option<String>,
    /// Error Code
    pub error_code: Option<String>,
    /// Error Message
    pub error_message: Option<String>,
}

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

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentListResponse {
    /// The number of payments included in the list
    pub size: usize,
    // The list of payments response objects
    pub data: Vec<PaymentsResponse>,
}

#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct IncrementalAuthorizationResponse {
    /// The unique identifier of authorization
    pub authorization_id: String,
    /// Amount the authorization has been made for
    #[schema(value_type = i64, example = 6540)]
    pub amount: MinorUnit,
    #[schema(value_type= AuthorizationStatus)]
    /// The status of the authorization
    pub status: common_enums::AuthorizationStatus,
    /// Error code sent by the connector for authorization
    pub error_code: Option<String>,
    /// Error message sent by the connector for authorization
    pub error_message: Option<String>,
    /// Previously authorized amount for the payment
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
                        additional_info::WalletAdditionalDataForCard {
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
                        },
                    ))),
                })),
                (_, Some(google_pay_pm), _) => Self::Wallet(Box::new(WalletResponse {
                    details: Some(WalletResponseData::GooglePay(Box::new(google_pay_pm))),
                })),
                (_, _, Some(samsung_pay_pm)) => Self::Wallet(Box::new(WalletResponse {
                    details: Some(WalletResponseData::SamsungPay(Box::new(samsung_pay_pm))),
                })),
                _ => Self::Wallet(Box::new(WalletResponse { details: None })),
            },
            AdditionalPaymentData::BankRedirect { bank_name, details } => {
                Self::BankRedirect(Box::new(BankRedirectResponse { bank_name, details }))
            }
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
    /// The parameters passed to a retrieve request
    pub param: Option<String>,
    /// The name of the connector
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
}

#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    #[schema(max_length = 255, example = "shirt")]
    pub product_name: String,
    /// The quantity of the product to be purchased
    #[schema(example = 1)]
    pub quantity: u16,
    /// the amount per quantity of product
    #[schema(value_type = i64)]
    pub amount: MinorUnit,
    /// tax rate applicable to the product
    pub tax_rate: Option<f64>,
    /// total tax amount applicable to the product
    #[schema(value_type = Option<i64>)]
    pub total_tax_amount: Option<MinorUnit>,
    // Does the order includes shipping
    pub requires_shipping: Option<bool>,
    /// The image URL of the product
    pub product_img_link: Option<String>,
    /// ID of the product that is being purchased
    pub product_id: Option<String>,
    /// Category of the product that is being purchased
    pub category: Option<String>,
    /// Sub category of the product that is being purchased
    pub sub_category: Option<String>,
    /// Brand of the product that is being purchased
    pub brand: Option<String>,
    /// Type of the product that is being purchased
    pub product_type: Option<ProductType>,
    /// The tax code for the product
    pub product_tax_code: Option<String>,
}

impl masking::SerializableSecret for OrderDetailsWithAmount {}

#[derive(Default, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct RedirectResponse {
    #[schema(value_type = Option<String>)]
    pub param: Option<Secret<String>>,
    #[schema(value_type = Option<Object>)]
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayAllowedMethodsParameters {
    /// The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
    pub allowed_auth_methods: Vec<String>,
    /// The list of allowed card networks (ex: AMEX,JCB etc)
    pub allowed_card_networks: Vec<String>,
    /// Is billing address required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address_required: Option<bool>,
    /// Billing address parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address_parameters: Option<GpayBillingAddressParameters>,
    /// Whether assurance details are required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assurance_details_required: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayBillingAddressParameters {
    /// Is billing phone number required
    pub phone_number_required: bool,
    /// Billing address format
    pub format: GpayBillingAddressFormat,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub enum GpayBillingAddressFormat {
    FULL,
    MIN,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTokenParameters {
    /// The name of the connector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// The merchant ID registered in the connector associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_merchant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stripe:version")]
    pub stripe_version: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "stripe:publishableKey"
    )]
    pub stripe_publishable_key: Option<String>,
    /// The protocol version for encryption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    /// The public key provided by the merchant
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub public_key: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTokenizationSpecification {
    /// The token specification type(ex: PAYMENT_GATEWAY)
    #[serde(rename = "type")]
    pub token_specification_type: String,
    /// The parameters for the token specification Google Pay
    pub parameters: GpayTokenParameters,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayAllowedPaymentMethods {
    /// The type of payment method
    #[serde(rename = "type")]
    pub payment_method_type: String,
    /// The parameters Google Pay requires
    pub parameters: GpayAllowedMethodsParameters,
    /// The tokenization specification for Google Pay
    pub tokenization_specification: GpayTokenizationSpecification,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTransactionInfo {
    /// The country code
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub country_code: api_enums::CountryAlpha2,
    /// The currency code
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// The total price status (ex: 'FINAL')
    pub total_price_status: String,
    /// The total price
    #[schema(value_type = String, example = "38.02")]
    pub total_price: StringMajorUnit,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayMerchantInfo {
    /// The merchant Identifier that needs to be passed while invoking Gpay SDK
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
    /// The name of the merchant that needs to be displayed on Gpay PopUp
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConnectorMetadata {
    pub apple_pay: Option<ApplepayConnectorMetadataRequest>,
    pub airwallex: Option<AirwallexData>,
    pub noon: Option<NoonData>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AirwallexData {
    /// payload required by airwallex
    payload: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct NoonData {
    /// Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by the merchant in Noon's Dashboard)
    pub order_category: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ApplepayConnectorMetadataRequest {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SessionTokenInfo {
    #[schema(value_type = String)]
    pub certificate: Secret<String>,
    #[schema(value_type = String)]
    pub certificate_keys: Secret<String>,
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: ApplepayInitiative,
    pub initiative_context: Option<String>,
    #[schema(value_type = Option<CountryAlpha2>)]
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    #[serde(flatten)]
    pub payment_processing_details_at: Option<PaymentProcessingDetailsAt>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Display, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApplepayInitiative {
    Web,
    Ios,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(tag = "payment_processing_details_at")]
pub enum PaymentProcessingDetailsAt {
    Hyperswitch(PaymentProcessingDetails),
    Connector,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, ToSchema)]
pub struct PaymentProcessingDetails {
    #[schema(value_type = String)]
    pub payment_processing_certificate: Secret<String>,
    #[schema(value_type = String)]
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(tag = "wallet_name")]
#[serde(rename_all = "snake_case")]
pub enum SessionToken {
    /// The session response structure for Google Pay
    GooglePay(Box<GpaySessionTokenResponse>),
    /// The session response structure for Samsung Pay
    SamsungPay(Box<SamsungPaySessionTokenResponse>),
    /// The session response structure for Klarna
    Klarna(Box<KlarnaSessionTokenResponse>),
    /// The session response structure for PayPal
    Paypal(Box<PaypalSessionTokenResponse>),
    /// The session response structure for Apple Pay
    ApplePay(Box<ApplepaySessionTokenResponse>),
    /// Session token for OpenBanking PIS flow
    OpenBanking(OpenBankingSessionToken),
    /// The session response structure for Paze
    Paze(Box<PazeSessionTokenResponse>),
    /// The sessions response structure for ClickToPay
    ClickToPay(Box<ClickToPaySessionResponse>),
    /// Whenever there is no session token response or an error in session response
    NoSessionTokenReceived,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct PazeSessionTokenResponse {
    /// Paze Client ID
    pub client_id: String,
    /// Client Name to be displayed on the Paze screen
    pub client_name: String,
    /// Paze Client Profile ID
    pub client_profile_id: String,
    /// The transaction currency code
    #[schema(value_type = Currency, example = "USD")]
    pub transaction_currency_code: api_enums::Currency,
    /// The transaction amount
    #[schema(value_type = String, example = "38.02")]
    pub transaction_amount: StringMajorUnit,
    /// Email Address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email_address: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(untagged)]
pub enum GpaySessionTokenResponse {
    /// Google pay response involving third party sdk
    ThirdPartyResponse(GooglePayThirdPartySdk),
    /// Google pay session response for non third party sdk
    GooglePaySession(GooglePaySessionResponse),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct GooglePayThirdPartySdk {
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The name of the connector
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct GooglePaySessionResponse {
    /// The merchant info
    pub merchant_info: GpayMerchantInfo,
    /// Is shipping address required
    pub shipping_address_required: bool,
    /// Is email required
    pub email_required: bool,
    /// Shipping address parameters
    pub shipping_address_parameters: GpayShippingAddressParameters,
    /// List of the allowed payment meythods
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
    /// The transaction info Google Pay requires
    pub transaction_info: GpayTransactionInfo,
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The name of the connector
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
    /// Secrets for sdk display and payment
    pub secrets: Option<SecretInfoToInitiateSdk>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct SamsungPaySessionTokenResponse {
    /// Samsung Pay API version
    pub version: String,
    /// Samsung Pay service ID to which session call needs to be made
    pub service_id: String,
    /// Order number of the transaction
    pub order_number: String,
    /// Field containing merchant information
    #[serde(rename = "merchant")]
    pub merchant_payment_information: SamsungPayMerchantPaymentInformation,
    /// Field containing the payment amount
    pub amount: SamsungPayAmountDetails,
    /// Payment protocol type
    pub protocol: SamsungPayProtocolType,
    /// List of supported card brands
    pub allowed_brands: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SamsungPayProtocolType {
    Protocol3ds,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct SamsungPayMerchantPaymentInformation {
    /// Merchant name, this will be displayed on the Samsung Pay screen
    pub name: String,
    /// Merchant domain that process payments, required for web payments
    pub url: Option<String>,
    /// Merchant country code
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub country_code: api_enums::CountryAlpha2,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct SamsungPayAmountDetails {
    #[serde(rename = "option")]
    /// Amount format to be displayed
    pub amount_format: SamsungPayAmountFormat,
    /// The currency code
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// The total amount of the transaction
    #[serde(rename = "total")]
    #[schema(value_type = String, example = "38.02")]
    pub total_amount: StringMajorUnit,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SamsungPayAmountFormat {
    /// Display the total amount only
    FormatTotalPriceOnly,
    /// Display "Total (Estimated amount)" and total amount
    FormatTotalEstimatedAmount,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct GpayShippingAddressParameters {
    /// Is shipping phone number required
    pub phone_number_required: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct KlarnaSessionTokenResponse {
    /// The session token for Klarna
    pub session_token: String,
    /// The identifier for the session
    pub session_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct PaypalSessionTokenResponse {
    /// Name of the connector
    pub connector: String,
    /// The session token for PayPal
    pub session_token: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct OpenBankingSessionToken {
    /// The session token for OpenBanking Connectors
    pub open_banking_session_token: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct ApplepaySessionTokenResponse {
    /// Session object for Apple Pay
    /// The session_token_data will be null for iOS devices because the Apple Pay session call is skipped, as there is no web domain involved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token_data: Option<ApplePaySessionResponse>,
    /// Payment request object for Apple Pay
    pub payment_request_data: Option<ApplePayPaymentRequest>,
    /// The session token is w.r.t this connector
    pub connector: String,
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
    /// The connector transaction id
    pub connector_reference_id: Option<String>,
    /// The public key id is to invoke third party sdk
    pub connector_sdk_public_key: Option<String>,
    /// The connector merchant id
    pub connector_merchant_id: Option<String>,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize, Clone, ToSchema)]
pub struct SdkNextAction {
    /// The type of next action
    pub next_action: NextActionCall,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionCall {
    /// The next action call is Post Session Tokens
    PostSessionTokens,
    /// The next action call is confirm
    Confirm,
    /// The next action call is sync
    Sync,
    /// The next action call is Complete Authorize
    CompleteAuthorize,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(untagged)]
pub enum ApplePaySessionResponse {
    ///  We get this session response, when third party sdk is involved
    ThirdPartySdk(ThirdPartySdkSessionResponse),
    ///  We get this session response, when there is no involvement of third party sdk
    /// This is the common response most of the times
    NoThirdPartySdk(NoThirdPartySdkSessionResponse),
    /// This is for the empty session response
    NoSessionResponse,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct NoThirdPartySdkSessionResponse {
    /// Timestamp at which session is requested
    pub epoch_timestamp: u64,
    /// Timestamp at which session expires
    pub expires_at: u64,
    /// The identifier for the merchant session
    pub merchant_session_identifier: String,
    /// Apple pay generated unique ID (UUID) value
    pub nonce: String,
    /// The identifier for the merchant
    pub merchant_identifier: String,
    /// The domain name of the merchant which is registered in Apple Pay
    pub domain_name: String,
    /// The name to be displayed on Apple Pay button
    pub display_name: String,
    /// A string which represents the properties of a payment
    pub signature: String,
    /// The identifier for the operational analytics
    pub operational_analytics_identifier: String,
    /// The number of retries to get the session response
    pub retries: u8,
    /// The identifier for the connector transaction
    pub psp_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct ThirdPartySdkSessionResponse {
    pub secrets: SecretInfoToInitiateSdk,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct SecretInfoToInitiateSdk {
    // Authorization secrets used by client to initiate sdk
    #[schema(value_type = String)]
    pub display: Secret<String>,
    // Authorization secrets used by client for payment
    #[schema(value_type = String)]
    pub payment: Secret<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct ApplePayPaymentRequest {
    /// The code for country
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub country_code: api_enums::CountryAlpha2,
    /// The code for currency
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// Represents the total for the payment.
    pub total: AmountInfo,
    /// The list of merchant capabilities(ex: whether capable of 3ds or no-3ds)
    pub merchant_capabilities: Option<Vec<String>>,
    /// The list of supported networks
    pub supported_networks: Option<Vec<String>>,
    pub merchant_identifier: Option<String>,
    /// The required billing contact fields for connector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_billing_contact_fields: Option<ApplePayBillingContactFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The required shipping contacht fields for connector
    pub required_shipping_contact_fields: Option<ApplePayShippingContactFields>,
    /// Recurring payment request for apple pay Merchant Token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_payment_request: Option<ApplePayRecurringPaymentRequest>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRecurringPaymentRequest {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    pub payment_description: String,
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    pub regular_billing: ApplePayRegularBillingRequest,
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_agreement: Option<String>,
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    pub management_u_r_l: common_utils::types::Url,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRegularBillingRequest {
    /// The amount of the recurring payment
    #[schema(value_type = String, example = "38.02")]
    pub amount: StringMajorUnit,
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    pub label: String,
    /// The time that the payment occurs as part of a successful transaction
    pub payment_timing: ApplePayPaymentTiming,
    /// The date of the first payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_start_date: Option<PrimitiveDateTime>,
    /// The date of the final payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_end_date: Option<PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_payment_interval_count: Option<i32>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplePayAddressParameters {
    PostalAddress,
    Phone,
    Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct AmountInfo {
    /// The label must be the name of the merchant.
    pub label: String,
    /// A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending.
    #[serde(rename = "type")]
    pub total_type: Option<String>,
    /// The total amount for the payment in majot unit string (Ex: 38.02)
    #[schema(value_type = String, example = "38.02")]
    pub amount: StringMajorUnit,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayErrorResponse {
    pub status_code: String,
    pub status_message: String,
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
}

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
}

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
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsCancelRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: id_type::PaymentId,
    /// The reason for the payment cancel
    pub cancellation_reason: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>, deprecated)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
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
pub struct PaymentMethodsListRequest {}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodListResponseForPayments {
    /// The list of payment methods that are enabled for the business profile
    pub payment_methods_enabled: Vec<ResponsePaymentMethodTypesForPayments>,

    /// The list of payment methods that are saved by the given customer
    /// This field is only returned if the customer_id is provided in the request
    #[schema(value_type = Option<Vec<CustomerPaymentMethod>>)]
    pub customer_payment_methods: Option<Vec<payment_methods::CustomerPaymentMethod>>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Clone, serde::Serialize, ToSchema, PartialEq)]
pub struct ResponsePaymentMethodTypesForPayments {
    /// The payment method type enabled
    #[schema(example = "pay_later", value_type = PaymentMethod)]
    pub payment_method_type: common_enums::PaymentMethod,

    /// The payment method subtype enabled
    #[schema(example = "klarna", value_type = PaymentMethodType)]
    pub payment_method_subtype: common_enums::PaymentMethodType,

    /// payment method subtype specific information
    #[serde(flatten)]
    #[schema(value_type = Option<PaymentMethodSubtypeSpecificData>)]
    pub extra_information: Option<payment_methods::PaymentMethodSubtypeSpecificData>,

    /// Required fields for the payment_method_type.
    /// This is the union of all the required fields for the payment method type enabled in all the connectors.
    #[schema(value_type = Option<RequiredFieldInfo>)]
    pub required_fields: Option<Vec<payment_methods::RequiredFieldInfo>>,

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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
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
    pub payment_revenue_recovery_metadata: Option<PaymentRevenueRecoveryMetadata>,
}

/// additional data that might be required by hyperswitch
#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    #[schema(value_type = Option<RedirectResponse>)]
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    #[schema(value_type = Option<Vec<String>>)]
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRecurringDetails {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    pub payment_description: String,
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    pub regular_billing: ApplePayRegularBillingDetails,
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    pub billing_agreement: Option<String>,
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    pub management_url: common_utils::types::Url,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRegularBillingDetails {
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    pub label: String,
    /// The date of the first payment
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_start_date: Option<PrimitiveDateTime>,
    /// The date of the final payment
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_end_date: Option<PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    pub recurring_payment_interval_count: Option<i32>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecurringPaymentIntervalUnit {
    Year,
    Month,
    Day,
    Hour,
    Minute,
}

///frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct FrmMessage {
    pub frm_name: String,
    pub frm_transaction_id: Option<String>,
    pub frm_transaction_type: Option<String>,
    pub frm_status: Option<String>,
    pub frm_score: Option<i32>,
    pub frm_reason: Option<serde_json::Value>,
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
    #![allow(clippy::unwrap_used)]
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

#[derive(Debug, serde::Serialize, Clone)]
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
    pub custom_message_for_card_terms: Option<String>,
    pub payment_button_colour: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct SecurePaymentLinkDetails {
    pub enabled_saved_payment_method: bool,
    pub hide_card_nickname_field: bool,
    pub show_card_form_by_default: bool,
    #[serde(flatten)]
    pub payment_link_details: PaymentLinkDetails,
    pub payment_button_text: Option<String>,
    pub custom_message_for_card_terms: Option<String>,
    pub payment_button_colour: Option<String>,
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct ClickToPaySessionResponse {
    pub dpa_id: String,
    pub dpa_name: String,
    pub locale: String,
    pub card_brands: Vec<String>,
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
    pub merchant_category_code: String,
    pub merchant_country_code: String,
    #[schema(value_type = String, example = "38.02")]
    pub transaction_amount: StringMajorUnit,
    #[schema(value_type = Currency)]
    pub transaction_currency_code: common_enums::Currency,
    #[schema(value_type = Option<String>, max_length = 255, example = "9123456789")]
    pub phone_number: Option<Secret<String>>,
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,
    pub phone_country_code: Option<String>,
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod payments_request_api_contract {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::panic)]
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
    #![allow(clippy::unwrap_used)]
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
    #![allow(clippy::unwrap_used)]
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
    pub payment_connector_transmission: PaymentConnectorTransmission,
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
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg(feature = "v2")]
pub struct BillingConnectorPaymentDetails {
    /// Payment Processor Token to process the Revenue Recovery Payment
    pub payment_processor_token: String,
    /// Billing Connector's Customer Id
    pub connector_customer_id: String,
}
