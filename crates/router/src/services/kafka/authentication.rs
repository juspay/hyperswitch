use api_models::payments::DeviceChannel;
use common_enums::MerchantCategoryCode;
use diesel_models::enums as storage_enums;
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaAuthentication<'a> {
    pub authentication_id: &'a common_utils::id_type::AuthenticationId,
    pub merchant_id: &'a common_utils::id_type::MerchantId,
    pub authentication_connector: Option<&'a String>,
    pub connector_authentication_id: Option<&'a String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: &'a String,
    pub authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
    pub authentication_status: storage_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: storage_enums::AuthenticationLifecycleStatus,
    #[serde(default, with = "time::serde::timestamp::milliseconds")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::milliseconds")]
    pub modified_at: OffsetDateTime,
    pub error_message: Option<&'a String>,
    pub error_code: Option<&'a String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<&'a String>,
    pub cavv: Option<&'a String>,
    pub authentication_flow_type: Option<&'a String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<&'a String>,
    pub trans_status: Option<storage_enums::TransactionStatus>,
    pub acquirer_bin: Option<&'a String>,
    pub acquirer_merchant_id: Option<&'a String>,
    pub three_ds_method_data: Option<&'a String>,
    pub three_ds_method_url: Option<&'a String>,
    pub acs_url: Option<&'a String>,
    pub challenge_request: Option<&'a String>,
    pub acs_reference_number: Option<&'a String>,
    pub acs_trans_id: Option<&'a String>,
    pub acs_signed_content: Option<&'a String>,
    pub profile_id: &'a common_utils::id_type::ProfileId,
    pub payment_id: Option<&'a common_utils::id_type::PaymentId>,
    pub merchant_connector_id: Option<&'a common_utils::id_type::MerchantConnectorAccountId>,
    pub ds_trans_id: Option<&'a String>,
    pub directory_server_id: Option<&'a String>,
    pub acquirer_country_code: Option<&'a String>,
    pub organization_id: &'a common_utils::id_type::OrganizationId,
    pub platform: Option<&'a DeviceChannel>,
    pub mcc: Option<&'a MerchantCategoryCode>,
    pub currency: Option<&'a common_enums::Currency>,
    pub merchant_country: Option<&'a String>,
    pub billing_country: Option<storage_enums::CountryAlpha2>,
    pub shipping_country: Option<storage_enums::CountryAlpha2>,
    pub issuer_country: Option<&'a String>,
    pub earliest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub latest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub device_type: Option<&'a String>,
    pub device_brand: Option<&'a String>,
    pub device_os: Option<&'a String>,
    pub device_display: Option<&'a String>,
    pub browser_name: Option<&'a String>,
    pub browser_version: Option<&'a String>,
    pub issuer_id: Option<&'a String>,
    pub scheme_name: Option<&'a String>,
    pub exemption_requested: Option<bool>,
    pub exemption_accepted: Option<bool>,
}

impl<'a> KafkaAuthentication<'a> {
    pub fn from_storage(
        authentication: &'a hyperswitch_domain_models::authentication::Authentication,
    ) -> Self {
        Self {
            created_at: authentication.created_at.assume_utc(),
            modified_at: authentication.modified_at.assume_utc(),
            authentication_id: &authentication.authentication_id,
            merchant_id: &authentication.merchant_id,
            authentication_status: authentication.authentication_status,
            authentication_connector: authentication.authentication_connector.as_ref(),
            connector_authentication_id: authentication.connector_authentication_id.as_ref(),
            authentication_data: authentication.authentication_data.clone(),
            payment_method_id: &authentication.payment_method_id,
            authentication_type: authentication.authentication_type,
            authentication_lifecycle_status: authentication.authentication_lifecycle_status,
            error_code: authentication.error_code.as_ref(),
            error_message: authentication.error_message.as_ref(),
            connector_metadata: authentication.connector_metadata.clone(),
            maximum_supported_version: authentication.maximum_supported_version.clone(),
            threeds_server_transaction_id: authentication.threeds_server_transaction_id.as_ref(),
            cavv: authentication.cavv.as_ref(),
            authentication_flow_type: authentication.authentication_flow_type.as_ref(),
            message_version: authentication.message_version.clone(),
            eci: authentication.eci.as_ref(),
            trans_status: authentication.trans_status.clone(),
            acquirer_bin: authentication.acquirer_bin.as_ref(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.as_ref(),
            three_ds_method_data: authentication.three_ds_method_data.as_ref(),
            three_ds_method_url: authentication.three_ds_method_url.as_ref(),
            acs_url: authentication.acs_url.as_ref(),
            challenge_request: authentication.challenge_request.as_ref(),
            acs_reference_number: authentication.acs_reference_number.as_ref(),
            acs_trans_id: authentication.acs_trans_id.as_ref(),
            acs_signed_content: authentication.acs_signed_content.as_ref(),
            profile_id: &authentication.profile_id,
            payment_id: authentication.payment_id.as_ref(),
            merchant_connector_id: authentication.merchant_connector_id.as_ref(),
            ds_trans_id: authentication.ds_trans_id.as_ref(),
            directory_server_id: authentication.directory_server_id.as_ref(),
            acquirer_country_code: authentication.acquirer_country_code.as_ref(),
            organization_id: &authentication.organization_id,
            platform: authentication.platform.as_ref(),
            mcc: authentication.mcc.as_ref(),
            currency: authentication.currency.as_ref(),
            merchant_country: authentication.merchant_country_code.as_ref(),
            billing_country: authentication.billing_country,
            shipping_country: authentication.shipping_country,
            issuer_country: authentication.issuer_country.as_ref(),
            earliest_supported_version: authentication.earliest_supported_version.clone(),
            latest_supported_version: authentication.latest_supported_version.clone(),
            device_type: authentication.device_type.as_ref(),
            device_brand: authentication.device_brand.as_ref(),
            device_os: authentication.device_os.as_ref(),
            device_display: authentication.device_display.as_ref(),
            browser_name: authentication.browser_name.as_ref(),
            browser_version: authentication.browser_version.as_ref(),
            issuer_id: authentication.issuer_id.as_ref(),
            scheme_name: authentication.scheme_name.as_ref(),
            exemption_requested: authentication.exemption_requested,
            exemption_accepted: authentication.exemption_accepted,
        }
    }
}

impl super::KafkaMessage for KafkaAuthentication<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}",
            self.merchant_id.get_string_repr(),
            self.authentication_id.get_string_repr()
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Authentication
    }
}
