use api_models::resources as api_resources;
use async_trait::async_trait;
use base64::Engine;
use common_utils::{
    date_time, id_type,
    types::keymanager::{self as km_types, EncryptionTransferRequest, KeyManagerState},
};
use error_stack::{report, ResultExt};
use hyperswitch_masking::{PeekInterface, Secret};
use openssl::{
    ec::{EcGroup, EcKey},
    nid::Nid,
    pkey::PKey,
    x509::{X509NameBuilder, X509Req, X509},
};

use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::StorageInterface,
    routes::SessionState,
    services::api::ApplicationResponse,
    types::domain,
};

const RESOURCE_SCOPE_ORGANIZATION: &str = "organization";

#[async_trait]
trait ResourceHandler {
    async fn generate(
        state: &SessionState,
        processor: &domain::Processor,
        req: &api_resources::GenerateResourceRequest,
    ) -> RouterResult<api_resources::GenerateResourceResponse>;

    async fn upload(
        state: &SessionState,
        resource: domain::Resource,
        org_key: &Secret<Vec<u8>>,
        req: api_resources::UploadCertificateRequest,
    ) -> RouterResult<api_resources::UploadCertificateResponse>;

    fn display_schema() -> serde_json::Value;

    fn display_data(resource: &domain::Resource) -> RouterResult<serde_json::Value>;

    #[allow(clippy::too_many_arguments)]
    async fn on_link(
        state: &SessionState,
        key_manager_state: &KeyManagerState,
        organization_id: &id_type::OrganizationId,
        org_key: &Secret<Vec<u8>>,
        resource: &domain::Resource,
        resource_id: &id_type::ResourceId,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> RouterResult<()>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApplePayCertificateData {
    status: common_enums::ResourceStatus,
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
    resource_data: &serde_json::Value,
) -> RouterResult<ApplePayCertificateData> {
    serde_json::from_value(resource_data.clone())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse Apple Pay certificate resource data")
}

struct ApplePayCertificateResource;

#[async_trait]
impl ResourceHandler for ApplePayCertificateResource {
    async fn generate(
        state: &SessionState,
        processor: &domain::Processor,
        req: &api_resources::GenerateResourceRequest,
    ) -> RouterResult<api_resources::GenerateResourceResponse> {
        let db = state.store.as_ref();
        let key_manager_state: &KeyManagerState = &state.into();
        let merchant_account = processor.get_account();
        let created_by = format!("merchant:{}", merchant_account.get_id().get_string_repr());
        let organization_id = merchant_account.organization_id.clone();

        let org_key_store =
            ensure_organization_key_store(db, key_manager_state, &organization_id).await?;

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
            common_utils::type_name!(domain::Resource),
            domain::types::CryptoOperation::Encrypt(Secret::new(private_key_payload)),
            identifier,
            org_key_store.key.peek(),
        )
        .await
        .and_then(|value| value.try_into_operation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt generated Apple Pay private key")?;

        let data = ApplePayCertificateData {
            status: common_enums::ResourceStatus::CsrGenerated,
            apple_pay_merchant_identifier: req.apple_merchant_identifier.clone(),
            payment_processing_certificate: None,
        };

        let resource_id = id_type::ResourceId::default();
        let resource = domain::Resource {
            id: resource_id.clone(),
            resource_type: common_enums::ResourceType::ApplePayCertificate.to_string(),
            scope: RESOURCE_SCOPE_ORGANIZATION.to_string(),
            scope_id: organization_id.get_string_repr().to_string(),
            data: serde_json::to_value(&data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize Apple Pay certificate resource data")?,
            encrypted_data: Some(encrypted_private_key),
            created_by,
            created_at: date_time::now(),
            modified_at: date_time::now(),
        };

        db.insert_linked_resource(resource, &org_key_store.key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to persist generated Apple Pay certificate resource")?;

        let csr_data = serde_json::to_value(ApplePayCertificateCsrData { csr: csr_pem })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize Apple Pay CSR response data")?;

        let data = serde_json::Map::from_iter([(
            common_enums::ResourceType::ApplePayCertificate.to_string(),
            csr_data,
        )]);

        Ok(api_resources::GenerateResourceResponse {
            id: resource_id,
            data: serde_json::Value::Object(data),
        })
    }

    async fn upload(
        state: &SessionState,
        resource: domain::Resource,
        org_key: &Secret<Vec<u8>>,
        req: api_resources::UploadCertificateRequest,
    ) -> RouterResult<api_resources::UploadCertificateResponse> {
        let db = state.store.as_ref();

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
        if !keys_match {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Uploaded certificate does not match the generated key".to_string(),
            }));
        }

        let apple_merchant_identifier = parse_apple_merchant_identifier(&certificate)
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Could not parse Apple merchant identifier from certificate".to_string(),
            })?;

