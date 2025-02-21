use std::collections::{HashMap, HashSet};

use common_utils::{
    consts,
    crypto::Encryptable,
    errors::{self, CustomResult},
    ext_traits::Encode,
    id_type, link_utils, pii,
};
#[cfg(feature = "v1")]
use common_utils::{
    crypto::OptionalEncryptableName, ext_traits::ValueExt,
    types::AlwaysRequestExtendedAuthorization,
};
#[cfg(feature = "v2")]
use masking::ExposeInterface;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url;
use utoipa::ToSchema;

use super::payments::AddressDetails;
#[cfg(feature = "v1")]
use crate::routing;
use crate::{
    consts::{MAX_ORDER_FULFILLMENT_EXPIRY, MIN_ORDER_FULFILLMENT_EXPIRY},
    enums as api_enums, payment_methods,
};

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantAccountListRequest {
    pub organization_id: id_type::OrganizationId,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantAccountCreate {
    /// The identifier for the Merchant Account
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: id_type::MerchantId,

    /// Name of the Merchant Account
    #[schema(value_type= Option<String>,example = "NewAge Retailer")]
    pub merchant_name: Option<Secret<String>>,

    /// Details about the merchant, can contain phone and emails of primary and secondary contact person
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf", value_type = Option<String>)]
    pub parent_merchant_id: Option<id_type::MerchantId>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled.
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Metadata is useful for storing additional, unstructured information on an object
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<MerchantAccountMetadata>,

    /// API key that will be used for client side API access. A publishable key has to be always paired with a `client_secret`.
    /// A `client_secret` can be obtained by creating a payment with `confirm` set to false
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    #[schema(value_type = Option<PrimaryBusinessDetails>)]
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The id of the organization to which the merchant belongs to, if not passed an organization is created
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "org_q98uSGAYbjEwqs0mJwnz")]
    pub organization_id: Option<id_type::OrganizationId>,

    /// Default payment method collect link config
    #[schema(value_type = Option<BusinessCollectLinkConfig>)]
    pub pm_collect_link_config: Option<BusinessCollectLinkConfig>,
}

#[cfg(feature = "v1")]
impl MerchantAccountCreate {
    pub fn get_merchant_reference_id(&self) -> id_type::MerchantId {
        self.merchant_id.clone()
    }

    pub fn get_payment_response_hash_key(&self) -> Option<String> {
        self.payment_response_hash_key.clone().or(Some(
            common_utils::crypto::generate_cryptographically_secure_random_string(64),
        ))
    }

    pub fn get_primary_details_as_value(
        &self,
    ) -> CustomResult<serde_json::Value, errors::ParsingError> {
        self.primary_business_details
            .clone()
            .unwrap_or_default()
            .encode_to_value()
    }

    pub fn get_pm_link_config_as_value(
        &self,
    ) -> CustomResult<Option<serde_json::Value>, errors::ParsingError> {
        self.pm_collect_link_config
            .as_ref()
            .map(|pm_collect_link_config| pm_collect_link_config.encode_to_value())
            .transpose()
    }

