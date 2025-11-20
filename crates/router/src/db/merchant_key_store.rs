pub use hyperswitch_domain_models::merchant_key_store::{self, MerchantKeyStoreInterface};

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, sync::Arc};

    use common_utils::{
        type_name,
        types::keymanager::{Identifier, KeyManagerState},
    };
    use hyperswitch_domain_models::master_key::MasterKeyInterface;
    use time::macros::datetime;
    use tokio::sync::oneshot;

    use crate::{
        db::{merchant_key_store::MerchantKeyStoreInterface, MockDb},
        routes::{
            self,
            app::{settings::Settings, StorageImpl},
        },
        services,
        types::domain,
    };

    #[tokio::test]
    async fn test_mock_db_merchant_key_store_interface() {
        let conf = Settings::new().expect("invalid settings");
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;
        let state = &Arc::new(app_state)
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
        let mock_db = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::new(),
        )
        .await
        .expect("Failed to create mock DB");
        let master_key = mock_db.get_master_key();
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(Cow::from("merchant1")).unwrap();
        let identifier = Identifier::Merchant(merchant_id.clone());
        let key_manager_state = &state.into();
        let merchant_key1 = mock_db
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.clone(),
                    key: domain::types::crypto_operation(
                        key_manager_state,
                        type_name!(domain::MerchantKeyStore),
                        domain::types::CryptoOperation::Encrypt(
                            services::generate_aes256_key().unwrap().to_vec().into(),
                        ),
                        identifier.clone(),
                        master_key,
                    )
                    .await
                    .and_then(|val| val.try_into_operation())
                    .unwrap(),
                    created_at: datetime!(2023-02-01 0:00),
                },
                &master_key.to_vec().into(),
            )
            .await
            .unwrap();

        let found_merchant_key1 = mock_db
            .get_merchant_key_store_by_merchant_id(&merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        assert_eq!(found_merchant_key1.merchant_id, merchant_key1.merchant_id);
        assert_eq!(found_merchant_key1.key, merchant_key1.key);

        let insert_duplicate_merchant_key1_result = mock_db
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.clone(),
                    key: domain::types::crypto_operation(
                        key_manager_state,
                        type_name!(domain::MerchantKeyStore),
                        domain::types::CryptoOperation::Encrypt(
                            services::generate_aes256_key().unwrap().to_vec().into(),
                        ),
                        identifier.clone(),
                        master_key,
                    )
                    .await
                    .and_then(|val| val.try_into_operation())
                    .unwrap(),
                    created_at: datetime!(2023-02-01 0:00),
                },
                &master_key.to_vec().into(),
            )
            .await;
        assert!(insert_duplicate_merchant_key1_result.is_err());

        let non_existent_merchant_id =
            common_utils::id_type::MerchantId::try_from(Cow::from("non_existent")).unwrap();

        let find_non_existent_merchant_key_result = mock_db
            .get_merchant_key_store_by_merchant_id(
                &non_existent_merchant_id,
                &master_key.to_vec().into(),
            )
            .await;
        assert!(find_non_existent_merchant_key_result.is_err());

        let find_merchant_key_with_incorrect_master_key_result = mock_db
            .get_merchant_key_store_by_merchant_id(&merchant_id, &vec![0; 32].into())
            .await;
        assert!(find_merchant_key_with_incorrect_master_key_result.is_err());
    }
}
