//! Conversion implementations for Authentication

use std::str::FromStr;

use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::keymanager::{self, Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    authentication::{Authentication, EncryptedAuthentication, FromRequestEncryptableAuthentication},
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;
use crate::transformers::ForeignFrom;

impl ForeignFrom<Box<Option<Encryptable<Secret<serde_json::Value>>>>> for Option<Encryption> {
    fn foreign_from(from: Box<Option<Encryptable<Secret<serde_json::Value>>>>) -> Self {
        (*from).map(Encryption::from)
    }
}

impl<T: Clone> ForeignFrom<Encryptable<T>> for Encryption
where
    Encryption: From<Encryptable<T>>,
{
    fn foreign_from(from: Encryptable<T>) -> Self {
        Self::from(from)
    }
}

#[async_trait::async_trait]
impl Conversion for Authentication {
    type DstType = diesel_models::authentication::Authentication;
    type NewDstType = diesel_models::authentication::AuthenticationNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            authentication_id: self.authentication_id,
            merchant_id: self.merchant_id,
            authentication_connector: self.authentication_connector,
            connector_authentication_id: self.connector_authentication_id,
            authentication_data: self.authentication_data,
            payment_method_id: self.payment_method_id,
            authentication_type: self.authentication_type,
            authentication_status: self.authentication_status,
            authentication_lifecycle_status: self.authentication_lifecycle_status,
            created_at: self.created_at,
            modified_at: self.modified_at,
            error_message: self.error_message,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            maximum_supported_version: self.maximum_supported_version,
            threeds_server_transaction_id: self.threeds_server_transaction_id,
            cavv: self.cavv,
            authentication_flow_type: self.authentication_flow_type,
            message_version: self.message_version,
            eci: self.eci,
            trans_status: self.trans_status,
            acquirer_bin: self.acquirer_bin,
            acquirer_merchant_id: self.acquirer_merchant_id,
            three_ds_method_data: self.three_ds_method_data,
            three_ds_method_url: self.three_ds_method_url,
            acs_url: self.acs_url,
            challenge_request: self.challenge_request,
            acs_reference_number: self.acs_reference_number,
            acs_trans_id: self.acs_trans_id,
            acs_signed_content: self.acs_signed_content,
            profile_id: self.profile_id,
            payment_id: self.payment_id,
            merchant_connector_id: self.merchant_connector_id,
            ds_trans_id: self.ds_trans_id,
            directory_server_id: self.directory_server_id,
            acquirer_country_code: self.acquirer_country_code,
            service_details: self.service_details,
            organization_id: self.organization_id,
            authentication_client_secret: self.authentication_client_secret,
            force_3ds_challenge: self.force_3ds_challenge,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            return_url: self.return_url,
            amount: self.amount,
            currency: self.currency,
            billing_address: self.billing_address.map(Encryption::from),
            shipping_address: self.shipping_address.map(Encryption::from),
            browser_info: self.browser_info,
            email: self.email.map(|email| email.into()),
            profile_acquirer_id: self.profile_acquirer_id,
            challenge_code: self.challenge_code,
            challenge_cancel: self.challenge_cancel,
            challenge_code_reason: self.challenge_code_reason,
            message_extension: self.message_extension,
            challenge_request_key: self.challenge_request_key,
            customer_details: self.customer_details,
            earliest_supported_version: self.earliest_supported_version,
            latest_supported_version: self.latest_supported_version,
            mcc: self.mcc,
            platform: self.platform.map(|platform| platform.to_string()),
            device_type: self.device_type,
            device_brand: self.device_brand,
            device_os: self.device_os,
            device_display: self.device_display,
            browser_name: self.browser_name,
            browser_version: self.browser_version,
            scheme_name: self.scheme_name,
            exemption_requested: self.exemption_requested,
            exemption_accepted: self.exemption_accepted,
            issuer_id: self.issuer_id,
            issuer_country: self.issuer_country,
            merchant_country_code: self.merchant_country_code,
            billing_country: self.billing_country,
            shipping_country: self.shipping_country,
            processor_merchant_id: self.processor_merchant_id,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let encrypted_data = crypto_operation(
            state,
            type_name!(Self),
            CryptoOperation::BatchDecrypt(EncryptedAuthentication::to_encryptable(
                EncryptedAuthentication {
                    billing_address: other.billing_address,
                    shipping_address: other.shipping_address,
                },
            )),
            Identifier::Merchant(other.merchant_id.clone()),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting authentication data".to_string(),
        })?;

        let decrypted_data = FromRequestEncryptableAuthentication::from_encryptable(encrypted_data)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting authentication data".to_string(),
            })?;

        let email_decrypted = other
            .email
            .clone()
            .async_lift(|inner| async {
                crypto_operation::<String, common_utils::pii::EmailStrategy>(
                    state,
                    type_name!(Self),
                    CryptoOperation::DecryptOptional(inner),
                    Identifier::Merchant(other.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting authentication email".to_string(),
            })?;

        Ok(Self {
            authentication_id: other.authentication_id,
            merchant_id: other.merchant_id,
            authentication_connector: other.authentication_connector,
            connector_authentication_id: other.connector_authentication_id,
            authentication_data: other.authentication_data,
            payment_method_id: other.payment_method_id,
            authentication_type: other.authentication_type,
            authentication_status: other.authentication_status,
            authentication_lifecycle_status: other.authentication_lifecycle_status,
            created_at: other.created_at,
            modified_at: other.modified_at,
            error_message: other.error_message,
            error_code: other.error_code,
            connector_metadata: other.connector_metadata,
            maximum_supported_version: other.maximum_supported_version,
            threeds_server_transaction_id: other.threeds_server_transaction_id,
            cavv: other.cavv,
            authentication_flow_type: other.authentication_flow_type,
            message_version: other.message_version,
            eci: other.eci,
            trans_status: other.trans_status,
            acquirer_bin: other.acquirer_bin,
            acquirer_merchant_id: other.acquirer_merchant_id,
            three_ds_method_data: other.three_ds_method_data,
            three_ds_method_url: other.three_ds_method_url,
            acs_url: other.acs_url,
            challenge_request: other.challenge_request,
            acs_reference_number: other.acs_reference_number,
            acs_trans_id: other.acs_trans_id,
            acs_signed_content: other.acs_signed_content,
            profile_id: other.profile_id,
            payment_id: other.payment_id,
            merchant_connector_id: other.merchant_connector_id,
            ds_trans_id: other.ds_trans_id,
            directory_server_id: other.directory_server_id,
            acquirer_country_code: other.acquirer_country_code,
            organization_id: other.organization_id,
            mcc: other.mcc,
            amount: other.amount,
            currency: other.currency,
            issuer_country: other.issuer_country,
            earliest_supported_version: other.earliest_supported_version,
            latest_supported_version: other.latest_supported_version,
            platform: other
                .platform
                .as_deref()
                .map(|s| {
                    api_models::payments::DeviceChannel::from_str(s).change_context(
                        ValidationError::InvalidValue {
                            message: "Invalid device channel".into(),
                        },
                    )
                })
                .transpose()?,
            device_type: other.device_type,
            device_brand: other.device_brand,
            device_os: other.device_os,
            device_display: other.device_display,
            browser_name: other.browser_name,
            browser_version: other.browser_version,
            issuer_id: other.issuer_id,
            scheme_name: other.scheme_name,
            exemption_requested: other.exemption_requested,
            exemption_accepted: other.exemption_accepted,
            service_details: other.service_details,
            authentication_client_secret: other.authentication_client_secret,
            force_3ds_challenge: other.force_3ds_challenge,
            psd2_sca_exemption_type: other.psd2_sca_exemption_type,
            return_url: other.return_url,
            billing_address: decrypted_data.billing_address,
            shipping_address: decrypted_data.shipping_address,
            browser_info: other.browser_info,
            email: email_decrypted,
            profile_acquirer_id: other.profile_acquirer_id,
            challenge_code: other.challenge_code,
            challenge_cancel: other.challenge_cancel,
            challenge_code_reason: other.challenge_code_reason,
            message_extension: other.message_extension,
            challenge_request_key: other.challenge_request_key,
            customer_details: other.customer_details,
            billing_country: other.billing_country,
            shipping_country: other.shipping_country,
            merchant_country_code: other.merchant_country_code,
            processor_merchant_id: other.processor_merchant_id,
            created_by: other
                .created_by
                .and_then(|created_by| created_by.parse::<common_utils::types::CreatedBy>().ok()),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            authentication_id: self.authentication_id,
            merchant_id: self.merchant_id,
            authentication_connector: self.authentication_connector,
            connector_authentication_id: self.connector_authentication_id,
            payment_method_id: self.payment_method_id,
            authentication_type: self.authentication_type,
            authentication_status: self.authentication_status,
            authentication_lifecycle_status: self.authentication_lifecycle_status,
            error_message: self.error_message,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            maximum_supported_version: self.maximum_supported_version,
            threeds_server_transaction_id: self.threeds_server_transaction_id,
            cavv: self.cavv,
            authentication_flow_type: self.authentication_flow_type,
            message_version: self.message_version,
            eci: self.eci,
            trans_status: self.trans_status,
            acquirer_bin: self.acquirer_bin,
            acquirer_merchant_id: self.acquirer_merchant_id,
            three_ds_method_data: self.three_ds_method_data,
            three_ds_method_url: self.three_ds_method_url,
            acs_url: self.acs_url,
            challenge_request: self.challenge_request,
            acs_reference_number: self.acs_reference_number,
            acs_trans_id: self.acs_trans_id,
            acs_signed_content: self.acs_signed_content,
            profile_id: self.profile_id,
            payment_id: self.payment_id,
            merchant_connector_id: self.merchant_connector_id,
            ds_trans_id: self.ds_trans_id,
            directory_server_id: self.directory_server_id,
            acquirer_country_code: self.acquirer_country_code,
            service_details: self.service_details,
            organization_id: self.organization_id,
            authentication_client_secret: self.authentication_client_secret,
            force_3ds_challenge: self.force_3ds_challenge,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            return_url: self.return_url,
            amount: self.amount,
            currency: self.currency,
            billing_address: self.billing_address.map(Encryption::from),
            shipping_address: self.shipping_address.map(Encryption::from),
            browser_info: self.browser_info,
            email: self.email.map(|email| email.into()),
            profile_acquirer_id: self.profile_acquirer_id,
            challenge_code: self.challenge_code,
            challenge_cancel: self.challenge_cancel,
            challenge_code_reason: self.challenge_code_reason,
            message_extension: self.message_extension,
            challenge_request_key: self.challenge_request_key,
            customer_details: self.customer_details,
            earliest_supported_version: self.earliest_supported_version,
            latest_supported_version: self.latest_supported_version,
            mcc: self.mcc,
            platform: self.platform.map(|platform| platform.to_string()),
            device_type: self.device_type,
            device_brand: self.device_brand,
            device_os: self.device_os,
            device_display: self.device_display,
            browser_name: self.browser_name,
            browser_version: self.browser_version,
            scheme_name: self.scheme_name,
            exemption_requested: self.exemption_requested,
            exemption_accepted: self.exemption_accepted,
            issuer_id: self.issuer_id,
            issuer_country: self.issuer_country,
            merchant_country_code: self.merchant_country_code,
            created_at: self.created_at,
            modified_at: self.modified_at,
            authentication_data: self.authentication_data,
            billing_country: self.billing_country,
            shipping_country: self.shipping_country,
            processor_merchant_id: self.processor_merchant_id,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
        })
    }
}

