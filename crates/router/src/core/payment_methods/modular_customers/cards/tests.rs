//! Synthetic storage backends cover duplicate cards, partial writes and resumed attempts.
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::*;

pub(crate) fn stored() -> StoredCard {
    StoredCard {
        payment_method_id: "0a_pm_01926c58bc6e77c09e809964e72af8c8".to_string(),
    }
}

pub(crate) struct FakeCards {
    finds: tokio::sync::Mutex<VecDeque<Result<Option<StoredCard>, CardError>>>,
    attempts: tokio::sync::Mutex<HashMap<String, Attempt>>,
    create_result: tokio::sync::Mutex<Result<StoredCard, CardError>>,
    pub(crate) creates: AtomicUsize,
    locked: AtomicBool,
    writes: AtomicUsize,
    fail_write_at: usize,
    create_delay: Option<std::time::Duration>,
}

impl FakeCards {
    pub(crate) fn new(
        finds: Vec<Result<Option<StoredCard>, CardError>>,
        create_result: Result<StoredCard, CardError>,
    ) -> Self {
        Self {
            finds: tokio::sync::Mutex::new(finds.into()),
            attempts: tokio::sync::Mutex::new(HashMap::new()),
            create_result: tokio::sync::Mutex::new(create_result),
            creates: AtomicUsize::new(0),
            locked: AtomicBool::new(false),
            writes: AtomicUsize::new(0),
            fail_write_at: 0,
            create_delay: None,
        }
    }

    pub(crate) async fn set_create_result(&self, result: Result<StoredCard, CardError>) {
        *self.create_result.lock().await = result;
    }
}

#[async_trait::async_trait]
impl CardBackend for FakeCards {
    async fn forget_rejected(&self, key: &str) -> Result<(), CardError> {
        self.attempts.lock().await.remove(key);
        Ok(())
    }
    async fn lock(&self, _: &str) -> Result<(), CardError> {
        self.locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| CardError::LockUnavailable)
    }
    async fn unlock(&self, _: &str) -> Result<(), CardError> {
        self.locked.store(false, Ordering::SeqCst);
        Ok(())
    }
    async fn attempt(&self, key: &str) -> Result<Option<Attempt>, CardError> {
        Ok(self.attempts.lock().await.get(key).cloned())
    }
    async fn remember(&self, key: &str, attempt: Attempt) -> Result<(), CardError> {
        if self.writes.fetch_add(1, Ordering::SeqCst) + 1 == self.fail_write_at {
            return Err(CardError::JournalUnavailable);
        }
        self.attempts.lock().await.insert(key.to_string(), attempt);
        Ok(())
    }
    async fn find(
        &self,
        _: &id_type::ProfileId,
        _: &id_type::GlobalCustomerId,
        _: &MigrationCardInput,
    ) -> Result<Option<StoredCard>, CardError> {
        tokio::task::yield_now().await;
        self.finds.lock().await.pop_front().expect("expected find")
    }
    async fn create(
        &self,
        _: &id_type::ProfileId,
        _: &id_type::GlobalCustomerId,
        _: &MigrationCardInput,
    ) -> Result<StoredCard, CardError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        if let Some(delay) = self.create_delay {
            tokio::time::sleep(delay).await;
        }
        self.create_result.lock().await.clone()
    }
}

fn customer() -> id_type::GlobalCustomerId {
    id_type::GlobalCustomerId::new_unchecked("0a_cus_01926c58bc6e77c09e809964e72af8c8".to_string())
}

