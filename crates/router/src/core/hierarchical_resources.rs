use api_models::hierarchical_resources as api_resources;
use async_trait::async_trait;
use base64::Engine;
use common_utils::{
    date_time, id_type,
    types::keymanager::{self as km_types, EncryptionTransferRequest, KeyManagerState},
};
use error_stack::{report, ResultExt};
use hyperswitch_masking::{PeekInterface, Secret};
use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, PointConversionForm},
    nid::Nid,
    pkey::PKey,
    x509::{X509NameBuilder, X509Req, X509},
};
use storage_impl::behaviour::ForeignInto;

use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::AccountsStorageInterface,
    routes::SessionState,
    services::api::ApplicationResponse,
    types::domain,
};

const RESOURCE_SCOPE_ORGANIZATION: &str = "organization";

#[async_trait]
trait HierarchicalResourceHandler {
    async fn generate(
        state: &SessionState,
        processor: &domain::Processor,
        org_key_store: &domain::MerchantKeyStore,
        request: &api_resources::GenerateHierarchicalResourceRequest,
    ) -> RouterResult<(
        domain::HierarchicalResource,
        api_resources::GenerateHierarchicalResourceResponse,
    )>;

    fn display_schema() -> Secret<serde_json::Value>;

    fn display_data(
        resource: &domain::HierarchicalResource,
    ) -> RouterResult<Secret<serde_json::Value>>;

    fn is_linkable(resource: &domain::HierarchicalResource) -> RouterResult<bool>;