    pub fn get_merchant_details_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.merchant_details
            .as_ref()
            .map(|merchant_details| merchant_details.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_metadata_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn parse_routing_algorithm(&self) -> CustomResult<(), errors::ParsingError> {
        match self.routing_algorithm {
            Some(ref routing_algorithm) => {
                let _: routing::RoutingAlgorithm =
                    routing_algorithm.clone().parse_value("RoutingAlgorithm")?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    // Get the enable payment response hash as a boolean, where the default value is true
    pub fn get_enable_payment_response_hash(&self) -> bool {
        self.enable_payment_response_hash.unwrap_or(true)
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
#[schema(as = MerchantAccountCreate)]
pub struct MerchantAccountCreateWithoutOrgId {
    /// Name of the Merchant Account, This will be used as a prefix to generate the id
    #[schema(value_type= String, max_length = 64, example = "NewAge Retailer")]
    pub merchant_name: Secret<common_utils::new_type::MerchantName>,

    /// Details about the merchant, contains phone and emails of primary and secondary contact person.
    pub merchant_details: Option<MerchantDetails>,

    /// Metadata is useful for storing additional, unstructured information about the merchant account.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

// In v2 the struct used in the API is MerchantAccountCreateWithoutOrgId
// The following struct is only used internally, so we can reuse the common
// part of `create_merchant_account` without duplicating its code for v2
#[cfg(feature = "v2")]
#[derive(Clone, Debug, Serialize)]
pub struct MerchantAccountCreate {
    pub merchant_name: Secret<common_utils::new_type::MerchantName>,
    pub merchant_details: Option<MerchantDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub organization_id: id_type::OrganizationId,
}

#[cfg(feature = "v2")]
impl MerchantAccountCreate {
    pub fn get_merchant_reference_id(&self) -> id_type::MerchantId {
        id_type::MerchantId::from_merchant_name(self.merchant_name.clone().expose())
    }

    pub fn get_merchant_details_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.merchant_details
            .as_ref()
            .map(|merchant_details| merchant_details.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_metadata_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_primary_details_as_value(
        &self,
    ) -> CustomResult<serde_json::Value, errors::ParsingError> {
        Vec::<PrimaryBusinessDetails>::new().encode_to_value()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuthenticationConnectorDetails {
    /// List of authentication connectors
    #[schema(value_type = Vec<AuthenticationConnectors>)]
    pub authentication_connectors: Vec<common_enums::AuthenticationConnectors>,
    /// URL of the (customer service) website that will be shown to the shopper in case of technical errors during the 3D Secure 2 process.
    pub three_ds_requestor_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MerchantAccountMetadata {
    pub compatible_connector: Option<api_enums::Connector>,

    #[serde(flatten)]
    pub data: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantAccountUpdate {
    /// The identifier for the Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// Name of the Merchant Account
    #[schema(example = "NewAge Retailer")]
    pub merchant_name: Option<String>,

    /// Details about the merchant
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf", value_type = Option<String>)]
    pub parent_merchant_id: Option<id_type::MerchantId>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The default profile that must be used for creating merchant accounts and payments
    #[schema(max_length = 64, value_type = Option<String>)]
    pub default_profile: Option<id_type::ProfileId>,

    /// Default payment method collect link config
    #[schema(value_type = Option<BusinessCollectLinkConfig>)]
    pub pm_collect_link_config: Option<BusinessCollectLinkConfig>,
}

#[cfg(feature = "v1")]
impl MerchantAccountUpdate {
    pub fn get_primary_details_as_value(
        &self,
    ) -> CustomResult<Option<serde_json::Value>, errors::ParsingError> {
        self.primary_business_details
            .as_ref()
            .map(|primary_business_details| primary_business_details.encode_to_value())
            .transpose()
    }

    pub fn get_pm_link_config_as_value(
        &self,
    ) -> CustomResult<Option<serde_json::Value>, errors::ParsingError> {
        self.pm_collect_link_config
            .as_ref()
            .map(|pm_collect_link_config| pm_collect_link_config.encode_to_value())
            .transpose()
    }

    pub fn get_merchant_details_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.merchant_details
            .as_ref()
            .map(|merchant_details| merchant_details.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_metadata_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_webhook_details_as_value(
        &self,
    ) -> CustomResult<Option<serde_json::Value>, errors::ParsingError> {
        self.webhook_details
            .as_ref()
            .map(|webhook_details| webhook_details.encode_to_value())
            .transpose()
    }

    pub fn parse_routing_algorithm(&self) -> CustomResult<(), errors::ParsingError> {
        match self.routing_algorithm {
            Some(ref routing_algorithm) => {
                let _: routing::RoutingAlgorithm =
                    routing_algorithm.clone().parse_value("RoutingAlgorithm")?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    // Get the enable payment response hash as a boolean, where the default value is true
    pub fn get_enable_payment_response_hash(&self) -> bool {
        self.enable_payment_response_hash.unwrap_or(true)
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantAccountUpdate {
    /// Name of the Merchant Account
    #[schema(example = "NewAge Retailer")]
    pub merchant_name: Option<String>,

    /// Details about the merchant
    pub merchant_details: Option<MerchantDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v2")]
impl MerchantAccountUpdate {
    pub fn get_merchant_details_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.merchant_details
            .as_ref()
            .map(|merchant_details| merchant_details.encode_to_value().map(Secret::new))
            .transpose()
    }

    pub fn get_metadata_as_secret(
        &self,
    ) -> CustomResult<Option<pii::SecretSerdeValue>, errors::ParsingError> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.encode_to_value().map(Secret::new))
            .transpose()
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct MerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// Name of the Merchant Account
    #[schema(value_type = Option<String>,example = "NewAge Retailer")]
    pub merchant_name: OptionalEncryptableName,

    /// The URL to redirect after completion of the payment
    #[schema(max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Details about the merchant
    #[schema(value_type = Option<MerchantDetails>)]
    pub merchant_details: Option<Encryptable<pii::SecretSerdeValue>>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf", value_type = Option<String>)]
    pub parent_merchant_id: Option<id_type::MerchantId>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    #[schema(value_type = Vec<PrimaryBusinessDetails>)]
    pub primary_business_details: Vec<PrimaryBusinessDetails>,

    /// The frm routing algorithm to be used to process the incoming request from merchant to outgoing payment FRM.
    #[schema(value_type = Option<RoutingAlgorithm>, max_length = 255, example = r#"{"type": "single", "data": "stripe" }"#)]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The organization id merchant is associated with
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "org_q98uSGAYbjEwqs0mJwnz")]
    pub organization_id: id_type::OrganizationId,

    ///  A boolean value to indicate if the merchant has recon service is enabled or not, by default value is false
    pub is_recon_enabled: bool,

    /// The default profile that must be used for creating merchant accounts and payments
    #[schema(max_length = 64, value_type = Option<String>)]
    pub default_profile: Option<id_type::ProfileId>,

    /// Used to indicate the status of the recon module for a merchant account
    #[schema(value_type = ReconStatus, example = "not_requested")]
    pub recon_status: api_enums::ReconStatus,

    /// Default payment method collect link config
    #[schema(value_type = Option<BusinessCollectLinkConfig>)]
    pub pm_collect_link_config: Option<BusinessCollectLinkConfig>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct MerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub id: id_type::MerchantId,

    /// Name of the Merchant Account
    #[schema(value_type = String,example = "NewAge Retailer")]
    pub merchant_name: Secret<String>,

    /// Details about the merchant
    #[schema(value_type = Option<MerchantDetails>)]
    pub merchant_details: Option<Encryptable<pii::SecretSerdeValue>>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: String,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The id of the organization which the merchant is associated with
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "org_q98uSGAYbjEwqs0mJwnz")]
    pub organization_id: id_type::OrganizationId,

    /// Used to indicate the status of the recon module for a merchant account
    #[schema(value_type = ReconStatus, example = "not_requested")]
    pub recon_status: api_enums::ReconStatus,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantDetails {
    /// The merchant's primary contact name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Doe")]
    pub primary_contact_person: Option<Secret<String>>,

    /// The merchant's primary phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "999999999")]
    pub primary_phone: Option<Secret<String>>,

    /// The merchant's primary email address
    #[schema(value_type = Option<String>, max_length = 255, example = "johndoe@test.com")]
    pub primary_email: Option<pii::Email>,

    /// The merchant's secondary contact name
    #[schema(value_type = Option<String>, max_length= 255, example = "John Doe2")]
    pub secondary_contact_person: Option<Secret<String>>,

    /// The merchant's secondary phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "999999988")]
    pub secondary_phone: Option<Secret<String>>,

    /// The merchant's secondary email address
    #[schema(value_type = Option<String>, max_length = 255, example = "johndoe2@test.com")]
    pub secondary_email: Option<pii::Email>,

    /// The business website of the merchant
    #[schema(max_length = 255, example = "www.example.com")]
    pub website: Option<String>,

    /// A brief description about merchant's business
    #[schema(
        max_length = 255,
        example = "Online Retail with a wide selection of organic products for North America"
    )]
    pub about_business: Option<String>,

    /// The merchant's address details
    pub address: Option<AddressDetails>,
}
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrimaryBusinessDetails {
    #[schema(value_type = CountryAlpha2)]
    pub country: api_enums::CountryAlpha2,
    #[schema(example = "food")]
    pub business: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDetails {
    ///The version for Webhook
    #[schema(max_length = 255, max_length = 255, example = "1.0.2")]
    pub webhook_version: Option<String>,

    ///The user name for Webhook login
    #[schema(max_length = 255, max_length = 255, example = "ekart_retail")]
    pub webhook_username: Option<String>,

    ///The password for Webhook login
    #[schema(value_type = Option<String>, max_length = 255, example = "ekart@123")]
    pub webhook_password: Option<Secret<String>>,

    ///The url for the webhook endpoint
    #[schema(value_type = Option<String>, example = "www.ekart.com/webhooks")]
    pub webhook_url: Option<Secret<String>>,

    /// If this property is true, a webhook message is posted whenever a new payment is created
    #[schema(example = true)]
    pub payment_created_enabled: Option<bool>,

    /// If this property is true, a webhook message is posted whenever a payment is successful
    #[schema(example = true)]
    pub payment_succeeded_enabled: Option<bool>,

    /// If this property is true, a webhook message is posted whenever a payment fails
    #[schema(example = true)]
    pub payment_failed_enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MerchantAccountDeleteResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// If the connector is deleted or not
    #[schema(example = false)]
    pub deleted: bool,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MerchantId {
    pub merchant_id: id_type::MerchantId,
}

#[cfg(feature = "v1")]
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantConnectorId {
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    #[schema(value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}
#[cfg(feature = "v2")]
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantConnectorId {
    #[schema(value_type = String)]
    pub id: id_type::MerchantConnectorAccountId,
}

#[cfg(feature = "v2")]
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorCreate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: api_enums::Connector,

    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports, If not passed then if will take `connector_name`_`profile_name`. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(value_type = PaymentMethodsEnabled)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt
    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = Option<ConnectorStatus>, example = "inactive")]
    // By default the ConnectorStatus is Active
    pub status: Option<api_enums::ConnectorStatus>,

    /// In case the merchant needs to store any additional sensitive data
    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    #[schema(value_type = Option<ConnectorWalletDetails>)]
    pub connector_wallets_details: Option<ConnectorWalletDetails>,

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    #[schema(value_type = Option<MerchantConnectorAccountFeatureMetadata>)]
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v2")]
impl MerchantConnectorCreate {
    pub fn get_transaction_type(&self) -> api_enums::TransactionType {
        match self.connector_type {
            #[cfg(feature = "payouts")]
            api_enums::ConnectorType::PayoutProcessor => api_enums::TransactionType::Payout,
            _ => api_enums::TransactionType::Payment,
        }
    }

    pub fn get_frm_config_as_secret(&self) -> Option<Vec<Secret<serde_json::Value>>> {
        match self.frm_configs.as_ref() {
            Some(frm_value) => {
                let configs_for_frm_value: Vec<Secret<serde_json::Value>> = frm_value
                    .iter()
                    .map(|config| config.encode_to_value().map(Secret::new))
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                Some(configs_for_frm_value)
            }
            None => None,
        }
    }

    pub fn get_connector_label(&self, profile_name: String) -> String {
        match self.connector_label.clone() {
            Some(connector_label) => connector_label,
            None => format!("{}_{}", self.connector_name, profile_name),
        }
    }
}

#[cfg(feature = "v1")]
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorCreate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: api_enums::Connector,
    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(example = json!([
        {
            "payment_method": "wallet",
            "payment_method_types": [
                "upi_collect",
                "upi_intent"
            ],
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ],
            "payment_schemes": [
                "Discover",
                "Discover"
            ],
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// The business country to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    pub business_label: Option<String>,

    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = Option<String>)]
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = Option<ConnectorStatus>, example = "inactive")]
    pub status: Option<api_enums::ConnectorStatus>,

    /// In case the merchant needs to store any additional sensitive data
    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    #[schema(value_type = Option<ConnectorWalletDetails>)]
    pub connector_wallets_details: Option<ConnectorWalletDetails>,
}

#[cfg(feature = "v1")]
impl MerchantConnectorCreate {
    pub fn get_transaction_type(&self) -> api_enums::TransactionType {
        match self.connector_type {
            #[cfg(feature = "payouts")]
            api_enums::ConnectorType::PayoutProcessor => api_enums::TransactionType::Payout,
            _ => api_enums::TransactionType::Payment,
        }
    }

    pub fn get_frm_config_as_secret(&self) -> Option<Vec<Secret<serde_json::Value>>> {
        match self.frm_configs.as_ref() {
            Some(frm_value) => {
                let configs_for_frm_value: Vec<Secret<serde_json::Value>> = frm_value
                    .iter()
                    .map(|config| config.encode_to_value().map(Secret::new))
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                Some(configs_for_frm_value)
            }
            None => None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalMerchantData {
    OpenBankingRecipientData(MerchantRecipientData),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
/// Feature metadata for merchant connector account
pub struct MerchantConnectorAccountFeatureMetadata {
    /// Revenue recovery metadata for merchant connector account
    pub revenue_recovery: Option<RevenueRecoveryMetadata>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
/// Revenue recovery metadata for merchant connector account
pub struct RevenueRecoveryMetadata {
    /// The maximum number of retries allowed for an invoice. This limit is set by the merchant for each `billing connector`. Once this limit is reached, no further retries will be attempted.
    #[schema(value_type = u16, example = "15")]
    pub max_retry_count: u16,
    /// Maximum number of `billing connector` retries before revenue recovery can start executing retries.
    #[schema(value_type = u16, example = "10")]
    pub billing_connector_retry_threshold: u16,
    /// Billing account reference id is payment gateway id at billing connector end.
    /// Merchants need to provide a mapping between these merchant connector account and the corresponding account reference IDs for each `billing connector`.
    #[schema(value_type = u16, example = r#"{ "mca_vDSg5z6AxnisHq5dbJ6g": "stripe_123", "mca_vDSg5z6AumisHqh4x5m1": "adyen_123" }"#)]
    pub billing_account_reference: HashMap<id_type::MerchantConnectorAccountId, String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MerchantAccountData {
    Iban {
        #[schema(value_type= String)]
        iban: Secret<String>,
        name: String,
        #[schema(value_type= Option<String>)]
        #[serde(skip_serializing_if = "Option::is_none")]
        connector_recipient_id: Option<Secret<String>>,
    },
    Bacs {
        #[schema(value_type= String)]
        account_number: Secret<String>,
        #[schema(value_type= String)]
        sort_code: Secret<String>,
        name: String,
        #[schema(value_type= Option<String>)]
        #[serde(skip_serializing_if = "Option::is_none")]
        connector_recipient_id: Option<Secret<String>>,
    },
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MerchantRecipientData {
    #[schema(value_type= Option<String>)]
    ConnectorRecipientId(Secret<String>),
    #[schema(value_type= Option<String>)]
    WalletId(Secret<String>),
    AccountData(MerchantAccountData),
}

// Different patterns of authentication.
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    TemporaryAuth,
    HeaderKey {
        api_key: Secret<String>,
    },
    BodyKey {
        api_key: Secret<String>,
        key1: Secret<String>,
    },
    SignatureKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
    },
    MultiAuthKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
        key2: Secret<String>,
    },
    CurrencyAuthKey {
        auth_key_map: HashMap<common_enums::Currency, pii::SecretSerdeValue>,
    },
    CertificateAuth {
        certificate: Secret<String>,
        private_key: Secret<String>,
    },
    #[default]
    NoKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorWebhookDetails {
    #[schema(value_type = String, example = "12345678900987654321")]
    pub merchant_secret: Secret<String>,
    #[schema(value_type = String, example = "12345678900987654321")]
    pub additional_secret: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MerchantConnectorInfo {
    pub connector_label: String,
    #[schema(value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}

impl MerchantConnectorInfo {
    pub fn new(
        connector_label: String,
        merchant_connector_id: id_type::MerchantConnectorAccountId,
    ) -> Self {
        Self {
            connector_label,
            merchant_connector_id,
        }
    }
}

/// Response of creating a new Merchant Connector for the merchant account."
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: common_enums::connector_enums::Connector,

    /// A unique label to identify the connector account created under a profile
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the merchant connector account
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub id: id_type::MerchantConnectorAccountId,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(value_type = Vec<PaymentMethodsEnabled>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    /// pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt
    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: api_enums::ConnectorStatus,

    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    #[schema(value_type = Option<ConnectorWalletDetails>)]
    pub connector_wallets_details: Option<ConnectorWalletDetails>,

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    #[schema(value_type = Option<MerchantConnectorAccountFeatureMetadata>)]
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v2")]
impl MerchantConnectorResponse {
    pub fn to_merchant_connector_info(&self, connector_label: &String) -> MerchantConnectorInfo {
        MerchantConnectorInfo {
            connector_label: connector_label.to_string(),
            merchant_connector_id: self.id.clone(),
        }
    }
}

/// Response of creating a new Merchant Connector for the merchant account."
#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: String,

    /// A unique label to identify the connector account created under a profile
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the merchant connector account
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(example = json!([
        {
            "payment_method": "wallet",
            "payment_method_types": [
                "upi_collect",
                "upi_intent"
            ],
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ],
            "payment_schemes": [
                "Discover",
                "Discover"
            ],
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// The business country to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    ///The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "travel")]
    pub business_label: Option<String>,

    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: api_enums::ConnectorStatus,

    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    #[schema(value_type = Option<ConnectorWalletDetails>)]
    pub connector_wallets_details: Option<ConnectorWalletDetails>,
}

#[cfg(feature = "v1")]
impl MerchantConnectorResponse {
    pub fn to_merchant_connector_info(&self, connector_label: &String) -> MerchantConnectorInfo {
        MerchantConnectorInfo {
            connector_label: connector_label.to_string(),
            merchant_connector_id: self.merchant_connector_id.clone(),
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorListResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: String,

    /// A unique label to identify the connector account created under a profile
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the merchant connector account
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(example = json!([
        {
            "payment_method": "wallet",
            "payment_method_types": [
                "upi_collect",
                "upi_intent"
            ],
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ],
            "payment_schemes": [
                "Discover",
                "Discover"
            ],
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// The business country to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    ///The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "travel")]
    pub business_label: Option<String>,

    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: api_enums::ConnectorStatus,
}

#[cfg(feature = "v1")]
impl MerchantConnectorListResponse {
    pub fn to_merchant_connector_info(&self, connector_label: &String) -> MerchantConnectorInfo {
        MerchantConnectorInfo {
            connector_label: connector_label.to_string(),
            merchant_connector_id: self.merchant_connector_id.clone(),
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorListResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: common_enums::connector_enums::Connector,

    /// A unique label to identify the connector account created under a profile
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the merchant connector account
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub id: id_type::MerchantConnectorAccountId,

    /// Identifier for the profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64, value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(value_type = Vec<PaymentMethodsEnabled>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: api_enums::ConnectorStatus,
}

#[cfg(feature = "v2")]
impl MerchantConnectorListResponse {
    pub fn to_merchant_connector_info(&self, connector_label: &String) -> MerchantConnectorInfo {
        MerchantConnectorInfo {
            connector_label: connector_label.to_string(),
            merchant_connector_id: self.id.clone(),
        }
    }
}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorUpdate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(example = json!([
        {
            "payment_method": "wallet",
            "payment_method_types": [
                "upi_collect",
                "upi_intent"
            ],
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ],
            "payment_schemes": [
                "Discover",
                "Discover"
            ],
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt
    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: Option<api_enums::ConnectorStatus>,

    /// In case the merchant needs to store any additional sensitive data
    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    #[schema(value_type = Option<ConnectorWalletDetails>)]
    pub connector_wallets_details: Option<ConnectorWalletDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWalletDetails {
    /// This field contains the Apple Pay certificates and credentials for iOS and Web Apple Pay flow
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub apple_pay_combined: Option<pii::SecretSerdeValue>,
    /// This field is for our legacy Apple Pay flow that contains the Apple Pay certificates and credentials for only iOS Apple Pay flow
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub apple_pay: Option<pii::SecretSerdeValue>,
    /// This field contains the Samsung Pay certificates and credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub samsung_pay: Option<pii::SecretSerdeValue>,
    /// This field contains the Paze certificates and credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub paze: Option<pii::SecretSerdeValue>,
    /// This field contains the Google Pay certificates and credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub google_pay: Option<pii::SecretSerdeValue>,
}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorUpdate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports, If not passed then if will take `connector_name`_`profile_name`. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
    #[schema(value_type = Option<Vec<PaymentMethodsEnabled>>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt
    #[schema(value_type = Option<Object>)]
    pub pm_auth_config: Option<pii::SecretSerdeValue>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: Option<api_enums::ConnectorStatus>,

    /// The identifier for the Merchant Account
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: id_type::MerchantId,

    /// In case the merchant needs to store any additional sensitive data
    #[schema(value_type = Option<AdditionalMerchantData>)]
    pub additional_merchant_data: Option<AdditionalMerchantData>,

    /// The connector_wallets_details is used to store wallet details such as certificates and wallet credentials
    pub connector_wallets_details: Option<ConnectorWalletDetails>,

    /// Additional data that might be required by hyperswitch, to enable some specific features.
    #[schema(value_type = Option<MerchantConnectorAccountFeatureMetadata>)]
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v2")]
impl MerchantConnectorUpdate {
    pub fn get_frm_config_as_secret(&self) -> Option<Vec<Secret<serde_json::Value>>> {
        match self.frm_configs.as_ref() {
            Some(frm_value) => {
                let configs_for_frm_value: Vec<Secret<serde_json::Value>> = frm_value
                    .iter()
                    .map(|config| config.encode_to_value().map(Secret::new))
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                Some(configs_for_frm_value)
            }
            None => None,
        }
    }
}

///Details of FrmConfigs are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmConfigs {
    ///this is the connector that can be used for the payment
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub gateway: Option<api_enums::Connector>,
    ///payment methods that can be used in the payment
    pub payment_methods: Vec<FrmPaymentMethod>,
}

///Details of FrmPaymentMethod are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmPaymentMethod {
    ///payment methods(card, wallet, etc) that can be used in the payment
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: Option<common_enums::PaymentMethod>,
    ///payment method types(credit, debit) that can be used in the payment. This field is deprecated. It has not been removed to provide backward compatibility.
    pub payment_method_types: Option<Vec<FrmPaymentMethodType>>,
    ///frm flow type to be used, can be pre/post
    #[schema(value_type = Option<FrmPreferredFlowTypes>)]
    pub flow: Option<api_enums::FrmPreferredFlowTypes>,
}

///Details of FrmPaymentMethodType are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmPaymentMethodType {
    ///payment method types(credit, debit) that can be used in the payment
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    ///card networks(like visa mastercard) types that can be used in the payment
    #[schema(value_type = CardNetwork)]
    pub card_networks: Option<Vec<common_enums::CardNetwork>>,
    ///frm flow type to be used, can be pre/post
    #[schema(value_type = FrmPreferredFlowTypes)]
    pub flow: api_enums::FrmPreferredFlowTypes,
    ///action that the frm would take, in case fraud is detected
    #[schema(value_type = FrmAction)]
    pub action: api_enums::FrmAction,
}
/// Details of all the payment methods enabled for the connector for the given merchant account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodsEnabled {
    /// Type of payment method.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: common_enums::PaymentMethod,

    /// Subtype of payment method
    #[schema(value_type = Option<Vec<RequestPaymentMethodTypes>>,example = json!(["credit"]))]
    pub payment_method_types: Option<Vec<payment_methods::RequestPaymentMethodTypes>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
pub enum AcceptedCurrencies {
    #[schema(value_type = Vec<Currency>)]
    EnableOnly(Vec<api_enums::Currency>),
    #[schema(value_type = Vec<Currency>)]
    DisableOnly(Vec<api_enums::Currency>),
    AllAccepted,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
/// Object to filter the customer countries for which the payment method is displayed
pub enum AcceptedCountries {
    #[schema(value_type = Vec<CountryAlpha2>)]
    EnableOnly(Vec<api_enums::CountryAlpha2>),
    #[schema(value_type = Vec<CountryAlpha2>)]
    DisableOnly(Vec<api_enums::CountryAlpha2>),
    AllAccepted,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MerchantConnectorDeleteResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
    /// If the connector is deleted or not
    #[schema(example = false)]
    pub deleted: bool,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MerchantConnectorDeleteResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub id: id_type::MerchantConnectorAccountId,
    /// If the connector is deleted or not
    #[schema(example = false)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleKVResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MerchantKeyTransferRequest {
    /// Offset for merchant account
    #[schema(example = 32)]
    pub from: u32,
    /// Limit for merchant account
    #[schema(example = 32)]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransferKeyResponse {
    /// The identifier for the Merchant Account
    #[schema(example = 32)]
    pub total_transferred: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleKVRequest {
    #[serde(skip_deserializing)]
    #[schema(value_type = String)]
    pub merchant_id: id_type::MerchantId,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleAllKVRequest {
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleAllKVResponse {
    ///Total number of updated merchants
    #[schema(example = 20)]
    pub total_updated: usize,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

/// Merchant connector details used to make payments.
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MerchantConnectorDetailsWrap {
    /// Creds Identifier is to uniquely identify the credentials. Do not send any sensitive info, like encoded_data in this field. And do not send the string "null".
    pub creds_identifier: String,
    /// Merchant connector details type type. Base64 Encode the credentials and send it in  this type and send as a string.
    #[schema(value_type = Option<MerchantConnectorDetails>, example = r#"{
        "connector_account_details": {
            "auth_type": "HeaderKey",
            "api_key":"sk_test_xxxxxexamplexxxxxx12345"
        },
        "metadata": {
            "user_defined_field_1": "sample_1",
            "user_defined_field_2": "sample_2",
        },
    }"#)]
    pub encoded_data: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MerchantConnectorDetails {
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,
    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Deserialize, ToSchema, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCreate {
    /// The name of profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<u32>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<HashMap<String, String>>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    #[serde(default)]
    pub is_tax_connector_enabled: bool,

    /// Indicates if network tokenization is enabled or not.
    #[serde(default)]
    pub is_network_tokenization_enabled: bool,

    /// Indicates if is_auto_retries_enabled is enabled or not.
    pub is_auto_retries_enabled: Option<bool>,

    /// Maximum number of auto retries allowed for a payment
    pub max_auto_retries_enabled: Option<u8>,

    /// Bool indicating if extended authentication must be requested for all payments
    #[schema(value_type = Option<bool>)]
    pub always_request_extended_authorization: Option<AlwaysRequestExtendedAuthorization>,

    /// Indicates if click to pay is enabled or not.
    #[serde(default)]
    pub is_click_to_pay_enabled: bool,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[nutype::nutype(
    validate(greater_or_equal = MIN_ORDER_FULFILLMENT_EXPIRY, less_or_equal = MAX_ORDER_FULFILLMENT_EXPIRY),
    derive(Clone, Copy, Debug, Deserialize, Serialize)
)]
pub struct OrderFulfillmentTime(i64);

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Deserialize, ToSchema, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCreate {
    /// The name of profile
    #[schema(max_length = 64)]
    pub profile_name: String,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<common_utils::types::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(value_type = Option<u32>, example = 900)]
    pub order_fulfillment_time: Option<OrderFulfillmentTime>,

    /// Whether the order fulfillment time is calculated from the origin or the time of creating the payment, or confirming the payment
    #[schema(value_type = Option<OrderFulfillmentTimeOrigin>, example = "create")]
    pub order_fulfillment_time_origin: Option<api_enums::OrderFulfillmentTimeOrigin>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<HashMap<String, String>>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    #[serde(default)]
    pub is_tax_connector_enabled: bool,

    /// Indicates if network tokenization is enabled or not.
    #[serde(default)]
    pub is_network_tokenization_enabled: bool,

    /// Indicates if click to pay is enabled or not.
    #[schema(default = false, example = false)]
    #[serde(default)]
    pub is_click_to_pay_enabled: bool,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct ProfileResponse {
    /// The identifier for Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The identifier for profile. This must be used for creating merchant accounts, payments and payouts
    #[schema(max_length = 64, value_type = String, example = "pro_abcdefghijklmnopqrstuvwxyz")]
    pub profile_id: id_type::ProfileId,

    /// Name of the profile
    #[schema(max_length = 64)]
    pub profile_name: String,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<i64>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<i64>,

    /// Default Payment Link config for all payment links created under this profile
    #[schema(value_type = Option<BusinessPaymentLinkConfig>)]
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// Merchant's config to support extended card info feature
    pub extended_card_info_config: Option<ExtendedCardInfoConfig>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<MaskedHeaders>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    pub is_tax_connector_enabled: bool,

    /// Indicates if network tokenization is enabled or not.
    #[schema(default = false, example = false)]
    pub is_network_tokenization_enabled: bool,

    /// Indicates if is_auto_retries_enabled is enabled or not.
    #[schema(default = false, example = false)]
    pub is_auto_retries_enabled: bool,

    /// Maximum number of auto retries allowed for a payment
    pub max_auto_retries_enabled: Option<i16>,

    /// Bool indicating if extended authentication must be requested for all payments
    #[schema(value_type = Option<bool>)]
    pub always_request_extended_authorization: Option<AlwaysRequestExtendedAuthorization>,

    /// Indicates if click to pay is enabled or not.
    #[schema(default = false, example = false)]
    pub is_click_to_pay_enabled: bool,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct ProfileResponse {
    /// The identifier for Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The identifier for profile. This must be used for creating merchant accounts, payments and payouts
    #[schema(max_length = 64, value_type = String, example = "pro_abcdefghijklmnopqrstuvwxyz")]
    pub id: id_type::ProfileId,

    /// Name of the profile
    #[schema(max_length = 64)]
    pub profile_name: String,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<common_utils::types::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<i64>,

    /// Default Payment Link config for all payment links created under this profile
    #[schema(value_type = Option<BusinessPaymentLinkConfig>)]
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// Merchant's config to support extended card info feature
    pub extended_card_info_config: Option<ExtendedCardInfoConfig>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<MaskedHeaders>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(value_type = Option<u32>, example = 900)]
    pub order_fulfillment_time: Option<OrderFulfillmentTime>,

    /// Whether the order fulfillment time is calculated from the origin or the time of creating the payment, or confirming the payment
    #[schema(value_type = Option<OrderFulfillmentTimeOrigin>, example = "create")]
    pub order_fulfillment_time_origin: Option<api_enums::OrderFulfillmentTimeOrigin>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    pub is_tax_connector_enabled: bool,

    /// Indicates if network tokenization is enabled or not.
    #[schema(default = false, example = false)]
    pub is_network_tokenization_enabled: bool,

    /// Indicates if CVV should be collected during payment or not.
    pub should_collect_cvv_during_payment: bool,

    /// Indicates if click to pay is enabled or not.
    #[schema(default = false, example = false)]
    pub is_click_to_pay_enabled: bool,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileUpdate {
    /// The name of profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<u32>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Merchant's config to support extended card info feature
    pub extended_card_info_config: Option<ExtendedCardInfoConfig>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<HashMap<String, String>>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    pub is_tax_connector_enabled: Option<bool>,

    /// Indicates if dynamic routing is enabled or not.
    #[serde(default)]
    pub dynamic_routing_algorithm: Option<serde_json::Value>,

    /// Indicates if network tokenization is enabled or not.
    pub is_network_tokenization_enabled: Option<bool>,

    /// Indicates if is_auto_retries_enabled is enabled or not.
    pub is_auto_retries_enabled: Option<bool>,

    /// Maximum number of auto retries allowed for a payment
    pub max_auto_retries_enabled: Option<u8>,

    /// Indicates if click to pay is enabled or not.
    #[schema(default = false, example = false)]
    pub is_click_to_pay_enabled: Option<bool>,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileUpdate {
    /// The name of profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<common_utils::types::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Will be used to determine the time till which your payment will be active once the payment session starts
    #[schema(value_type = Option<u32>, example = 900)]
    pub order_fulfillment_time: Option<OrderFulfillmentTime>,

    /// Whether the order fulfillment time is calculated from the origin or the time of creating the payment, or confirming the payment
    #[schema(value_type = Option<OrderFulfillmentTimeOrigin>, example = "create")]
    pub order_fulfillment_time_origin: Option<api_enums::OrderFulfillmentTimeOrigin>,

    /// Verified Apple Pay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Merchant's config to support extended card info feature
    pub extended_card_info_config: Option<ExtendedCardInfoConfig>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_shipping_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    #[schema(default = false, example = false)]
    pub collect_billing_details_from_wallet_connector_if_required: Option<bool>,

    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,

    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    #[schema(default = false, example = false)]
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,

    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    pub is_connector_agnostic_mit_enabled: Option<bool>,

    /// Default payout link config
    #[schema(value_type = Option<BusinessPayoutLinkConfig>)]
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,

    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs.
    #[schema(value_type = Option<Object>, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub outgoing_webhook_custom_http_headers: Option<HashMap<String, String>>,

    /// Merchant Connector id to be stored for tax_calculator connector
    #[schema(value_type = Option<String>)]
    pub tax_connector_id: Option<id_type::MerchantConnectorAccountId>,

    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    pub is_tax_connector_enabled: Option<bool>,

    /// Indicates if network tokenization is enabled or not.
    pub is_network_tokenization_enabled: Option<bool>,

    /// Indicates if click to pay is enabled or not.
    #[schema(default = false, example = false)]
    pub is_click_to_pay_enabled: Option<bool>,

    /// Product authentication ids
    #[schema(value_type = Option<Object>, example = r#"{ "click_to_pay": "mca_ushduqwhdohwd", "netcetera": "mca_kwqhudqwd" }"#)]
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BusinessCollectLinkConfig {
    #[serde(flatten)]
    pub config: BusinessGenericLinkConfig,

    /// List of payment methods shown on collect UI
    #[schema(value_type = Vec<EnabledPaymentMethod>, example = r#"[{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs", "sepa"]}]"#)]
    pub enabled_payment_methods: Vec<link_utils::EnabledPaymentMethod>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BusinessPayoutLinkConfig {
    #[serde(flatten)]
    pub config: BusinessGenericLinkConfig,

    /// Form layout of the payout link
    #[schema(value_type = Option<UIWidgetFormLayout>, max_length = 255, example = "tabs")]
    pub form_layout: Option<api_enums::UIWidgetFormLayout>,

    /// Allows for removing any validations / pre-requisites which are necessary in a production environment
    #[schema(value_type = Option<bool>, default = false)]
    pub payout_test_mode: Option<bool>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MaskedHeaders(HashMap<String, String>);

impl MaskedHeaders {
    fn mask_value(value: &str) -> String {
        let value_len = value.len();

        let masked_value = if value_len <= 4 {
            "*".repeat(value_len)
        } else {
            value
                .char_indices()
                .map(|(index, ch)| {
                    if index < 2 || index >= value_len - 2 {
                        // Show the first two and last two characters, mask the rest with '*'
                        ch
                    } else {
                        // Mask the remaining characters
                        '*'
                    }
                })
                .collect::<String>()
        };

        masked_value
    }

    pub fn from_headers(headers: HashMap<String, Secret<String>>) -> Self {
        let masked_headers = headers
            .into_iter()
            .map(|(key, value)| (key, Self::mask_value(value.peek())))
            .collect();

        Self(masked_headers)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BusinessGenericLinkConfig {
    /// Custom domain name to be used for hosting the link
    pub domain_name: Option<String>,

    /// A list of allowed domains (glob patterns) where this link can be embedded / opened from
    pub allowed_domains: HashSet<String>,

    #[serde(flatten)]
    #[schema(value_type = GenericLinkUiConfig)]
    pub ui_config: link_utils::GenericLinkUiConfig,
}

impl BusinessGenericLinkConfig {
    pub fn validate(&self) -> Result<(), &str> {
        // Validate host domain name
        let host_domain_valid = self
            .domain_name
            .clone()
            .map(|host_domain| link_utils::validate_strict_domain(&host_domain))
            .unwrap_or(true);
        if !host_domain_valid {
            return Err("Invalid host domain name received in payout_link_config");
        }

        let are_allowed_domains_valid = self
            .allowed_domains
            .clone()
            .iter()
            .all(|allowed_domain| link_utils::validate_wildcard_domain(allowed_domain));
        if !are_allowed_domains_valid {
            return Err("Invalid allowed domain names received in payout_link_config");
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct BusinessPaymentLinkConfig {
    /// Custom domain name to be used for hosting the link in your own domain
    pub domain_name: Option<String>,
    /// Default payment link config for all future payment link
    #[serde(flatten)]
    #[schema(value_type = PaymentLinkConfigRequest)]
    pub default_config: Option<PaymentLinkConfigRequest>,
    /// list of configs for multi theme setup
    pub business_specific_configs: Option<HashMap<String, PaymentLinkConfigRequest>>,
    /// A list of allowed domains (glob patterns) where this link can be embedded / opened from
    #[schema(value_type = Option<HashSet<String>>)]
    pub allowed_domains: Option<HashSet<String>>,
    /// Toggle for HyperSwitch branding visibility
    pub branding_visibility: Option<bool>,
}

impl BusinessPaymentLinkConfig {
    pub fn validate(&self) -> Result<(), &str> {
        let host_domain_valid = self
            .domain_name
            .clone()
            .map(|host_domain| link_utils::validate_strict_domain(&host_domain))
            .unwrap_or(true);
        if !host_domain_valid {
            return Err("Invalid host domain name received in payment_link_config");
        }

        let are_allowed_domains_valid = self
            .allowed_domains
            .clone()
            .map(|allowed_domains| {
                allowed_domains
                    .iter()
                    .all(|allowed_domain| link_utils::validate_wildcard_domain(allowed_domain))
            })
            .unwrap_or(true);
        if !are_allowed_domains_valid {
            return Err("Invalid allowed domain names received in payment_link_config");
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkConfigRequest {
    /// custom theme for the payment link
    #[schema(value_type = Option<String>, max_length = 255, example = "#4E6ADD")]
    pub theme: Option<String>,
    /// merchant display logo
    #[schema(value_type = Option<String>, max_length = 255, example = "https://i.pinimg.com/736x/4d/83/5c/4d835ca8aafbbb15f84d07d926fda473.jpg")]
    pub logo: Option<String>,
    /// Custom merchant name for payment link
    #[schema(value_type = Option<String>, max_length = 255, example = "hyperswitch")]
    pub seller_name: Option<String>,
    /// Custom layout for sdk
    #[schema(value_type = Option<String>, max_length = 255, example = "accordion")]
    pub sdk_layout: Option<String>,
    /// Display only the sdk for payment link
    #[schema(default = false, example = true)]
    pub display_sdk_only: Option<bool>,
    /// Enable saved payment method option for payment link
    #[schema(default = false, example = true)]
    pub enabled_saved_payment_method: Option<bool>,
    /// Hide card nickname field option for payment link
    #[schema(default = false, example = true)]
    pub hide_card_nickname_field: Option<bool>,
    /// Show card form by default for payment link
    #[schema(default = true, example = true)]
    pub show_card_form_by_default: Option<bool>,
    /// Dynamic details related to merchant to be rendered in payment link
    pub transaction_details: Option<Vec<PaymentLinkTransactionDetails>>,
    /// Configurations for the background image for details section
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    /// Custom layout for details section
    #[schema(value_type = Option<PaymentLinkDetailsLayout>, example = "layout1")]
    pub details_layout: Option<api_enums::PaymentLinkDetailsLayout>,
    /// Text for payment link's handle confirm button
    pub payment_button_text: Option<String>,
    /// Text for customizing message for card terms
    pub custom_message_for_card_terms: Option<String>,
    /// Custom background colour for payment link's handle confirm button
    pub payment_button_colour: Option<String>,
    /// Display the status screen after payment completion
    pub display_status_screen: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkTransactionDetails {
    /// Key for the transaction details
    #[schema(value_type = String, max_length = 255, example = "Policy-Number")]
    pub key: String,
    /// Value for the transaction details
    #[schema(value_type = String, max_length = 255, example = "297472368473924")]
    pub value: String,
    /// UI configuration for the transaction details
    pub ui_configuration: Option<TransactionDetailsUiConfiguration>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct TransactionDetailsUiConfiguration {
    /// Position of the key-value pair in the UI
    #[schema(value_type = Option<i8>, example = 5)]
    pub position: Option<i8>,
    /// Whether the key should be bold
    #[schema(default = false, example = true)]
    pub is_key_bold: Option<bool>,
    /// Whether the value should be bold
    #[schema(default = false, example = true)]
    pub is_value_bold: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkBackgroundImageConfig {
    /// URL of the image
    #[schema(value_type = String, example = "https://hyperswitch.io/favicon.ico")]
    pub url: common_utils::types::Url,
    /// Position of the image in the UI
    #[schema(value_type = Option<ElementPosition>, example = "top-left")]
    pub position: Option<api_enums::ElementPosition>,
    /// Size of the image in the UI
    #[schema(value_type = Option<ElementSize>, example = "contain")]
    pub size: Option<api_enums::ElementSize>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, ToSchema)]
pub struct PaymentLinkConfig {
    /// custom theme for the payment link
    pub theme: String,
    /// merchant display logo
    pub logo: String,
    /// Custom merchant name for payment link
    pub seller_name: String,
    /// Custom layout for sdk
    pub sdk_layout: String,
    /// Display only the sdk for payment link
    pub display_sdk_only: bool,
    /// Enable saved payment method option for payment link
    pub enabled_saved_payment_method: bool,
    /// Hide card nickname field option for payment link
    pub hide_card_nickname_field: bool,
    /// Show card form by default for payment link
    pub show_card_form_by_default: bool,
    /// A list of allowed domains (glob patterns) where this link can be embedded / opened from
    pub allowed_domains: Option<HashSet<String>>,
    /// Dynamic details related to merchant to be rendered in payment link
    pub transaction_details: Option<Vec<PaymentLinkTransactionDetails>>,
    /// Configurations for the background image for details section
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    /// Custom layout for details section
    #[schema(value_type = Option<PaymentLinkDetailsLayout>, example = "layout1")]
    pub details_layout: Option<api_enums::PaymentLinkDetailsLayout>,
    /// Toggle for HyperSwitch branding visibility
    pub branding_visibility: Option<bool>,
    /// Text for payment link's handle confirm button
    pub payment_button_text: Option<String>,
    /// Text for customizing message for card terms
    pub custom_message_for_card_terms: Option<String>,
    /// Custom background colour for payment link's handle confirm button
    pub payment_button_colour: Option<String>,
    /// Display the status screen after payment completion
    pub display_status_screen: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExtendedCardInfoChoice {
    pub enabled: bool,
}

impl common_utils::events::ApiEventMetric for ExtendedCardInfoChoice {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ConnectorAgnosticMitChoice {
    pub enabled: bool,
}

impl common_utils::events::ApiEventMetric for ConnectorAgnosticMitChoice {}

impl common_utils::events::ApiEventMetric for payment_methods::PaymentMethodMigrate {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ExtendedCardInfoConfig {
    /// Merchant public key
    #[schema(value_type = String)]
    pub public_key: Secret<String>,
    /// TTL for extended card info
    #[schema(default = 900, maximum = 7200, value_type = u16)]
    #[serde(default)]
    pub ttl_in_secs: TtlForExtendedCardInfo,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct TtlForExtendedCardInfo(u16);

impl Default for TtlForExtendedCardInfo {
    fn default() -> Self {
        Self(consts::DEFAULT_TTL_FOR_EXTENDED_CARD_INFO)
    }
}

impl<'de> Deserialize<'de> for TtlForExtendedCardInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;

        // Check if value exceeds the maximum allowed
        if value > consts::MAX_TTL_FOR_EXTENDED_CARD_INFO {
            Err(serde::de::Error::custom(
                "ttl_in_secs must be less than or equal to 7200 (2hrs)",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

impl std::ops::Deref for TtlForExtendedCardInfo {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