#[tokio::test]
async fn lost_create_response_is_reconciled_to_one_permanent_card() {
    let row = super::super::tests::input();
    let backend = FakeCards::new(
        vec![Ok(None), Ok(Some(stored()))],
        Err(CardError::ServiceUnavailable),
    );
    let result = store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card"),
    )
    .await
    .expect("reconciled");
    assert_eq!(result.0, stored());
    assert!(!result.1);
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert!(!backend.locked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn uncertain_post_never_reposts_after_lock_expiry() {
    let row = super::super::tests::input();
    let backend = FakeCards::new(
        vec![Ok(None), Ok(None), Ok(None), Ok(Some(stored()))],
        Err(CardError::ServiceUnavailable),
    );
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::UncertainStorage)
    );
    assert!(backend.locked.load(Ordering::SeqCst));
    backend.locked.store(false, Ordering::SeqCst); // Redis lease has expired.
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::UncertainStorage)
    );
    assert!(store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card")
    )
    .await
    .is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn concurrent_card_requests_create_at_most_once() {
    let row = super::super::tests::input();
    let backend = FakeCards::new(
        vec![Ok(None), Ok(Some(stored())), Ok(Some(stored()))],
        Ok(stored()),
    );
    let c = customer();
    let a = store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &c,
        row.card.as_ref().expect("card"),
    );
    let b = store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &c,
        row.card.as_ref().expect("card"),
    );
    let (a, b) = tokio::join!(a, b);
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert!(store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card")
    )
    .await
    .is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn journal_failure_before_post_never_creates_a_card() {
    let row = super::super::tests::input();
    let mut backend = FakeCards::new(vec![Ok(None)], Ok(stored()));
    backend.fail_write_at = 1;
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::JournalUnavailable)
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    assert!(!backend.locked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn journal_failure_after_post_is_reconciled_without_reposting() {
    let row = super::super::tests::input();
    let mut backend = FakeCards::new(vec![Ok(None), Ok(Some(stored()))], Ok(stored()));
    backend.fail_write_at = 2;
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::JournalWriteUncertain)
    );
    assert!(backend.locked.load(Ordering::SeqCst));
    backend.locked.store(false, Ordering::SeqCst);
    assert!(store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card")
    )
    .await
    .is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn definite_auth_rejection_clears_pending_attempt_for_safe_retry() {
    let row = super::super::tests::input();
    let backend = FakeCards::new(
        vec![Ok(None), Ok(None), Ok(Some(stored()))],
        Err(CardError::AuthRejected),
    );
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::AuthRejected)
    );
    assert!(backend.attempt("key").await.expect("journal").is_none());
    assert!(!backend.locked.load(Ordering::SeqCst));
    backend.set_create_result(Ok(stored())).await;
    assert!(store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card")
    )
    .await
    .is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn cancellation_after_post_preserves_attempt_and_prevents_repost() {
    let row = super::super::tests::input();
    let mut backend = FakeCards::new(vec![Ok(None), Ok(None)], Ok(stored()));
    backend.create_delay = Some(std::time::Duration::from_secs(1));
    assert!(tokio::time::timeout(
        std::time::Duration::from_millis(10),
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
    )
    .await
    .is_err());
    assert!(backend.locked.load(Ordering::SeqCst));
    assert!(backend.attempt("key").await.expect("journal").is_some());
    backend.locked.store(false, Ordering::SeqCst);
    assert_eq!(
        store(
            &backend,
            "key",
            "digest",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::UncertainStorage)
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn changed_payload_is_rejected_instead_of_reusing_a_duplicate() {
    let row = super::super::tests::input();
    let backend = FakeCards::new(vec![Ok(None), Ok(Some(stored()))], Ok(stored()));
    assert!(store(
        &backend,
        "key",
        "digest",
        &row.profile_id,
        &customer(),
        row.card.as_ref().expect("card")
    )
    .await
    .is_ok());
    assert_eq!(
        store(
            &backend,
            "key",
            "changed",
            &row.profile_id,
            &customer(),
            row.card.as_ref().expect("card")
        )
        .await,
        Err(CardError::PayloadConflict)
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[test]
fn same_pan_is_serialized_across_profiles_without_persisting_pan_or_pii() {
    let row = super::super::tests::input();
    let mid = id_type::MerchantId::wrap("merchant_one".to_string()).expect("merchant");
    let (key, digest) = identity(
        "tenant",
        &mid,
        &customer(),
        row.card.as_ref().expect("card"),
        &[1; 32],
    )
    .expect("identity");
    let journal = serde_json::to_string(&Attempt {
        payload_digest: digest,
        payment_method_id: Some(stored().payment_method_id),
    })
    .expect("journal");
    assert!(!key.contains("4242424242424242"));
    assert!(!journal.contains("4242424242424242"));
    assert!(!journal.contains("Different Cardholder"));
    assert!(!journal.contains("customer@example.com"));
    let changed_tenant = identity(
        "another_tenant",
        &mid,
        &customer(),
        row.card.as_ref().expect("card"),
        &[1; 32],
    )
    .expect("identity");
    assert_ne!(key, changed_tenant.0);
}

#[test]
fn stored_card_must_preserve_supplied_metadata_and_billing() {
    let row = super::super::tests::input();
    let card = row.card.expect("card");
    assert!(verify_fields(&card, &card.card, card.subtype, card.billing.as_ref()).is_ok());
    let mut changed = card.card.clone();
    changed.card_holder_name = Some(hyperswitch_masking::Secret::new("Wrong Name".to_string()));
    assert_eq!(
        verify_fields(&card, &changed, card.subtype, card.billing.as_ref()),
        Err(CardError::PayloadConflict)
    );
    let mut changed = card.card.clone();
    changed.card_exp_year = hyperswitch_masking::Secret::new("2098".to_string());
    assert_eq!(
        verify_fields(&card, &changed, card.subtype, card.billing.as_ref()),
        Err(CardError::PayloadConflict)
    );
}

#[test]
fn billing_enrichment_is_allowed_but_supplied_fields_cannot_be_lost() {
    use api_models::payments::{Address, AddressDetails};
    let expected = Address {
        address: Some(AddressDetails {
            line1: Some(hyperswitch_masking::Secret::new(
                "Original line".to_string(),
            )),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut actual = expected.clone();
    actual.address.as_mut().expect("address").first_name = Some(hyperswitch_masking::Secret::new(
        "Enriched name".to_string(),
    ));
    assert!(billing_matches(&expected, Some(&actual)));
    actual.address.as_mut().expect("address").line1 = None;
    assert!(!billing_matches(&expected, Some(&actual)));
    assert!(!billing_matches(&expected, None));
}

#[test]
fn reconciliation_verifies_full_pan_merchant_customer_and_permanent_storage() {
    use payment_methods::client::retrieve::RetrievePaymentMethodResponse;
    let row = super::super::tests::input();
    let card = row.card.expect("card");
    let merchant = id_type::MerchantId::wrap("merchant_one".to_string()).expect("merchant");
    let id = stored().payment_method_id;
    let mut response = RetrievePaymentMethodResponse {
        payment_method_id: id.clone(),
        merchant_id: merchant.clone(),
        customer_id: Some(
            id_type::CustomerId::wrap(customer().get_string_repr().to_string()).expect("customer"),
        ),
        payment_method: common_enums::PaymentMethod::Card,
        payment_method_type: card.subtype,
        recurring_enabled: None,
        created: None,
        last_used_at: None,
        payment_method_data: None,
        connector_tokens: None,
        network_token: None,
        network_transaction_id: None,
        raw_payment_method_data: Some(RawPaymentMethodData::Card(card.card.clone())),
        billing: card.billing.clone().map(Into::into),
        storage_type: Some(common_enums::StorageType::Persistent),
    };
    assert_eq!(
        matches_stored_card(&merchant, &customer(), &id, &card, &response),
        Ok(true)
    );
    let mut other = card.card.clone();
    other.card_number = "4000000000024242".parse().expect("valid different PAN");
    assert_eq!(
        other.card_number.get_last4(),
        card.card.card_number.get_last4()
    );
    response.raw_payment_method_data = Some(RawPaymentMethodData::Card(other));
    assert_eq!(
        matches_stored_card(&merchant, &customer(), &id, &card, &response),
        Ok(false)
    );
    response.raw_payment_method_data = Some(RawPaymentMethodData::Card(card.card.clone()));
    response.storage_type = Some(common_enums::StorageType::Volatile);
    assert_eq!(
        matches_stored_card(&merchant, &customer(), &id, &card, &response),
        Err(CardError::InvalidResponse)
    );
    response.storage_type = Some(common_enums::StorageType::Persistent);
    response.customer_id =
        Some(id_type::CustomerId::wrap("wrong_customer".to_string()).expect("customer"));
    assert_eq!(
        matches_stored_card(&merchant, &customer(), &id, &card, &response),
        Err(CardError::InvalidResponse)
    );
}