    async fn on_link(
        state: &SessionState,
        key_manager_state: &KeyManagerState,
        resource: &domain::HierarchicalResource,
        resource_id: &id_type::ResourceId,
        account: RequestorAccount,
        account_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<()>;
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApplePayCertificateStatus {
    CsrGenerated,
    Active,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApplePayCertificateData {
    status: ApplePayCertificateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    apple_pay_merchant_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_processing_certificate: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateCachePayload {
    resource_id: id_type::ResourceId,
    data: ApplePayCertificateCacheData,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateCacheData {
    merchant_identifier: Option<String>,
    payment_processing_certificate: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateKeyWrapper {
    data: ApplePayCertificateKeyWrapperData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApplePayCertificateKeyWrapperData {
    payment_processing_certificate_key: Secret<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateCsrData {
    csr: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateDisplaySchema {
    apple_pay_merchant_identifier: &'static str,
}

impl Default for ApplePayCertificateDisplaySchema {
    fn default() -> Self {
        Self {
            apple_pay_merchant_identifier: "String",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct ApplePayCertificateDisplayData {
    apple_pay_merchant_identifier: Option<String>,
}

fn parse_apple_pay_certificate_data(
    resource_data: &Secret<serde_json::Value>,
) -> RouterResult<ApplePayCertificateData> {
    serde_json::from_value(resource_data.peek().clone())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse Apple Pay certificate resource data")
}

struct ApplePayCertificateResource;

#[async_trait]
impl HierarchicalResourceHandler for ApplePayCertificateResource {
    async fn generate(
        state: &SessionState,
        processor: &domain::Processor,
        org_key_store: &domain::MerchantKeyStore,
        _request: &api_resources::GenerateHierarchicalResourceRequest,
    ) -> RouterResult<(
        domain::HierarchicalResource,
        api_resources::GenerateHierarchicalResourceResponse,
    )> {
        let key_manager_state: &KeyManagerState = &state.into();
        let merchant_account = processor.get_account();
        let created_by = format!("merchant:{}", merchant_account.get_id().get_string_repr());
        let organization_id = merchant_account.organization_id.clone();

        let (private_key_pem, csr_pem) = generate_ec_keypair_and_csr()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to generate Apple Pay EC keypair/CSR")?;

        let identifier = km_types::Identifier::Merchant(
            organization_id
                .as_merchant_key_identifier()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to derive organization key identifier")?,
        );

        let private_key_payload = serde_json::to_string(&ApplePayCertificateKeyWrapperData {
            payment_processing_certificate_key: private_key_pem,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Apple Pay private key payload")?;

        let encrypted_private_key = domain::types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(domain::HierarchicalResource),
            domain::types::CryptoOperation::Encrypt(Secret::new(private_key_payload)),
            identifier,
            org_key_store.key.peek(),
        )
        .await
        .and_then(|value| value.try_into_operation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt generated Apple Pay private key")?;

        let data = ApplePayCertificateData {
            status: ApplePayCertificateStatus::CsrGenerated,
            apple_pay_merchant_identifier: None,
            payment_processing_certificate: None,
        };

        let resource_id = id_type::ResourceId::default();
        let resource = domain::HierarchicalResource {
            id: resource_id.clone(),
            resource_type: common_enums::ResourceType::ApplePayCertificate.to_string(),
            scope: RESOURCE_SCOPE_ORGANIZATION.to_string(),
            scope_id: organization_id.get_string_repr().to_string(),
            data: Secret::new(
                serde_json::to_value(&data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize Apple Pay certificate resource data")?,
            ),
            encrypted_data: Some(encrypted_private_key),
            created_by,
            created_at: date_time::now(),
            modified_at: date_time::now(),
        };

        let csr_data = serde_json::to_value(ApplePayCertificateCsrData { csr: csr_pem })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize Apple Pay CSR response data")?;

        let response_data = serde_json::Map::from_iter([(
            common_enums::ResourceType::ApplePayCertificate.to_string(),
            csr_data,
        )]);

        Ok((
            resource,
            api_resources::GenerateHierarchicalResourceResponse {
                id: resource_id,
                data: Secret::new(serde_json::Value::Object(response_data)),
            },
        ))
    }

    fn display_schema() -> Secret<serde_json::Value> {
        Secret::new(
            serde_json::to_value(ApplePayCertificateDisplaySchema::default())
                .unwrap_or(serde_json::Value::Null),
        )
    }

    fn display_data(
        resource: &domain::HierarchicalResource,
    ) -> RouterResult<Secret<serde_json::Value>> {
        let data = parse_apple_pay_certificate_data(&resource.data)?;
        serde_json::to_value(ApplePayCertificateDisplayData {
            apple_pay_merchant_identifier: data.apple_pay_merchant_identifier,
        })
        .map(Secret::new)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Apple Pay certificate display data")
    }

    fn is_linkable(resource: &domain::HierarchicalResource) -> RouterResult<bool> {
        let data = parse_apple_pay_certificate_data(&resource.data)?;
        Ok(data.status == ApplePayCertificateStatus::Active)
    }

    async fn on_link(
        state: &SessionState,
        key_manager_state: &KeyManagerState,
        resource: &domain::HierarchicalResource,
        resource_id: &id_type::ResourceId,
        account: RequestorAccount,
        account_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<()> {
        match resource.encrypted_data.as_ref() {
            Some(private_key) => {
                let data = parse_apple_pay_certificate_data(&resource.data)?;

                let plain_data = serde_json::to_value(ApplePayCertificateCachePayload {
                    resource_id: resource_id.clone(),
                    data: ApplePayCertificateCacheData {
                        merchant_identifier: data.apple_pay_merchant_identifier,
                        payment_processing_certificate: data.payment_processing_certificate,
                    },
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize Apple Pay certificate cache payload")?;

                let private_key_payload: ApplePayCertificateKeyWrapperData =
                    serde_json::from_str(private_key.peek())
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to parse stored Apple Pay private key payload")?;

                let key_wrapper = serde_json::to_value(ApplePayCertificateKeyWrapper {
                    data: private_key_payload,
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize Apple Pay certificate cache key wrapper")?;
                let key_wrapper: Secret<serde_json::Value> = Secret::new(key_wrapper);

                let identifier =
                    km_types::Identifier::Merchant(account_key_store.merchant_id.clone());

                let encrypted_cache = domain::types::crypto_operation(
                    key_manager_state,
                    common_utils::type_name!(domain::HierarchicalResource),
                    domain::types::CryptoOperation::Encrypt(key_wrapper),
                    identifier,
                    account_key_store.key.peek(),
                )
                .await
                .and_then(|value| value.try_into_operation())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encrypt decrypt-time certificate cache")?;

                let update = ApplePayCertificateResourceUpdate {
                    apple_pay_certificates: Some(Secret::new(plain_data)),
                    apple_pay_certificates_encrypted: Some(encrypted_cache.into()),
                };
                link_hierarchical_resource_data_to_scope(
                    state.accounts_store.as_ref(),
                    account_key_store,
                    account,
                    &update,
                )
                .await
            }
            None => Ok(()),
        }
    }
}

#[derive(serde::Serialize)]
struct GooglePayPublicKeyData<'a> {
    public_key: &'a str,
    protocol_version: &'a str,
    recipient_id: &'a str,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GooglePayDisplayData {
    mode: api_resources::GooglePayResourceMode,
    environment: api_resources::GooglePayResourceEnvironment,
    google_merchant_id: String,
    public_key: String,
    protocol_version: String,
    recipient_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GooglePayResourceData {
    merchant_id: id_type::MerchantId,
    mode: api_resources::GooglePayResourceMode,
    environment: api_resources::GooglePayResourceEnvironment,
    google_merchant_id: String,
    public_key: String,
    protocol_version: String,
    recipient_id: String,
}

impl GooglePayResourceData {
    fn is_linkable(&self, resource: &domain::HierarchicalResource) -> bool {
        resource.encrypted_data.is_some()
            && self.protocol_version == "ECv2"
            && !self.public_key.is_empty()
    }
}

impl From<GooglePayResourceData> for GooglePayDisplayData {
    fn from(data: GooglePayResourceData) -> Self {
        Self {
            mode: data.mode,
            environment: data.environment,
            google_merchant_id: data.google_merchant_id,
            public_key: data.public_key,
            protocol_version: data.protocol_version,
            recipient_id: data.recipient_id,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GooglePayResourceKeyData {
    google_pay_private_key: Secret<String>,
}

#[derive(serde::Serialize)]
struct GooglePayResourceKeyWrapper {
    data: GooglePayResourceKeyData,
}

#[derive(serde::Serialize)]
struct GooglePayResourceCachePayload {
    resource_id: id_type::ResourceId,
    data: GooglePayDisplayData,
}

fn validate_google_pay_environment(
    environment: api_resources::GooglePayResourceEnvironment,
) -> RouterResult<()> {
    let is_production = matches!(router_env::env::which(), router_env::env::Env::Production);
    common_utils::fp_utils::when(
        is_production != (environment == api_resources::GooglePayResourceEnvironment::Production),
        || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Google Pay resource environment must match the Hyperswitch environment"
                    .to_string(),
            }))
        },
    )
}

fn parse_google_pay_resource_data(
    resource: &domain::HierarchicalResource,
) -> RouterResult<GooglePayResourceData> {
    serde_json::from_value(resource.data.peek().clone())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse Google Pay DIRECT resource data")
}

/// Google expects a base64 uncompressed P-256 public point and a PKCS#8 DER private key.
fn generate_google_pay_direct_keypair(
) -> Result<(Secret<String>, String), openssl::error::ErrorStack> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let key = EcKey::generate(&group)?;
    let mut context = BigNumContext::new()?;
    let public_key =
        key.public_key()
            .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut context)?;
    let private_key = PKey::from_ec_key(key)?.private_key_to_pkcs8()?;
    Ok((
        Secret::new(common_utils::consts::BASE64_ENGINE.encode(private_key)),
        common_utils::consts::BASE64_ENGINE.encode(public_key),
    ))
}

struct GooglePayResource;

#[async_trait]
impl HierarchicalResourceHandler for GooglePayResource {
    async fn generate(
        state: &SessionState,
        processor: &domain::Processor,
        org_key_store: &domain::MerchantKeyStore,
        request: &api_resources::GenerateHierarchicalResourceRequest,
    ) -> RouterResult<(
        domain::HierarchicalResource,
        api_resources::GenerateHierarchicalResourceResponse,
    )> {
        let configuration = request.google_pay.as_ref().ok_or_else(|| {
            report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "google_pay".into(),
            })
        })?;
        common_utils::fp_utils::when(
            configuration.google_merchant_id.is_empty()
                || configuration.google_merchant_id.trim() != configuration.google_merchant_id,
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message:
                        "google_merchant_id must be nonempty and contain no surrounding whitespace"
                            .to_string(),
                }))
            },
        )?;
        validate_google_pay_environment(configuration.environment)?;

        let (private_key, public_key) = generate_google_pay_direct_keypair()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to generate Google Pay DIRECT P-256 keypair")?;
        let key_manager_state: &KeyManagerState = &state.into();
        let merchant_account = processor.get_account();
        let organization_id = &merchant_account.organization_id;
        let private_key_payload = serde_json::to_string(&GooglePayResourceKeyData {
            google_pay_private_key: private_key,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Google Pay private key payload")?;
        let identifier = km_types::Identifier::Merchant(
            organization_id
                .as_merchant_key_identifier()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to derive Google Pay organization key identifier")?,
        );
        let encrypted_private_key = domain::types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(domain::HierarchicalResource),
            domain::types::CryptoOperation::Encrypt(Secret::new(private_key_payload)),
            identifier,
            org_key_store.key.peek(),
        )
        .await
        .and_then(|value| value.try_into_operation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt generated Google Pay private key")?;
        let resource_data = GooglePayResourceData {
            merchant_id: merchant_account.get_id().clone(),
            mode: configuration.mode,
            environment: configuration.environment,
            google_merchant_id: configuration.google_merchant_id.clone(),
            public_key,
            protocol_version: "ECv2".to_string(),
            recipient_id: format!("merchant:{}", configuration.google_merchant_id),
        };
        let public_key_data = serde_json::to_value(GooglePayPublicKeyData {
            public_key: &resource_data.public_key,
            protocol_version: &resource_data.protocol_version,
            recipient_id: &resource_data.recipient_id,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Google Pay public key response data")?;
        let response_data = serde_json::Map::from_iter([(
            common_enums::ResourceType::GooglePay.to_string(),
            public_key_data,
        )]);
        let resource_id = id_type::ResourceId::default();
        let resource = domain::HierarchicalResource {
            id: resource_id.clone(),
            resource_type: common_enums::ResourceType::GooglePay.to_string(),
            scope: RESOURCE_SCOPE_ORGANIZATION.to_string(),
            scope_id: organization_id.get_string_repr().to_string(),
            data: Secret::new(
                serde_json::to_value(resource_data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize Google Pay resource data")?,
            ),
            encrypted_data: Some(encrypted_private_key),
            created_by: format!("merchant:{}", merchant_account.get_id().get_string_repr()),
            created_at: date_time::now(),
            modified_at: date_time::now(),
        };
        Ok((
            resource,
            api_resources::GenerateHierarchicalResourceResponse {
                id: resource_id,
                data: Secret::new(serde_json::Value::Object(response_data)),
            },
        ))
    }

    fn display_schema() -> Secret<serde_json::Value> {
        Secret::new(serde_json::json!({
            "google_merchant_id": "Google Merchant ID",
            "mode": "Tokenization mode",
            "environment": "Environment",
            "public_key": "Public key",
            "protocol_version": "Protocol version",
            "recipient_id": "Recipient ID"
        }))
    }

    fn display_data(
        resource: &domain::HierarchicalResource,
    ) -> RouterResult<Secret<serde_json::Value>> {
        let data = parse_google_pay_resource_data(resource)?;
        serde_json::to_value(GooglePayDisplayData::from(data))
            .map(Secret::new)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize Google Pay resource display data")
    }

    fn is_linkable(resource: &domain::HierarchicalResource) -> RouterResult<bool> {
        let data = parse_google_pay_resource_data(resource)?;
        Ok(data.is_linkable(resource))
    }

    async fn on_link(
        state: &SessionState,
        key_manager_state: &KeyManagerState,
        resource: &domain::HierarchicalResource,
        resource_id: &id_type::ResourceId,
        account: RequestorAccount,
        account_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<()> {
        common_utils::fp_utils::when(
            matches!(&account, RequestorAccount::MerchantConnectorAccount(_)),
            || {
                Err(report!(errors::ApiErrorResponse::NotSupported {
                    message: "Google Pay DIRECT resources can only be linked to a profile or merchant account"
                        .to_string(),
                }))
            },
        )?;
        let data = parse_google_pay_resource_data(resource)?;
        common_utils::fp_utils::when(data.merchant_id != account_key_store.merchant_id, || {
            Err(report!(errors::ApiErrorResponse::GenericNotFoundError {
                message: "resource not found".to_string(),
            }))
        })?;
        validate_google_pay_environment(data.environment)?;
        common_utils::fp_utils::when(!data.is_linkable(resource), || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Google Pay resource is not ready for linking".to_string(),
            }))
        })?;
        let private_key = resource.encrypted_data.as_ref().ok_or_else(|| {
            report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Google Pay resource has no encrypted private key")
        })?;
        let private_key_payload: GooglePayResourceKeyData =
            serde_json::from_str(private_key.peek())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse stored Google Pay private key payload")?;
        let key_wrapper = serde_json::to_value(GooglePayResourceKeyWrapper {
            data: private_key_payload,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Google Pay cache key wrapper")?;
        let key_wrapper: Secret<serde_json::Value> = Secret::new(key_wrapper);
        let encrypted_cache = domain::types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(domain::HierarchicalResource),
            domain::types::CryptoOperation::Encrypt(key_wrapper),
            km_types::Identifier::Merchant(account_key_store.merchant_id.clone()),
            account_key_store.key.peek(),
        )
        .await
        .and_then(|value| value.try_into_operation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt Google Pay decryption cache")?;
        let public_cache = serde_json::to_value(GooglePayResourceCachePayload {
            resource_id: resource_id.clone(),
            data: data.into(),
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Google Pay decryption configuration")?;
        let update = GooglePayResourceUpdate {
            google_pay_decryption_config: public_cache,
            google_pay_decryption_config_encrypted: encrypted_cache.into(),
        };
        link_hierarchical_resource_data_to_scope(
            state.accounts_store.as_ref(),
            account_key_store,
            account,
            &update,
        )
        .await
    }
}

impl ApplePayCertificateResource {
    async fn upload(
        state: &SessionState,
        resource: domain::HierarchicalResource,
        org_key: &Secret<Vec<u8>>,
        req: api_resources::UploadCertificateRequest,
    ) -> RouterResult<api_resources::UploadCertificateResponse> {
        let private_key_payload = resource
            .encrypted_data
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Resource has no private key — cannot verify uploaded certificate")?
            .peek()
            .clone();
        let private_key_pem: ApplePayCertificateKeyWrapperData =
            serde_json::from_str(&private_key_payload)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse stored Apple Pay private key payload")?;

        let certificate = parse_certificate_from_base64_der(&req.certificate)?;

        let keys_match = certificate_matches_private_key(
            &certificate,
            private_key_pem.payment_processing_certificate_key.peek(),
        )
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Failed to verify uploaded certificate".to_string(),
        })?;
        common_utils::fp_utils::when(!keys_match, || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Uploaded certificate does not match the generated key".to_string(),
            }))
        })?;

        let apple_merchant_identifier = parse_apple_merchant_identifier(&certificate)
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Could not parse Apple merchant identifier from certificate".to_string(),
            })?;

        let data = ApplePayCertificateData {
            status: ApplePayCertificateStatus::Active,
            apple_pay_merchant_identifier: Some(apple_merchant_identifier),
            payment_processing_certificate: Some(req.certificate),
        };

        let resource_update = domain::HierarchicalResourceDataUpdate {
            data: Secret::new(
                serde_json::to_value(&data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize Apple Pay certificate resource data")?,
            ),
        };
        let updated = state
            .accounts_store
            .update_linked_resource_data(resource.id.clone(), resource_update, org_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to persist uploaded Apple Pay certificate")?;

        Ok(api_resources::UploadCertificateResponse {
            id: updated.id,
            created_at: updated.created_at,
        })
    }
}

trait RequestorHierarchicalResourceUpdate {
    fn for_merchant_connector_account(
        &self,
    ) -> RouterResult<domain::MerchantConnectorAccountUpdate>;
    fn for_profile(&self) -> domain::ProfileUpdate;
    fn for_merchant_account(&self) -> domain::MerchantAccountUpdate;
}

struct ApplePayCertificateResourceUpdate {
    apple_pay_certificates: Option<Secret<serde_json::Value>>,
    apple_pay_certificates_encrypted: Option<common_utils::encryption::Encryption>,
}

impl ApplePayCertificateResourceUpdate {
    fn apple_pay_certificates(&self) -> Option<serde_json::Value> {
        self.apple_pay_certificates
            .as_ref()
            .map(|data| data.peek().clone())
    }
}

impl RequestorHierarchicalResourceUpdate for ApplePayCertificateResourceUpdate {
    fn for_merchant_connector_account(
        &self,
    ) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
        Ok(
            domain::MerchantConnectorAccountUpdate::ApplePayCertificateCacheUpdate {
                apple_pay_certificates: self.apple_pay_certificates(),
                apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
            },
        )
    }

    fn for_profile(&self) -> domain::ProfileUpdate {
        domain::ProfileUpdate::ApplePayCertificateCacheUpdate {
            apple_pay_certificates: self.apple_pay_certificates(),
            apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
        }
    }

    fn for_merchant_account(&self) -> domain::MerchantAccountUpdate {
        domain::MerchantAccountUpdate::ApplePayCertificateCacheUpdate {
            apple_pay_certificates: self.apple_pay_certificates(),
            apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
        }
    }
}

struct GooglePayResourceUpdate {
    google_pay_decryption_config: serde_json::Value,
    google_pay_decryption_config_encrypted: common_utils::encryption::Encryption,
}

impl RequestorHierarchicalResourceUpdate for GooglePayResourceUpdate {
    fn for_merchant_connector_account(
        &self,
    ) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
        Err(errors::ApiErrorResponse::NotSupported {
            message:
                "Google Pay DIRECT resources can only be linked to a profile or merchant account"
                    .to_string(),
        }
        .into())
    }

    fn for_profile(&self) -> domain::ProfileUpdate {
        domain::ProfileUpdate::GooglePayDecryptionConfigUpdate {
            google_pay_decryption_config: Some(self.google_pay_decryption_config.clone()),
            google_pay_decryption_config_encrypted: Some(
                self.google_pay_decryption_config_encrypted.clone(),
            ),
        }
    }

    fn for_merchant_account(&self) -> domain::MerchantAccountUpdate {
        domain::MerchantAccountUpdate::GooglePayDecryptionConfigUpdate {
            google_pay_decryption_config: Some(self.google_pay_decryption_config.clone()),
            google_pay_decryption_config_encrypted: Some(
                self.google_pay_decryption_config_encrypted.clone(),
            ),
        }
    }
}

fn resolve_google_pay_resource_data(
    processor: &domain::Processor,
    account: &RequestorAccount,
) -> RouterResult<Option<serde_json::Value>> {
    match account {
        RequestorAccount::Profile(profile) => Ok(profile
            .google_pay_decryption_config
            .clone()
            .or_else(|| processor.get_account().google_pay_decryption_config.clone())),
        RequestorAccount::MerchantAccount(merchant) => {
            Ok(merchant.google_pay_decryption_config.clone())
        }
        RequestorAccount::MerchantConnectorAccount(_) => {
            Err(errors::ApiErrorResponse::NotSupported {
                message: "Google Pay DIRECT resource linking does not use an MCA".to_string(),
            }
            .into())
        }
    }
}

enum RequestorAccount {
    MerchantConnectorAccount(Box<domain::MerchantConnectorAccount>),
    Profile(Box<domain::Profile>),
    MerchantAccount(Box<domain::MerchantAccount>),
}

fn parse_requestor_id<T>(value: &str) -> RouterResult<T>
where
    T: TryFrom<
        std::borrow::Cow<'static, str>,
        Error = error_stack::Report<common_utils::errors::ValidationError>,
    >,
{
    T::try_from(std::borrow::Cow::Owned(value.to_owned())).change_context(
        errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid requestor_id".to_string(),
        },
    )
}

/// Fetches the target MCA/profile/merchant_account scoped strictly to `processor`'s own
/// merchant — a `NotFound` here is itself the authorization check, no separate org-membership
/// check is needed since the fetch can never return an entity belonging to a different merchant.
#[cfg(feature = "v1")]
async fn fetch_requestor_account(
    db: &dyn AccountsStorageInterface,
    processor: &domain::Processor,
    requestor_type: common_enums::ResourceRequestorType,
    requestor_id: &str,
) -> RouterResult<RequestorAccount> {
    match requestor_type {
        common_enums::ResourceRequestorType::MerchantConnectorAccount => {
            let mca_id = parse_requestor_id::<id_type::MerchantConnectorAccountId>(requestor_id)?;
            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    processor.get_account().get_id(),
                    &mca_id,
                    processor.get_key_store(),
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: mca_id.get_string_repr().to_string(),
                    },
                )?;
            Ok(RequestorAccount::MerchantConnectorAccount(Box::new(mca)))
        }
        common_enums::ResourceRequestorType::Profile => {
            let profile_id = parse_requestor_id::<id_type::ProfileId>(requestor_id)?;
            let profile = db
                .find_business_profile_by_merchant_id_profile_id(
                    processor.get_key_store(),
                    processor.get_account().get_id(),
                    &profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                    id: profile_id.get_string_repr().to_owned(),
                })?;
            Ok(RequestorAccount::Profile(Box::new(profile)))
        }
        common_enums::ResourceRequestorType::MerchantAccount => {
            let merchant_id = parse_requestor_id::<id_type::MerchantId>(requestor_id)?;
            common_utils::fp_utils::when(&merchant_id != processor.get_account().get_id(), || {
                Err(report!(errors::ApiErrorResponse::MerchantAccountNotFound))
            })?;
            Ok(RequestorAccount::MerchantAccount(Box::new(
                processor.get_account().clone(),
            )))
        }
    }
}

#[cfg(feature = "v2")]
async fn fetch_requestor_account(
    _db: &dyn AccountsStorageInterface,
    _processor: &domain::Processor,
    _requestor_type: common_enums::ResourceRequestorType,
    _requestor_id: &str,
) -> RouterResult<RequestorAccount> {
    Err(report!(errors::ApiErrorResponse::NotSupported {
        message: "Resource linking is not supported for v2".to_string(),
    }))
}

/// Reads the linked-certificate cache off an already-fetched `RequestorAccount`, cascading to
/// its profile and then its merchant account exactly like `resolve_apple_pay_certificate_cache`
/// used to — safely, since an MCA's own profile/merchant are inherent properties of an entity
/// already scoped to `processor`'s merchant, not caller-supplied ids.
#[cfg(feature = "v1")]
async fn resolve_apple_pay_certificate_data(
    db: &dyn AccountsStorageInterface,
    processor: &domain::Processor,
    account: &RequestorAccount,
) -> Option<serde_json::Value> {
    match account {
        RequestorAccount::MerchantConnectorAccount(mca) => match mca.apple_pay_certificates.clone()
        {
            Some(data) => Some(data),
            None => {
                let profile = db
                    .find_business_profile_by_merchant_id_profile_id(
                        processor.get_key_store(),
                        processor.get_account().get_id(),
                        &mca.profile_id,
                    )
                    .await
                    .ok()?;
                profile
                    .apple_pay_certificates
                    .or_else(|| processor.get_account().apple_pay_certificates.clone())
            }
        },
        RequestorAccount::Profile(profile) => profile
            .apple_pay_certificates
            .clone()
            .or_else(|| processor.get_account().apple_pay_certificates.clone()),
        RequestorAccount::MerchantAccount(merchant_account) => {
            merchant_account.apple_pay_certificates.clone()
        }
    }
}

#[cfg(feature = "v2")]
async fn resolve_apple_pay_certificate_data(
    _db: &dyn AccountsStorageInterface,
    _processor: &domain::Processor,
    _account: &RequestorAccount,
) -> Option<serde_json::Value> {
    None
}

async fn link_hierarchical_resource_data_to_scope(
    db: &dyn AccountsStorageInterface,
    key_store: &domain::MerchantKeyStore,
    account: RequestorAccount,
    update: &impl RequestorHierarchicalResourceUpdate,
) -> RouterResult<()> {
    match account {
        RequestorAccount::MerchantConnectorAccount(mca) => {
            db.update_merchant_connector_account(
                *mca,
                update.for_merchant_connector_account()?.foreign_into(),
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to link resource data to merchant connector account")?;
        }
        RequestorAccount::Profile(profile) => {
            db.update_profile_by_profile_id(key_store, *profile, update.for_profile())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to link resource data to profile")?;
        }
        RequestorAccount::MerchantAccount(merchant_account) => {
            db.update_merchant(*merchant_account, update.for_merchant_account(), key_store)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to link resource data to merchant account")?;
        }
    }

    Ok(())
}

pub async fn generate_hierarchical_resource(
    state: SessionState,
    processor: domain::Processor,
    req: api_resources::GenerateHierarchicalResourceRequest,
) -> RouterResponse<api_resources::GenerateHierarchicalResourceResponse> {
    let key_manager_state: &KeyManagerState = &(&state).into();
    let organization_id = processor.get_account().organization_id.clone();

    let org_key_store =
        ensure_organization_key_store(&state, key_manager_state, &organization_id).await?;

    let (resource, response) = match req.resource_type {
        common_enums::ResourceType::ApplePayCertificate => {
            common_utils::fp_utils::when(req.google_pay.is_some(), || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "google_pay configuration is only valid for google_pay resources"
                        .to_string(),
                }))
            })?;
            ApplePayCertificateResource::generate(&state, &processor, &org_key_store, &req).await?
        }
        common_enums::ResourceType::GooglePay => {
            GooglePayResource::generate(&state, &processor, &org_key_store, &req).await?
        }
    };

    state
        .accounts_store
        .insert_linked_resource(resource, &org_key_store.key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to persist generated resource")?;

    Ok(ApplicationResponse::Json(response))
}

fn generate_ec_keypair_and_csr(
) -> error_stack::Result<(Secret<String>, String), openssl::error::ErrorStack> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let ec_key = EcKey::generate(&group)?;
    let pkey = PKey::from_ec_key(ec_key)?;

    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_text("CN", "Apple Pay Payment Processing Certificate Request")?;
    let name = name_builder.build();

    let mut req_builder = X509Req::builder()?;
    req_builder.set_subject_name(&name)?;
    req_builder.set_pubkey(&pkey)?;
    req_builder.sign(&pkey, openssl::hash::MessageDigest::sha256())?;
    let csr = req_builder.build();

    let csr_pem = csr.to_pem()?;
    let private_key_pem = pkey.private_key_to_pem_pkcs8()?;

    Ok((
        Secret::new(String::from_utf8_lossy(&private_key_pem).to_string()),
        String::from_utf8_lossy(&csr_pem).to_string(),
    ))
}

pub async fn upload_hierarchical_resource(
    state: SessionState,
    processor: domain::Processor,
    resource_id: id_type::ResourceId,
    req: api_resources::UploadCertificateRequest,
) -> RouterResponse<api_resources::UploadCertificateResponse> {
    let key_manager_state: &KeyManagerState = &(&state).into();
    let organization_id = processor.get_account().organization_id.clone();

    let scope_id = state
        .accounts_store
        .find_resource_scope_id(resource_id.clone())
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "resource not found".to_string(),
        })?;
    authorize_scope_id_belongs_to_org(&scope_id, &organization_id)?;

