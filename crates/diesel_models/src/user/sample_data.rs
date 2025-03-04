use common_enums::{
    AttemptStatus, AuthenticationType, CaptureMethod, Currency, PaymentExperience, PaymentMethod,
    PaymentMethodType,
};
use common_utils::types::{
    ConnectorTransactionId, ExtendedAuthorizationAppliedBool, MinorUnit,
    RequestExtendedAuthorizationBool,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(feature = "v1")]
use crate::schema::payment_attempt;
#[cfg(feature = "v2")]
use crate::schema_v2::payment_attempt;
use crate::{
    enums::{MandateDataType, MandateDetails},
    ConnectorMandateReferenceId, PaymentAttemptNew,
};

// #[cfg(feature = "v2")]
// #[derive(
//     Clone, Debug, diesel::Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
// )]
// #[diesel(table_name = payment_attempt)]
// pub struct PaymentAttemptBatchNew {
//     pub payment_id: common_utils::id_type::PaymentId,
//     pub merchant_id: common_utils::id_type::MerchantId,
//     pub status: AttemptStatus,
//     pub error_message: Option<String>,
//     pub surcharge_amount: Option<i64>,
//     pub tax_on_surcharge: Option<i64>,
//     pub payment_method_id: Option<String>,
//     pub authentication_type: Option<AuthenticationType>,
//     #[serde(with = "common_utils::custom_serde::iso8601")]
//     pub created_at: PrimitiveDateTime,
//     #[serde(with = "common_utils::custom_serde::iso8601")]
//     pub modified_at: PrimitiveDateTime,
//     #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
//     pub last_synced: Option<PrimitiveDateTime>,
//     pub cancellation_reason: Option<String>,
//     pub browser_info: Option<serde_json::Value>,
//     pub payment_token: Option<String>,
//     pub error_code: Option<String>,
//     pub connector_metadata: Option<serde_json::Value>,
//     pub payment_experience: Option<PaymentExperience>,
//     pub payment_method_data: Option<serde_json::Value>,
//     pub preprocessing_step_id: Option<String>,
//     pub error_reason: Option<String>,
//     pub connector_response_reference_id: Option<String>,
//     pub multiple_capture_count: Option<i16>,
//     pub amount_capturable: i64,
//     pub updated_by: String,
//     pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
//     pub authentication_data: Option<serde_json::Value>,
//     pub encoded_data: Option<String>,
//     pub unified_code: Option<String>,
//     pub unified_message: Option<String>,
//     pub net_amount: Option<i64>,
//     pub external_three_ds_authentication_attempted: Option<bool>,
//     pub authentication_connector: Option<String>,
//     pub authentication_id: Option<String>,
//     pub fingerprint_id: Option<String>,
//     pub charge_id: Option<String>,
//     pub client_source: Option<String>,
//     pub client_version: Option<String>,
//     pub customer_acceptance: Option<common_utils::pii::SecretSerdeValue>,
//     pub profile_id: common_utils::id_type::ProfileId,
//     pub organization_id: common_utils::id_type::OrganizationId,
// }