        let data = ApplePayCertificateData {
            status: common_enums::ResourceStatus::Active,
            apple_pay_merchant_identifier: Some(apple_merchant_identifier),
            payment_processing_certificate: Some(req.certificate),
        };

        let updated = db
            .update_linked_resource_data(
                resource.id.clone(),
                domain::ResourceDataUpdate {
                    data: serde_json::to_value(&data)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to serialize Apple Pay certificate resource data",
                        )?,
                },
                org_key,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to persist uploaded Apple Pay certificate")?;

        Ok(api_resources::UploadCertificateResponse {
            id: updated.id,
            created_at: updated.created_at,
        })
    }

    fn display_schema() -> serde_json::Value {
        serde_json::to_value(ApplePayCertificateDisplaySchema::default())
            .unwrap_or(serde_json::Value::Null)
    }

    fn display_data(resource: &domain::Resource) -> RouterResult<serde_json::Value> {
        let data = parse_apple_pay_certificate_data(&resource.data)?;
        serde_json::to_value(ApplePayCertificateDisplayData {
            apple_pay_merchant_identifier: data.apple_pay_merchant_identifier,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize Apple Pay certificate display data")
    }

    async fn on_link(
        state: &SessionState,
        key_manager_state: &KeyManagerState,
        organization_id: &id_type::OrganizationId,
        org_key: &Secret<Vec<u8>>,
        resource: &domain::Resource,
        resource_id: &id_type::ResourceId,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> RouterResult<()> {
        let db = state.store.as_ref();

        if let Some(private_key) = resource.encrypted_data.as_ref() {
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

            let key_wrapper = serde_json::to_string(&ApplePayCertificateKeyWrapper {
                data: private_key_payload,
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize Apple Pay certificate cache key wrapper")?;
            let key_wrapper: Secret<String> = Secret::new(key_wrapper);

            let identifier = km_types::Identifier::Merchant(
                organization_id
                    .as_merchant_key_identifier()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to derive organization key identifier")?,
            );

            let encrypted_cache = domain::types::crypto_operation(
                key_manager_state,
                common_utils::type_name!(domain::Resource),
                domain::types::CryptoOperation::Encrypt(key_wrapper),
                identifier,
                org_key.peek(),
            )
            .await
            .and_then(|value| value.try_into_operation())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encrypt decrypt-time certificate cache")?;

            let update = ApplePayCertificateResourceUpdate {
                apple_pay_certificates: Some(plain_data),
                apple_pay_certificates_encrypted: Some(encrypted_cache.into()),
            };
            link_resource_data_to_scope(db, requestor_type, requestor_id, &update).await?;
        }

        Ok(())
    }
}

trait RequestorResourceUpdate {
    fn for_merchant_connector_account(&self) -> domain::MerchantConnectorAccountUpdate;
    fn for_profile(&self) -> domain::ProfileUpdate;
    fn for_merchant_account(&self) -> domain::MerchantAccountUpdate;
}

struct ApplePayCertificateResourceUpdate {
    apple_pay_certificates: Option<serde_json::Value>,
    apple_pay_certificates_encrypted: Option<common_utils::encryption::Encryption>,
}

impl RequestorResourceUpdate for ApplePayCertificateResourceUpdate {
    fn for_merchant_connector_account(&self) -> domain::MerchantConnectorAccountUpdate {
        domain::MerchantConnectorAccountUpdate::ApplePayCertificateCacheUpdate {
            apple_pay_certificates: self.apple_pay_certificates.clone(),
            apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
        }
    }

    fn for_profile(&self) -> domain::ProfileUpdate {
        domain::ProfileUpdate::ApplePayCertificateCacheUpdate {
            apple_pay_certificates: self.apple_pay_certificates.clone(),
            apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
        }
    }

    fn for_merchant_account(&self) -> domain::MerchantAccountUpdate {
        domain::MerchantAccountUpdate::ApplePayCertificateCacheUpdate {
            apple_pay_certificates: self.apple_pay_certificates.clone(),
            apple_pay_certificates_encrypted: self.apple_pay_certificates_encrypted.clone(),
        }
    }
}

async fn link_resource_data_to_scope(
    db: &dyn StorageInterface,
    requestor_type: common_enums::ResourceRequestorType,
    requestor_id: String,
    update: &impl RequestorResourceUpdate,
) -> RouterResult<()> {
    let merchant_id = db
        .resolve_requestor_merchant_id(requestor_type, requestor_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to resolve requestor's owning merchant")?
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "requestor_id not found".to_string(),
        })?;

    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    match requestor_type {
        common_enums::ResourceRequestorType::MerchantConnectorAccount => {
            let mca_id = parse_requestor_id::<id_type::MerchantConnectorAccountId>(&requestor_id)?;
            link_merchant_connector_account_resource(
                db,
                &merchant_id,
                &mca_id,
                &key_store,
                update.for_merchant_connector_account(),
            )
            .await?;
        }
        common_enums::ResourceRequestorType::Profile => {
            let profile_id = parse_requestor_id::<id_type::ProfileId>(&requestor_id)?;
            let profile = db
                .find_business_profile_by_profile_id(&key_store, &profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                    id: profile_id.get_string_repr().to_owned(),
                })?;

            db.update_profile_by_profile_id(&key_store, profile, update.for_profile())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to link resource data to profile")?;
        }
        common_enums::ResourceRequestorType::MerchantAccount => {
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            db.update_merchant(merchant_account, update.for_merchant_account(), &key_store)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to link resource data to merchant account")?;
        }
    }

    Ok(())
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

#[cfg(feature = "v1")]
async fn link_merchant_connector_account_resource(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    mca_id: &id_type::MerchantConnectorAccountId,
    key_store: &domain::MerchantKeyStore,
    update: domain::MerchantConnectorAccountUpdate,
) -> RouterResult<()> {
    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            mca_id,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: mca_id.get_string_repr().to_string(),
        })?;

    db.update_merchant_connector_account(mca, update.into(), key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to link resource data to merchant connector account")?;

    Ok(())
}

#[cfg(feature = "v2")]
async fn link_merchant_connector_account_resource(
    _db: &dyn StorageInterface,
    _merchant_id: &id_type::MerchantId,
    _mca_id: &id_type::MerchantConnectorAccountId,
    _key_store: &domain::MerchantKeyStore,
    _update: domain::MerchantConnectorAccountUpdate,
) -> RouterResult<()> {
    Err(report!(errors::ApiErrorResponse::NotSupported {
        message: "Resource linking is not supported for v2".to_string(),
    }))
}

pub async fn generate_resource(
    state: SessionState,
    processor: domain::Processor,
    req: api_resources::GenerateResourceRequest,
) -> RouterResponse<api_resources::GenerateResourceResponse> {
    let response = match req.resource_type {
        common_enums::ResourceType::ApplePayCertificate => {
            ApplePayCertificateResource::generate(&state, &processor, &req).await?
        }
    };
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

pub async fn upload_resource(
    state: SessionState,
    processor: domain::Processor,
    resource_id: id_type::ResourceId,
    req: api_resources::UploadCertificateRequest,
) -> RouterResponse<api_resources::UploadCertificateResponse> {
    let db = state.store.as_ref();
    let key_manager_state: &KeyManagerState = &(&state).into();
    let organization_id = processor.get_account().organization_id.clone();

    let scope_id = db
        .find_resource_scope_id(resource_id.clone())
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "resource not found".to_string(),
        })?;
    authorize_scope_id_belongs_to_org(&scope_id, &organization_id)?;

    let org_key_store =
        ensure_organization_key_store(db, key_manager_state, &organization_id).await?;

    let resource = db
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
    };

    Ok(ApplicationResponse::Json(response))
}