    let org_key_store =
        ensure_organization_key_store(&state, key_manager_state, &organization_id).await?;

    let resource = state
        .accounts_store
        .find_linked_resource_by_id(resource_id.clone(), &org_key_store.key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "resource not found".to_string(),
        })?;

    let resource_type = parse_resource_type(&resource)?;

    let response = match resource_type {
        common_enums::ResourceType::ApplePayCertificate => {
            ApplePayCertificateResource::upload(&state, resource, &org_key_store.key, req).await?
        }
        common_enums::ResourceType::GooglePay => Err(errors::ApiErrorResponse::NotSupported {
            message: "Google Pay DIRECT resources do not accept certificate uploads".to_string(),
        })?,
    };

    Ok(ApplicationResponse::Json(response))
}

fn parse_resource_type(
    resource: &domain::HierarchicalResource,
) -> RouterResult<common_enums::ResourceType> {
    resource
        .resource_type
        .parse::<common_enums::ResourceType>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Unrecognized resource_type '{}' stored for resource {:?}",
                resource.resource_type, resource.id
            )
        })
}

fn parse_certificate_from_base64_der(certificate_base64: &str) -> RouterResult<X509> {
    let der_bytes = crate::consts::BASE64_ENGINE
        .decode(certificate_base64)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "certificate is not valid base64".to_string(),
        })?;

    X509::from_der(&der_bytes).change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "certificate is not a valid DER-encoded X.509 certificate".to_string(),
    })
}