// #[cfg(feature = "v2")]
// #[allow(dead_code)]
// impl PaymentAttemptBatchNew {
//     // Used to verify compatibility with PaymentAttemptTable
//     fn convert_into_normal_attempt_insert(self) -> PaymentAttemptNew {
//         // PaymentAttemptNew {
//         //     payment_id: self.payment_id,
//         //     merchant_id: self.merchant_id,
//         //     status: self.status,
//         //     error_message: self.error_message,
//         //     surcharge_amount: self.surcharge_amount,
//         //     tax_amount: self.tax_amount,
//         //     payment_method_id: self.payment_method_id,
//         //     confirm: self.confirm,
//         //     authentication_type: self.authentication_type,
//         //     created_at: self.created_at,
//         //     modified_at: self.modified_at,
//         //     last_synced: self.last_synced,
//         //     cancellation_reason: self.cancellation_reason,
//         //     browser_info: self.browser_info,
//         //     payment_token: self.payment_token,
//         //     error_code: self.error_code,
//         //     connector_metadata: self.connector_metadata,
//         //     payment_experience: self.payment_experience,
//         //     card_network: self
//         //         .payment_method_data
//         //         .as_ref()
//         //         .and_then(|data| data.as_object())
//         //         .and_then(|card| card.get("card"))
//         //         .and_then(|v| v.as_object())
//         //         .and_then(|v| v.get("card_network"))
//         //         .and_then(|network| network.as_str())
//         //         .map(|network| network.to_string()),
//         //     payment_method_data: self.payment_method_data,
//         //     straight_through_algorithm: self.straight_through_algorithm,
//         //     preprocessing_step_id: self.preprocessing_step_id,
//         //     error_reason: self.error_reason,
//         //     multiple_capture_count: self.multiple_capture_count,
//         //     connector_response_reference_id: self.connector_response_reference_id,
//         //     amount_capturable: self.amount_capturable,
//         //     updated_by: self.updated_by,
//         //     merchant_connector_id: self.merchant_connector_id,
//         //     authentication_data: self.authentication_data,
//         //     encoded_data: self.encoded_data,
//         //     unified_code: self.unified_code,
//         //     unified_message: self.unified_message,
//         //     net_amount: self.net_amount,
//         //     external_three_ds_authentication_attempted: self
//         //         .external_three_ds_authentication_attempted,
//         //     authentication_connector: self.authentication_connector,
//         //     authentication_id: self.authentication_id,
//         //     payment_method_billing_address_id: self.payment_method_billing_address_id,
//         //     fingerprint_id: self.fingerprint_id,
//         //     charge_id: self.charge_id,
//         //     client_source: self.client_source,
//         //     client_version: self.client_version,
//         //     customer_acceptance: self.customer_acceptance,
//         //     profile_id: self.profile_id,
//         //     organization_id: self.organization_id,
//         // }
//         todo!()
//     }
// }

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, diesel::Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_attempt)]
pub struct PaymentAttemptBatchNew {
    pub payment_id: common_utils::id_type::PaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub attempt_id: String,
    pub status: AttemptStatus,
    pub amount: MinorUnit,
    pub currency: Option<Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<MinorUnit>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<PaymentMethod>,
    pub capture_method: Option<CaptureMethod>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<AuthenticationType>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<MinorUnit>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub payment_experience: Option<PaymentExperience>,
    pub payment_method_type: Option<PaymentMethodType>,
    pub payment_method_data: Option<serde_json::Value>,
    pub business_sub_label: Option<String>,
    pub straight_through_algorithm: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
    pub mandate_details: Option<MandateDataType>,
    pub error_reason: Option<String>,
    pub connector_response_reference_id: Option<String>,
    pub connector_transaction_id: Option<ConnectorTransactionId>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: MinorUnit,
    pub updated_by: String,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub net_amount: Option<MinorUnit>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub mandate_data: Option<MandateDetails>,
    pub payment_method_billing_address_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<common_utils::pii::SecretSerdeValue>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
    pub processor_transaction_data: Option<String>,
    pub connector_mandate_detail: Option<ConnectorMandateReferenceId>,
    pub request_extended_authorization: Option<RequestExtendedAuthorizationBool>,
    pub extended_authorization_applied: Option<ExtendedAuthorizationAppliedBool>,
    pub capture_before: Option<PrimitiveDateTime>,
    pub card_discovery: Option<common_enums::CardDiscovery>,
}

#[cfg(feature = "v1")]
#[allow(dead_code)]
impl PaymentAttemptBatchNew {
    // Used to verify compatibility with PaymentAttemptTable
    fn convert_into_normal_attempt_insert(self) -> PaymentAttemptNew {
        PaymentAttemptNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            card_network: self
                .payment_method_data
                .as_ref()
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card"))
                .and_then(|v| v.as_object())
                .and_then(|v| v.get("card_network"))
                .and_then(|network| network.as_str())
                .map(|network| network.to_string()),
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details,
            error_reason: self.error_reason,
            multiple_capture_count: self.multiple_capture_count,
            connector_response_reference_id: self.connector_response_reference_id,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            net_amount: self.net_amount,
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data,
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            fingerprint_id: self.fingerprint_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            order_tax_amount: self.order_tax_amount,
            connector_mandate_detail: self.connector_mandate_detail,
            request_extended_authorization: self.request_extended_authorization,
            extended_authorization_applied: self.extended_authorization_applied,
            capture_before: self.capture_before,
            card_discovery: self.card_discovery,
        }
    }
}