fn parse_resource_type(resource: &domain::Resource) -> RouterResult<common_enums::ResourceType> {
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
        .as_utf8()?
        .to_string();

    Ok(common_name
        .rsplit(':')
        .next()
        .unwrap_or(&common_name)
        .trim()
        .to_string())
}

pub async fn list_resources(
    state: SessionState,
    organization_id: id_type::OrganizationId,
    req: api_resources::ListResourcesRequest,
) -> RouterResponse<api_resources::ListResourcesResponse> {
    let db = state.store.as_ref();
    let key_manager_state: &KeyManagerState = &(&state).into();

    let org_key_store =
        ensure_organization_key_store(db, key_manager_state, &organization_id).await?;

    let resources = db
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
            let requestor_org_id = db
                .find_requestor_organization_id(scope_type, scope_id.clone())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to resolve scope entity's organization")?
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "scope_id not found".to_string(),
                })?;
            authorize_scope_id_belongs_to_org(&requestor_org_id, &organization_id)?;

            db.resolve_effective_resource_id(scope_type, scope_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to resolve effective resource id")?
        }
        _ => None,
    };

    let resources = resources
        .iter()
        .map(|resource| resource_summary(resource, effective_resource_id.as_ref()))
        .collect::<RouterResult<Vec<_>>>()?;

    Ok(ApplicationResponse::Json(
        api_resources::ListResourcesResponse { resources },
    ))
}