fn certificate_matches_private_key(
    certificate: &X509,
    private_key_pem: &str,
) -> error_stack::Result<bool, openssl::error::ErrorStack> {
    let certificate_public_key_der = certificate.public_key()?.public_key_to_der()?;

    let private_key = PKey::private_key_from_pem(private_key_pem.as_bytes())?;
    let private_key_public_der = private_key.public_key_to_der()?;

    Ok(certificate_public_key_der == private_key_public_der)
}

fn parse_apple_merchant_identifier(
    certificate: &X509,
) -> error_stack::Result<String, openssl::error::ErrorStack> {
    let common_name = certificate
        .subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .ok_or_else(openssl::error::ErrorStack::get)?
        .data()
        .to_string()?;

    Ok(common_name
        .rsplit(':')
        .next()
        .unwrap_or(&common_name)
        .trim()
        .to_string())
}

pub async fn list_hierarchical_resources(
    state: SessionState,
    processor: domain::Processor,
    req: api_resources::ListHierarchicalResourcesRequest,
) -> RouterResponse<api_resources::ListHierarchicalResourcesResponse> {
    let key_manager_state: &KeyManagerState = &(&state).into();
    let organization_id = processor.get_account().organization_id.clone();

    let org_key_store =
        ensure_organization_key_store(&state, key_manager_state, &organization_id).await?;

    let resources = state
        .accounts_store
        .list_linked_resources_by_scope_id_and_resource_type(
            organization_id.get_string_repr().to_string(),
            req.resource_type.to_string(),
            &org_key_store.key,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to list resources")?;

    let effective_resource_id = match (req.scope_type, req.scope_id) {
        (Some(scope_type), Some(scope_id)) => {
            let account = fetch_requestor_account(
                state.accounts_store.as_ref(),
                &processor,
                scope_type,
                &scope_id,
            )
            .await?;
            let linked_data = match req.resource_type {
                common_enums::ResourceType::ApplePayCertificate => {
                    resolve_apple_pay_certificate_data(
                        state.accounts_store.as_ref(),
                        &processor,
                        &account,
                    )
                    .await
                }
                common_enums::ResourceType::GooglePay => {
                    resolve_google_pay_resource_data(&processor, &account)?
                }
            };
            match linked_data {
                Some(data) => data
                    .get("resource_id")
                    .and_then(|value| value.as_str())
                    .map(parse_requestor_id::<id_type::ResourceId>)
                    .transpose()?,
                None => None,
            }
        }
        _ => None,
    };

    let resources = resources
        .iter()
        .filter(|resource| {
            req.resource_type != common_enums::ResourceType::GooglePay
                || resource.created_by.strip_prefix("merchant:")
                    == Some(processor.get_account().get_id().get_string_repr())
        })
        .try_fold(Vec::new(), |mut summaries, resource| {
            let is_linkable = match req.resource_type {
                common_enums::ResourceType::ApplePayCertificate => {
                    hierarchical_resource_is_linkable(resource).unwrap_or(false)
                }
                common_enums::ResourceType::GooglePay => {
                    hierarchical_resource_is_linkable(resource)?
                }
            };
            if is_linkable {
                summaries.push(hierarchical_resource_summary(
                    resource,
                    effective_resource_id.as_ref(),
                )?);
            }
            Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(summaries)
        })?;

    Ok(ApplicationResponse::Json(
        api_resources::ListHierarchicalResourcesResponse { resources },
    ))
}

fn hierarchical_resource_is_linkable(
    resource: &domain::HierarchicalResource,
) -> RouterResult<bool> {
    let resource_type = parse_resource_type(resource)?;

    match resource_type {
        common_enums::ResourceType::ApplePayCertificate => {
            ApplePayCertificateResource::is_linkable(resource)
        }
        common_enums::ResourceType::GooglePay => GooglePayResource::is_linkable(resource),
    }
}

fn hierarchical_resource_summary(
    resource: &domain::HierarchicalResource,
    effective_resource_id: Option<&id_type::ResourceId>,
) -> RouterResult<api_resources::HierarchicalResourceSummary> {
    let resource_type = parse_resource_type(resource)?;

    let (display_schema, display_data) = match resource_type {
        common_enums::ResourceType::ApplePayCertificate => (
            ApplePayCertificateResource::display_schema(),
            ApplePayCertificateResource::display_data(resource)?,
        ),
        common_enums::ResourceType::GooglePay => (
            GooglePayResource::display_schema(),
            GooglePayResource::display_data(resource)?,
        ),
    };

    Ok(api_resources::HierarchicalResourceSummary {
        id: resource.id.clone(),
        display_schema,
        display_data,
        created_at: resource.created_at,
        is_linked: effective_resource_id == Some(&resource.id),
    })
}

pub async fn link_hierarchical_resource(
    state: SessionState,
    processor: domain::Processor,
    resource_id: id_type::ResourceId,
    req: api_resources::LinkHierarchicalResourceRequest,
) -> RouterResponse<api_resources::LinkHierarchicalResourceResponse> {
    let key_manager_state: &KeyManagerState = &(&state).into();
    let merchant_account = processor.get_account();
    let organization_id = merchant_account.organization_id.clone();

    let scope_id = state
        .accounts_store
        .find_resource_scope_id(resource_id.clone())
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "resource not found".to_string(),
        })?;
    authorize_scope_id_belongs_to_org(&scope_id, &organization_id)?;

    let account = fetch_requestor_account(
        state.accounts_store.as_ref(),
        &processor,
        req.requestor_type,
        &req.requestor_id,
    )
    .await?;

    let org_key_store =
        ensure_organization_key_store(&state, key_manager_state, &organization_id).await?;

    let resource = state
        .accounts_store
        .find_linked_resource_by_id(resource_id.clone(), &org_key_store.key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch resource to link")?;

    let resource_type = parse_resource_type(&resource)?;

    match resource_type {
        common_enums::ResourceType::ApplePayCertificate => {
            ApplePayCertificateResource::on_link(
                &state,
                key_manager_state,
                &resource,
                &resource_id,
                account,
                processor.get_key_store(),
            )
            .await?;
        }
        common_enums::ResourceType::GooglePay => {
            GooglePayResource::on_link(
                &state,
                key_manager_state,
                &resource,
                &resource_id,
                account,
                processor.get_key_store(),
            )
            .await?;
        }
    }

    Ok(ApplicationResponse::Json(
        api_resources::LinkHierarchicalResourceResponse { id: resource_id },
    ))
}

