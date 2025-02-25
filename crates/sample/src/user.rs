#[cfg(feature = "v1")]
use common_utils::types::keymanager;

#[cfg(feature = "v1")]
use hyperswitch_domain_models::{payments::{self, payment_attempt}, merchant_key_store};

#[cfg(feature = "v1")]
use diesel_models::{RefundNew, Refund, DisputeNew, Dispute, user::sample_data};

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use common_utils::types::theme::ThemeLineage;
use diesel_models::user as storage;
use crate::domain::user as domain;

#[async_trait::async_trait]
pub trait UserInterface {
    type Error;
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, Self::Error>;

    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, Self::Error>;

    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, Self::Error>;

    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, Self::Error>;

    async fn update_user_by_email(
        &self,
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, Self::Error>;

    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, Self::Error>;

    async fn find_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, Self::Error>;
}

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait BatchSampleDataInterface {
    type Error;
    #[cfg(feature = "v1")]
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        state: &keymanager::KeyManagerState,
        batch: Vec<payments::PaymentIntent>,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<payments::PaymentIntent>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<sample_data::PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<payment_attempt::PaymentAttempt>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn insert_disputes_batch_for_sample_data(
        &self,
        batch: Vec<DisputeNew>,
    ) -> CustomResult<Vec<Dispute>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_payment_intents_for_sample_data(
        &self,
        state: &keymanager::KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<payments::PaymentIntent>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<payment_attempt::PaymentAttempt>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<Refund>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_disputes_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<Dispute>, Self::Error>;
}

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait ThemeInterface {
    type Error;
    async fn insert_theme(
        &self,
        theme: storage::theme::ThemeNew,
    ) -> CustomResult<storage::theme::Theme, Self::Error>;

    async fn find_theme_by_theme_id(
        &self,
        theme_id: String,
    ) -> CustomResult<storage::theme::Theme, Self::Error>;

    async fn find_most_specific_theme_in_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::theme::Theme, Self::Error>;

    async fn find_theme_by_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::theme::Theme, Self::Error>;

    async fn delete_theme_by_lineage_and_theme_id(
        &self,
        theme_id: String,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::theme::Theme, Self::Error>;
}