fn resource_summary(
    resource: &domain::Resource,
    effective_resource_id: Option<&id_type::ResourceId>,
) -> RouterResult<api_resources::ResourceSummary> {
    let resource_type = parse_resource_type(resource)?;

    let (display_schema, display_data) = match resource_type {
        common_enums::ResourceType::ApplePayCertificate => (
            ApplePayCertificateResource::display_schema(),
            ApplePayCertificateResource::display_data(resource)?,
        ),
    };

    Ok(api_resources::ResourceSummary {
        id: resource.id.clone(),
        display_schema,
        display_data,
        created_at: resource.created_at,
        is_linked: effective_resource_id == Some(&resource.id),
    })
}

pub async fn link_resource(
    state: SessionState,
    processor: domain::Processor,
    resource_id: id_type::ResourceId,
    req: api_resources::LinkResourceRequest,
) -> RouterResponse<api_resources::LinkResourceResponse> {
    let db = state.store.as_ref();
    let key_manager_state: &KeyManagerState = &(&state).into();
    let merchant_account = processor.get_account();
    let organization_id = merchant_account.organization_id.clone();

    let scope_id = db
        .find_resource_scope_id(resource_id.clone())
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "resource not found".to_string(),
        })?;
    authorize_scope_id_belongs_to_org(&scope_id, &organization_id)?;

    let requestor_org_id = db
        .find_requestor_organization_id(req.requestor_type, req.requestor_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to resolve requestor_id's organization")?
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "requestor_id not found".to_string(),
        })?;
    authorize_scope_id_belongs_to_org(&requestor_org_id, &organization_id)?;

    let org_key_store =
        ensure_organization_key_store(db, key_manager_state, &organization_id).await?;

    let resource = db
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
                &organization_id,
                &org_key_store.key,
                &resource,
                &resource_id,
                req.requestor_type,
                req.requestor_id,
            )
            .await?;
        }
    }

    Ok(ApplicationResponse::Json(
        api_resources::LinkResourceResponse { id: resource_id },
    ))
}

fn authorize_scope_id_belongs_to_org(
    scope_id: &str,
    organization_id: &id_type::OrganizationId,
) -> RouterResult<()> {
    if scope_id != organization_id.get_string_repr() {
        return Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "resource".to_string(),
        }));
    }
    Ok(())
}

pub(crate) async fn ensure_organization_key_store(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    organization_id: &id_type::OrganizationId,
) -> RouterResult<domain::MerchantKeyStore> {
    let org_key_identifier = organization_id
        .as_merchant_key_identifier()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to derive organization key identifier")?;

    let master_key = db.get_master_key();

    match db
        .get_merchant_key_store_by_merchant_id(&org_key_identifier, &master_key.to_vec().into())
        .await
    {
        Ok(key_store) => Ok(key_store),
        Err(error) if error.current_context().is_db_not_found() => {
            create_organization_key_store(db, key_manager_state, &org_key_identifier, master_key)
                .await
        }
        Err(error) => Err(error).change_context(errors::ApiErrorResponse::InternalServerError),
    }
}

async fn create_organization_key_store(
    db: &dyn StorageInterface,
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

    db.insert_merchant_key_store(key_store.clone(), &master_key.to_vec().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert organization key store")?;

    Ok(key_store)
}