impl ForeignFrom<hyperswitch_domain_models::authentication::AuthenticationUpdate> for diesel_models::authentication::AuthenticationUpdate {
    fn foreign_from(from: hyperswitch_domain_models::authentication::AuthenticationUpdate) -> Self {
        use hyperswitch_domain_models::authentication::AuthenticationUpdate as DomainUpdate;
        use crate::transformers::ForeignInto;
        
        match from {
            DomainUpdate::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            } => Self::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            },
            DomainUpdate::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            } => Self::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            },
            DomainUpdate::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address,
                shipping_address,
                browser_info,
                email,
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
            } => Self::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address: billing_address.foreign_into(),
                shipping_address: shipping_address.foreign_into(),
                browser_info,
                email: email.foreign_into(),
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
            },
            DomainUpdate::AuthenticationUpdate {
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status,
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
                device_type,
                device_brand,
                device_os,
                device_display,
            } => Self::AuthenticationUpdate {
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status,
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
                device_type,
                device_brand,
                device_os,
                device_display,
            },
            DomainUpdate::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            } => Self::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            },
            DomainUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => Self::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            },
            DomainUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => Self::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            },
            DomainUpdate::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            } => Self::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            },
        }
    }
}

/*
impl ForeignTryFrom<&diesel_models::authentication::Authentication>
    for hyperswitch_domain_models::router_request_types::authentication::PreAuthenticationData
{
    type Error = error_stack::Report<hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse>;

    fn foreign_try_from(
        authentication: &diesel_models::authentication::Authentication,
    ) -> error_stack::Result<Self, Self::Error> {
        use common_utils::ext_traits::OptionExt;
        use error_stack::ResultExt;
        use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

        let error_message = ApiErrorResponse::UnprocessableEntity {
            message: "Pre Authentication must be completed successfully before Authentication can be performed".to_string(),
        };
        let threeds_server_transaction_id = authentication
            .threeds_server_transaction_id
            .clone()
            .get_required_value("threeds_server_transaction_id")
            .change_context(error_message.clone())?;
        let message_version = authentication
            .message_version
            .clone()
            .get_required_value("message_version")
            .change_context(error_message)?;
        Ok(Self {
            threeds_server_transaction_id,
            message_version,
            acquirer_bin: authentication.acquirer_bin.clone(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.clone(),
            connector_metadata: authentication.connector_metadata.clone(),
            acquirer_country_code: authentication.acquirer_country_code.clone(),
        })
    }
}
*/
