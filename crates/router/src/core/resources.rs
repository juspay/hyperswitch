use api_models::resources as api_resources;
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

const APPLE_PAY_CERTIFICATE_RESOURCE_TYPE: &str = "apple_pay_certificate";
const RESOURCE_SCOPE_ORGANIZATION: &str = "organization";

pub async fn generate_resource(
    state: SessionState,
    processor: domain::Processor,
    req: api_resources::GenerateResourceRequest,
) -> RouterResponse<api_resources::GenerateResourceResponse> {
    match req.resource_type.as_str() {
        APPLE_PAY_CERTIFICATE_RESOURCE_TYPE => {
            generate_apple_pay_certificate_resource(state, processor, req).await
        }
        unsupported => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!("unsupported resource type: {unsupported}"),
        })),
    }
}

async fn generate_apple_pay_certificate_resource(
    state: SessionState,
    processor: domain::Processor,
    req: api_resources::GenerateResourceRequest,
) -> RouterResponse<api_resources::GenerateResourceResponse> {
    let db = state.store.as_ref();
    let key_manager_state: &KeyManagerState = &(&state).into();
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

    let encrypted_private_key = domain::types::crypto_operation(
        key_manager_state,
        common_utils::type_name!(domain::Resource),
        domain::types::CryptoOperation::Encrypt(private_key_pem),
        identifier,
        org_key_store.key.peek(),
    )
    .await
    .and_then(|value| value.try_into_operation())
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encrypt generated Apple Pay private key")?;

    let mut data = serde_json::Map::new();
    data.insert(
        "status".to_string(),
        serde_json::Value::String("csr_generated".to_string()),
    );
    if let Some(apple_merchant_identifier) = req.apple_merchant_identifier {
        data.insert(
            "apple_pay_merchant_identifier".to_string(),
            serde_json::Value::String(apple_merchant_identifier),
        );
    }

    let resource_id = id_type::ResourceId::default();
    let resource = domain::Resource {
        id: resource_id.clone(),
        resource_type: APPLE_PAY_CERTIFICATE_RESOURCE_TYPE.to_string(),
        scope: RESOURCE_SCOPE_ORGANIZATION.to_string(),
        scope_id: organization_id.get_string_repr().to_string(),
        data: serde_json::Value::Object(data),
        encrypted_data: Some(encrypted_private_key),
        created_by,
        created_at: date_time::now(),
        modified_at: date_time::now(),
    };

    db.insert_linked_resource(resource, &org_key_store.key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to persist generated Apple Pay certificate resource")?;

    Ok(ApplicationResponse::Json(
        api_resources::GenerateResourceResponse {
            id: resource_id,
            data: serde_json::json!({
                APPLE_PAY_CERTIFICATE_RESOURCE_TYPE: { "csr": csr_pem },
            }),
        },
    ))
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

pub async fn upload_apple_pay_certificate(
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

    let private_key_pem = resource
        .encrypted_data
        .as_ref()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Resource has no private key — cannot verify uploaded certificate")?
        .peek()
        .clone();

    let certificate = parse_certificate_from_base64_der(&req.certificate)?;

    verify_certificate_matches_private_key(&certificate, &private_key_pem)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Uploaded certificate does not match the generated key".to_string(),
        })?;

    let apple_merchant_identifier = parse_apple_merchant_identifier(&certificate)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Could not parse Apple merchant identifier from certificate".to_string(),
        })?;

    let mut data = match resource.data {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    data.insert(
        "payment_processing_certificate".to_string(),
        serde_json::Value::String(req.certificate),
    );
    data.insert(
        "apple_pay_merchant_identifier".to_string(),
        serde_json::Value::String(apple_merchant_identifier.clone()),
    );
    data.insert(
        "status".to_string(),
        serde_json::Value::String("active".to_string()),
    );

    let updated = db
        .update_linked_resource_data(
            resource_id.clone(),
            domain::ResourceDataUpdate {
                data: serde_json::Value::Object(data),
            },
            &org_key_store.key,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to persist uploaded Apple Pay certificate")?;

    Ok(ApplicationResponse::Json(
        api_resources::UploadCertificateResponse {
            status: resource_status(&updated),
            id: updated.id,
            created_at: updated.created_at,
        },
    ))
}

fn resource_status(resource: &domain::Resource) -> String {
    resource
        .data
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("csr_generated")
        .to_string()
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

fn verify_certificate_matches_private_key(
    certificate: &X509,
    private_key_pem: &str,
) -> error_stack::Result<(), openssl::error::ErrorStack> {
    let certificate_public_key_der = certificate.public_key()?.public_key_to_der()?;

    let private_key = PKey::private_key_from_pem(private_key_pem.as_bytes())?;
    let private_key_public_der = private_key.public_key_to_der()?;

    if certificate_public_key_der != private_key_public_der {
        return Err(report!(openssl::error::ErrorStack::get()));
    }
    Ok(())
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
            req.resource_type.clone(),
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

    Ok(ApplicationResponse::Json(
        api_resources::ListResourcesResponse {
            resources: resources
                .iter()
                .map(|resource| resource_summary(resource, effective_resource_id.as_ref()))
                .collect(),
        },
    ))
}

fn apple_merchant_identifier(resource: &domain::Resource) -> Option<String> {
    resource
        .data
        .get("apple_pay_merchant_identifier")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn display_schema(resource_type: &str) -> serde_json::Value {
    match resource_type {
        APPLE_PAY_CERTIFICATE_RESOURCE_TYPE => {
            serde_json::json!({ "apple_pay_merchant_identifier": "String" })
        }
        _ => serde_json::json!({}),
    }
}

fn display_data(resource: &domain::Resource) -> serde_json::Value {
    match resource.resource_type.as_str() {
        APPLE_PAY_CERTIFICATE_RESOURCE_TYPE => serde_json::json!({
            "apple_pay_merchant_identifier": apple_merchant_identifier(resource),
        }),
        _ => serde_json::json!({}),
    }
}

fn resource_summary(
    resource: &domain::Resource,
    effective_resource_id: Option<&id_type::ResourceId>,
) -> api_resources::ResourceSummary {
    api_resources::ResourceSummary {
        id: resource.id.clone(),
        display_schema: display_schema(&resource.resource_type),
        display_data: display_data(resource),
        status: resource_status(resource),
        created_at: resource.created_at,
        is_linked: effective_resource_id == Some(&resource.id),
    }
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

    cache_apple_pay_certificate_on_link(
        db,
        key_manager_state,
        &organization_id,
        &resource_id,
        req.requestor_type,
        req.requestor_id,
    )
    .await?;

    Ok(ApplicationResponse::Json(
        api_resources::LinkResourceResponse {
            id: resource_id,
            status: "success".to_string(),
        },
    ))
}

async fn cache_apple_pay_certificate_on_link(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    organization_id: &id_type::OrganizationId,
    resource_id: &id_type::ResourceId,
    requestor_type: common_enums::ResourceRequestorType,
    requestor_id: String,
) -> RouterResult<()> {
    let org_key_store =
        ensure_organization_key_store(db, key_manager_state, organization_id).await?;

    let resource = db
        .find_linked_resource_by_id(resource_id.clone(), &org_key_store.key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch resource for decrypt-time cache")?;

    if resource.resource_type != APPLE_PAY_CERTIFICATE_RESOURCE_TYPE {
        return Ok(());
    }
    let Some(private_key) = resource.encrypted_data.as_ref() else {
        return Ok(());
    };

    let plain_data = serde_json::json!({
        "resource_id": resource_id,
        "data": {
            "merchant_identifier": resource.data.get("apple_pay_merchant_identifier"),
            "payment_processing_certificate": resource.data.get("payment_processing_certificate"),
        }
    });

    let key_wrapper = serde_json::json!({
        "data": { "payment_processing_certificate_key": private_key.peek() }
    })
    .to_string();
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
        org_key_store.key.peek(),
    )
    .await
    .and_then(|value| value.try_into_operation())
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encrypt decrypt-time certificate cache")?;

    db.set_apple_pay_certificate_cache(
        requestor_type,
        requestor_id,
        plain_data,
        encrypted_cache.into(),
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "requestor_id not found".to_string(),
    })?;

    Ok(())
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
            create_organization_key_store(
                db,
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
            key: hyperswitch_masking::StrongSecret::new(
                crate::consts::BASE64_ENGINE.encode(key),
            ),
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
