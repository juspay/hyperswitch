pub struct ApplePayCertificateCache {
    pub data: serde_json::Value,
    pub encrypted_data: Option<common_utils::encryption::Encryption>,
}

#[cfg(feature = "v1")]
mod v1 {
    use diesel::{associations::HasTable, ExpressionMethods};

    use super::ApplePayCertificateCache;
    use crate::{
        business_profile::Profile, merchant_account::MerchantAccount,
        merchant_connector_account::MerchantConnectorAccount, query::generics,
        schema::merchant_connector_account, DatabaseConnectionWithContext, StorageResult,
    };

    async fn find_mca_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        mca_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<MerchantConnectorAccount> {
        generics::generic_find_one::<<MerchantConnectorAccount as HasTable>::Table, _, _>(
            conn,
            merchant_connector_account::dsl::merchant_connector_id.eq(mca_id.to_owned()),
        )
        .await
    }

    pub async fn resolve_requestor_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: &str,
    ) -> StorageResult<Option<common_utils::id_type::MerchantId>> {
        match requestor_type {
            common_enums::ResourceRequestorType::MerchantConnectorAccount => {
                let mca_id = match parse_id::<common_utils::id_type::MerchantConnectorAccountId>(
                    requestor_id,
                ) {
                    Ok(id) => id,
                    Err(_) => return Ok(None),
                };
                match find_mca_by_id(conn, &mca_id).await {
                    Ok(mca) => Ok(Some(mca.merchant_id)),
                    Err(error) if is_not_found(&error) => Ok(None),
                    Err(error) => Err(error),
                }
            }
            common_enums::ResourceRequestorType::Profile => {
                let profile_id = match parse_id::<common_utils::id_type::ProfileId>(requestor_id) {
                    Ok(id) => id,
                    Err(_) => return Ok(None),
                };
                match Profile::find_by_profile_id(conn, &profile_id).await {
                    Ok(profile) => Ok(Some(profile.merchant_id)),
                    Err(error) if is_not_found(&error) => Ok(None),
                    Err(error) => Err(error),
                }
            }
            common_enums::ResourceRequestorType::MerchantAccount => {
                match parse_id::<common_utils::id_type::MerchantId>(requestor_id) {
                    Ok(id) => Ok(Some(id)),
                    Err(_) => Ok(None),
                }
            }
        }
    }

    pub async fn find_requestor_organization_id(
        conn: &DatabaseConnectionWithContext<'_>,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: &str,
    ) -> StorageResult<Option<String>> {
        let Some(merchant_id) =
            resolve_requestor_merchant_id(conn, requestor_type, requestor_id).await?
        else {
            return Ok(None);
        };

        match MerchantAccount::find_by_merchant_id(conn, &merchant_id).await {
            Ok(merchant) => Ok(Some(merchant.organization_id.get_string_repr().to_string())),
            Err(error) if is_not_found(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn is_not_found(error: &error_stack::Report<crate::errors::DatabaseError>) -> bool {
        matches!(
            error.current_context(),
            crate::errors::DatabaseError::NotFound
        )
    }

    pub async fn resolve_apple_pay_certificate_cache(
        conn: &DatabaseConnectionWithContext<'_>,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: &str,
    ) -> StorageResult<Option<ApplePayCertificateCache>> {
        match requestor_type {
            common_enums::ResourceRequestorType::MerchantConnectorAccount => {
                let mca_id = match parse_id::<common_utils::id_type::MerchantConnectorAccountId>(
                    requestor_id,
                ) {
                    Ok(id) => id,
                    Err(_) => return Ok(None),
                };
                let mca = find_mca_by_id(conn, &mca_id).await?;
                if let Some(data) = mca.apple_pay_certificates {
                    return Ok(Some(ApplePayCertificateCache {
                        data,
                        encrypted_data: mca.apple_pay_certificates_encrypted,
                    }));
                }
                if let Some(profile_id) = mca.profile_id {
                    if let Ok(profile) = Profile::find_by_profile_id(conn, &profile_id).await {
                        if let Some(data) = profile.apple_pay_certificates {
                            return Ok(Some(ApplePayCertificateCache {
                                data,
                                encrypted_data: profile.apple_pay_certificates_encrypted,
                            }));
                        }
                    }
                }
                let merchant_account =
                    MerchantAccount::find_by_merchant_id(conn, &mca.merchant_id).await?;
                Ok(merchant_account
                    .apple_pay_certificates
                    .map(|data| ApplePayCertificateCache {
                        data,
                        encrypted_data: merchant_account.apple_pay_certificates_encrypted,
                    }))
            }
            common_enums::ResourceRequestorType::Profile => {
                let profile_id = match parse_id::<common_utils::id_type::ProfileId>(requestor_id) {
                    Ok(id) => id,
                    Err(_) => return Ok(None),
                };
                let profile = Profile::find_by_profile_id(conn, &profile_id).await?;
                if let Some(data) = profile.apple_pay_certificates {
                    return Ok(Some(ApplePayCertificateCache {
                        data,
                        encrypted_data: profile.apple_pay_certificates_encrypted,
                    }));
                }
                let merchant_account =
                    MerchantAccount::find_by_merchant_id(conn, &profile.merchant_id).await?;
                Ok(merchant_account
                    .apple_pay_certificates
                    .map(|data| ApplePayCertificateCache {
                        data,
                        encrypted_data: merchant_account.apple_pay_certificates_encrypted,
                    }))
            }
            common_enums::ResourceRequestorType::MerchantAccount => {
                let merchant_id = match parse_id::<common_utils::id_type::MerchantId>(requestor_id)
                {
                    Ok(id) => id,
                    Err(_) => return Ok(None),
                };
                let merchant_account =
                    MerchantAccount::find_by_merchant_id(conn, &merchant_id).await?;
                Ok(merchant_account
                    .apple_pay_certificates
                    .map(|data| ApplePayCertificateCache {
                        data,
                        encrypted_data: merchant_account.apple_pay_certificates_encrypted,
                    }))
            }
        }
    }

    pub async fn resolve_effective_resource_id(
        conn: &DatabaseConnectionWithContext<'_>,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: &str,
    ) -> StorageResult<Option<common_utils::id_type::ResourceId>> {
        let Some(cache) =
            resolve_apple_pay_certificate_cache(conn, requestor_type, requestor_id).await?
        else {
            return Ok(None);
        };
        let Some(resource_id) = cache
            .data
            .get("resource_id")
            .and_then(|value| value.as_str())
        else {
            return Ok(None);
        };
        parse_id(resource_id).map(Some)
    }

    fn parse_id<T>(value: &str) -> StorageResult<T>
    where
        T: TryFrom<
            std::borrow::Cow<'static, str>,
            Error = error_stack::Report<common_utils::errors::ValidationError>,
        >,
    {
        T::try_from(std::borrow::Cow::Owned(value.to_owned())).map_err(|error| {
            error_stack::report!(crate::errors::DatabaseError::Others)
                .attach_printable(format!("{error:?}"))
        })
    }
}

#[cfg(feature = "v1")]
pub use v1::{
    find_requestor_organization_id, resolve_apple_pay_certificate_cache,
    resolve_effective_resource_id, resolve_requestor_merchant_id,
};

#[cfg(feature = "v2")]
mod v2 {
    use super::ApplePayCertificateCache;
    use crate::{DatabaseConnectionWithContext, StorageResult};

    fn not_supported() -> StorageResult<()> {
        Err(error_stack::report!(crate::errors::DatabaseError::Others)
            .attach_printable("Apple Pay resource linking is not supported for v2"))
    }

    pub async fn find_requestor_organization_id(
        _conn: &DatabaseConnectionWithContext<'_>,
        _requestor_type: common_enums::ResourceRequestorType,
        _requestor_id: &str,
    ) -> StorageResult<Option<String>> {
        not_supported()?;
        Ok(None)
    }

    pub async fn resolve_apple_pay_certificate_cache(
        _conn: &DatabaseConnectionWithContext<'_>,
        _requestor_type: common_enums::ResourceRequestorType,
        _requestor_id: &str,
    ) -> StorageResult<Option<ApplePayCertificateCache>> {
        not_supported()?;
        Ok(None)
    }

    pub async fn resolve_effective_resource_id(
        _conn: &DatabaseConnectionWithContext<'_>,
        _requestor_type: common_enums::ResourceRequestorType,
        _requestor_id: &str,
    ) -> StorageResult<Option<common_utils::id_type::ResourceId>> {
        not_supported()?;
        Ok(None)
    }

    pub async fn resolve_requestor_merchant_id(
        _conn: &DatabaseConnectionWithContext<'_>,
        _requestor_type: common_enums::ResourceRequestorType,
        _requestor_id: &str,
    ) -> StorageResult<Option<common_utils::id_type::MerchantId>> {
        not_supported()?;
        Ok(None)
    }
}

#[cfg(feature = "v2")]
pub use v2::{
    find_requestor_organization_id, resolve_apple_pay_certificate_cache,
    resolve_effective_resource_id, resolve_requestor_merchant_id,
};