fn authorize_scope_id_belongs_to_org(
    scope_id: &str,
    organization_id: &id_type::OrganizationId,
) -> RouterResult<()> {
    common_utils::fp_utils::when(scope_id != organization_id.get_string_repr(), || {
        Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "resource".to_string(),
        }))
    })
}

pub(crate) async fn ensure_organization_key_store(
    state: &SessionState,
    key_manager_state: &KeyManagerState,
    organization_id: &id_type::OrganizationId,
) -> RouterResult<domain::MerchantKeyStore> {
    let org_key_identifier = organization_id
        .as_merchant_key_identifier()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to derive organization key identifier")?;

    let master_key = state.store.get_master_key();

    let key_store_master_key = master_key.to_vec().into();
    match state
        .accounts_store
        .get_merchant_key_store_by_merchant_id(&org_key_identifier, &key_store_master_key)
        .await
    {
        Ok(key_store) => Ok(key_store),
        Err(error) if error.current_context().is_db_not_found() => {
            create_organization_key_store(
                state.accounts_store.as_ref(),
                key_manager_state,
                &org_key_identifier,
                master_key,
            )
            .await
        }
        Err(error) => Err(error).change_context(errors::ApiErrorResponse::InternalServerError),
    }
}

async fn create_organization_key_store(
    db: &dyn AccountsStorageInterface,
    key_manager_state: &KeyManagerState,
    org_key_identifier: &id_type::MerchantId,
    master_key: &[u8],
) -> RouterResult<domain::MerchantKeyStore> {
    let key = crate::services::generate_aes256_key()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate aes 256 key for organization")?;

    let identifier = km_types::Identifier::Merchant(org_key_identifier.clone());

    common_utils::keymanager::transfer_key_to_key_manager(
        key_manager_state,
        EncryptionTransferRequest {
            identifier: identifier.clone(),
            key: hyperswitch_masking::StrongSecret::new(crate::consts::BASE64_ENGINE.encode(key)),
        },
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to insert organization key to KeyManager")?;

    let key_store = domain::MerchantKeyStore {
        merchant_id: org_key_identifier.clone(),
        key: domain::types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(domain::MerchantKeyStore),
            domain::types::CryptoOperation::EncryptLocally(key.to_vec().into()),
            identifier,
            master_key,
        )
        .await
        .and_then(|value| value.try_into_operation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt organization key")?,
        created_at: date_time::now(),
    };

    let key_store_master_key = master_key.to_vec().into();
    db.insert_merchant_key_store(key_store.clone(), &key_store_master_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert organization key store")?;

    Ok(key_store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_pay_direct_keys_use_matching_pkcs8_and_uncompressed_p256_encodings(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (private_key, public_key) = generate_google_pay_direct_keypair()?;
        let private_der = common_utils::consts::BASE64_ENGINE.decode(private_key.peek())?;
        let public_point = common_utils::consts::BASE64_ENGINE.decode(public_key)?;
        let private_key = PKey::private_key_from_pkcs8(&private_der)?;
        let ec_key = private_key.ec_key()?;
        let mut context = BigNumContext::new()?;
        let derived_public = ec_key.public_key().to_bytes(
            ec_key.group(),
            PointConversionForm::UNCOMPRESSED,
            &mut context,
        )?;

        if ec_key.group().curve_name() != Some(Nid::X9_62_PRIME256V1)
            || public_point.first() != Some(&4)
            || public_point != derived_public
        {
            Err("Google Pay public point must match the retained P-256 PKCS#8 private key".into())
        } else {
            Ok(())
        }
    }
}
