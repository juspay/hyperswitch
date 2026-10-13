//! Permanent raw-card storage using existing modular create/list/retrieve transport.
//! Redis attempt records contain only keyed digests and IDs, never card/customer details.
//! Separate matching and recovery from transport so resumed rows can verify existing storage.

use common_utils::request::Headers;
use hyperswitch_masking::{Maskable, PeekInterface};
use payment_methods::{
    client::{
        list::ListCustomerPaymentMethodsV1Request,
        migration::{
            CreateMigrationPaymentMethod, CreateMigrationPaymentMethodRequest,
            ListMigrationCustomerPaymentMethods, MigrationCardInput,
        },
        retrieve::{RetrievePaymentMethod, RetrievePaymentMethodV1Request},
        PaymentMethodClient,
    },
    types::{PaymentMethodResponseData, RawPaymentMethodData},
};
use serde::{Deserialize, Serialize};

use super::{id_type, is_modular_id, ApiIdentifier, LockAction, LockingInput, SessionState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardError {
    ServiceUnavailable,
    InvalidResponse,
    PayloadConflict,
    AmbiguousCard,
    UncertainStorage,
    LockUnavailable,
    JournalUnavailable,
    JournalWriteUncertain,
    AuthRejected,
}

impl CardError {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::ServiceUnavailable => "migration_card_service_unavailable",
            Self::InvalidResponse => "migration_card_response_invalid",
            Self::PayloadConflict => "migration_card_payload_conflict",
            Self::AmbiguousCard => "migration_card_ambiguous",
            Self::UncertainStorage => "migration_card_storage_uncertain",
            Self::LockUnavailable => "migration_card_lock_unavailable",
            Self::JournalUnavailable | Self::JournalWriteUncertain => {
                "migration_card_journal_unavailable"
            }
            Self::AuthRejected => "migration_card_auth_rejected",
        }
    }

    pub(super) fn retryable(self) -> bool {
        matches!(
            self,
            Self::ServiceUnavailable
                | Self::UncertainStorage
                | Self::LockUnavailable
                | Self::JournalUnavailable
                | Self::JournalWriteUncertain
                | Self::AuthRejected
        )
    }

    pub(super) fn uncertain(self) -> bool {
        matches!(self, Self::UncertainStorage | Self::JournalWriteUncertain)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Attempt {
    pub payload_digest: String,
    pub payment_method_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StoredCard {
    pub payment_method_id: String,
}

#[async_trait::async_trait]
// Keep storage decisions separate from the existing modular service and Redis adapters.
pub(super) trait CardBackend: Sync {
    async fn lock(&self, key: &str) -> Result<(), CardError>;
    async fn unlock(&self, key: &str) -> Result<(), CardError>;
    async fn attempt(&self, key: &str) -> Result<Option<Attempt>, CardError>;
    async fn remember(&self, key: &str, attempt: Attempt) -> Result<(), CardError>;
    async fn forget_rejected(&self, key: &str) -> Result<(), CardError>;
    async fn find(
        &self,
        profile: &id_type::ProfileId,
        customer: &id_type::GlobalCustomerId,
        card: &MigrationCardInput,
    ) -> Result<Option<StoredCard>, CardError>;
    async fn create(
        &self,
        profile: &id_type::ProfileId,
        customer: &id_type::GlobalCustomerId,
        card: &MigrationCardInput,
    ) -> Result<StoredCard, CardError>;
}

pub(super) fn identity(
    tenant: &str,
    merchant: &id_type::MerchantId,
    customer: &id_type::GlobalCustomerId,
    card: &MigrationCardInput,
    hash_key: &[u8; 32],
) -> Result<(String, String), CardError> {
    // Cards are scoped to the merchant/customer, not a profile, in the modular database.
    let scoped = format!(
        "card-migration-v1\0{tenant}\0{}\0{}\0{}",
        merchant.get_string_repr(),
        customer.get_string_repr(),
        card.card.card_number.get_card_no()
    );
    let key = format!(
        "MODULAR_CARD_MIGRATION_{}",
        blake3::keyed_hash(hash_key, scoped.as_bytes()).to_hex()
    );
    // A second digest detects changed expiry, cardholder or billing data for the same PAN.
    let payload = serde_json::to_vec(&card.payload(customer.clone()))
        .map_err(|_| CardError::PayloadConflict)?;
    let digest = blake3::keyed_hash(hash_key, &payload).to_hex().to_string();
    Ok((key, digest))
}

pub(super) async fn store<B: CardBackend>(
    backend: &B,
    key: &str,
    digest: &str,
    profile: &id_type::ProfileId,
    customer: &id_type::GlobalCustomerId,
    card: &MigrationCardInput,
) -> Result<(StoredCard, bool), CardError> {
    // Serialize migration attempts for this customer/card across requests and profiles.
    backend.lock(key).await?;
    let mut posted = false;
    let result = async {
        let attempt = backend.attempt(key).await?;
        if attempt
            .as_ref()
            .is_some_and(|attempt| attempt.payload_digest != digest)
        {
            return Err(CardError::PayloadConflict);
        }
        // A verified existing card is reused; a remembered different ID is ambiguous.
        if let Some(found) = backend.find(profile, customer, card).await? {
            if attempt
                .as_ref()
                .and_then(|a| a.payment_method_id.as_ref())
                .is_some_and(|id| id != &found.payment_method_id)
            {
                return Err(CardError::AmbiguousCard);
            }
            backend
                .remember(
                    key,
                    Attempt {
                        payload_digest: digest.to_string(),
                        payment_method_id: Some(found.payment_method_id.clone()),
                    },
                )
                .await?;
            return Ok((found, false));
        }
        if attempt.is_some() {
            // Even after the short lock expires, an unresolved attempt is read-only on retry.
            // A timeout/absence from the active list does not prove the previous POST failed.
            return Err(CardError::UncertainStorage);
        }
        // Record the attempt before creating so a later application-level retry can reconcile.
        backend
            .remember(
                key,
                Attempt {
                    payload_digest: digest.to_string(),
                    payment_method_id: None,
                },
            )
            .await?;
        posted = true;
        let created = backend.create(profile, customer, card).await;
        if matches!(created, Err(CardError::AuthRejected)) {
            // These statuses come from the modular route's authentication middleware,
            // before its create closure. Do not generalize this to other 4xx/5xx responses.
            backend.forget_rejected(key).await?;
            posted = false;
            return Err(CardError::AuthRejected);
        }
        if let Ok(created) = &created {
            backend
                .remember(
                    key,
                    Attempt {
                        payload_digest: digest.to_string(),
                        payment_method_id: Some(created.payment_method_id.clone()),
                    },
                )
                .await?;
        }
        // Confirm storage through reads even when the create response was lost.
        let confirmed = backend
            .find(profile, customer, card)
            .await
            .map_err(|_| CardError::UncertainStorage)?;
        let confirmed = confirmed.ok_or(CardError::UncertainStorage)?;
        if created.as_ref().is_ok_and(|created| created != &confirmed) {
            return Err(CardError::UncertainStorage);
        }
        backend
            .remember(
                key,
                Attempt {
                    payload_digest: digest.to_string(),
                    payment_method_id: Some(confirmed.payment_method_id.clone()),
                },
            )
            .await?;
        Ok((confirmed, created.is_ok()))
    }
    .await;
    let result = result.map_err(|error| {
        if posted && error == CardError::JournalUnavailable {
            CardError::JournalWriteUncertain
        } else {
            error
        }
    });
    // Unknown writes retain the lease. The non-expiring journal also survives its expiry.
    if result.is_ok() || !posted {
        backend.unlock(key).await?;
    }
    result
}

pub(super) struct ServiceCardBackend<'a> {
    pub state: &'a SessionState,
    pub merchant: &'a id_type::MerchantId,
}

fn headers(
    state: &SessionState,
    merchant: &id_type::MerchantId,
    profile: &id_type::ProfileId,
) -> Headers {
    // Card calls reuse internal merchant/profile auth; customer calls use the temporary key.
    let mut headers = Headers::new();
    headers.insert((
        crate::headers::X_MERCHANT_ID.to_string(),
        Maskable::Normal(merchant.get_string_repr().to_string()),
    ));
    headers.insert((
        crate::headers::X_PROFILE_ID.to_string(),
        Maskable::Normal(profile.get_string_repr().to_string()),
    ));
    headers.insert((
        crate::headers::X_INTERNAL_API_KEY.to_string(),
        Maskable::Masked(
            state
                .conf
                .internal_merchant_id_profile_id_auth
                .internal_api_key
                .clone(),
        ),
    ));
    // PaymentMethodClient forwards the configured tenant and trace ID through the existing pipeline.
    headers
}

impl ServiceCardBackend<'_> {
    fn lock_action(&self, key: &str) -> LockAction {
        LockAction::Hold {
            input: LockingInput {
                unique_locking_key: key.to_string(),
                api_identifier: ApiIdentifier::Customers,
                override_lock_retries: None,
            },
        }
    }

    async fn find_stored(
        &self,
        profile: &id_type::ProfileId,
        customer: &id_type::GlobalCustomerId,
        card: &MigrationCardInput,
    ) -> Result<Option<StoredCard>, CardError> {
        let headers = headers(self.state, self.merchant, profile);
        let client = PaymentMethodClient::new(
            &self.state.conf.micro_services.payment_methods_base_url,
            &headers,
            &self.state.conf.trace_header.header_name,
        );
        // Fresh permanent cards are New, so the migration list explicitly includes that status.
        let listed = ListMigrationCustomerPaymentMethods::call(
            self.state,
            &client,
            ListCustomerPaymentMethodsV1Request {
                customer_id: customer.clone(),
                query_params: api_models::payment_methods::PaymentMethodListRequest::default(),
                modular_service_prefix: self
                    .state
                    .conf
                    .micro_services
                    .payment_methods_prefix
                    .0
                    .clone(),
            },
        )
        .await
        .map_err(|_| CardError::ServiceUnavailable)?;
        let mut found = None;
        for item in listed.0.customer_payment_methods {
            if item.customer_id.get_string_repr() != customer.get_string_repr() {
                return Err(CardError::InvalidResponse);
            }
            if item.payment_method_type != common_enums::PaymentMethod::Card {
                continue;
            }
            // Last four is only a prefilter, never proof of card identity.
            if let Some(PaymentMethodResponseData::Card(data)) = &item.payment_method_data {
                if data
                    .last4_digits
                    .as_ref()
                    .is_some_and(|last4| last4 != &card.card.card_number.get_last4())
                {
                    continue;
                }
            }
            if !is_modular_id(&item.id, "pm") {
                return Err(CardError::InvalidResponse);
            }
            // Read full details for identity verification; masked listing fields are insufficient.
            let retrieved = RetrievePaymentMethod::call(
                self.state,
                &client,
                RetrievePaymentMethodV1Request {
                    payment_method_id: api_models::payment_methods::PaymentMethodId {
                        payment_method_id: item.id.clone(),
                    },
                    fetch_raw_detail: true,
                    force_sync: false,
                    modular_service_prefix: self
                        .state
                        .conf
                        .micro_services
                        .payment_methods_prefix
                        .0
                        .clone(),
                },
            )
            .await
            .map_err(|_| CardError::ServiceUnavailable)?;
            if !matches_stored_card(self.merchant, customer, &item.id, card, &retrieved)? {
                continue;
            }
            if found.is_some() {
                return Err(CardError::AmbiguousCard);
            }
            found = Some(StoredCard {
                payment_method_id: item.id,
            });
        }
        Ok(found)
    }
}

fn matches_stored_card(
    merchant: &id_type::MerchantId,
    customer: &id_type::GlobalCustomerId,
    payment_method_id: &str,
    card: &MigrationCardInput,
    retrieved: &payment_methods::client::retrieve::RetrievePaymentMethodResponse,
) -> Result<bool, CardError> {
    // Validate response ownership and permanent storage before trusting its card data.
    if !is_modular_id(payment_method_id, "pm")
        || retrieved.payment_method_id != payment_method_id
        || retrieved.merchant_id != *merchant
        || retrieved
            .customer_id
            .as_ref()
            .map(|id| id.get_string_repr())
            != Some(customer.get_string_repr())
        || retrieved.payment_method != common_enums::PaymentMethod::Card
        || retrieved.storage_type != Some(common_enums::StorageType::Persistent)
    {
        return Err(CardError::InvalidResponse);
    }
    let raw = match retrieved.raw_payment_method_data.as_ref() {
        Some(RawPaymentMethodData::Card(raw)) => raw,
        Some(RawPaymentMethodData::CardWithNT(raw)) => &raw.card_details,
        _ => return Err(CardError::InvalidResponse),
    };
    if raw.card_number.get_card_no() != card.card.card_number.get_card_no() {
        return Ok(false);
    }
    let billing = retrieved
        .billing
        .as_ref()
        .map(|address| address.clone().into());
    verify_fields(card, raw, retrieved.payment_method_type, billing.as_ref())?;
    Ok(true)
}

fn verify_fields(
    card: &MigrationCardInput,
    raw: &payment_methods::client::create::CardDetail,
    subtype: Option<common_enums::PaymentMethodType>,
    billing: Option<&api_models::payments::Address>,
) -> Result<(), CardError> {
    // Compare every supplied field while allowing the service to enrich missing metadata.
    let month = |value: &hyperswitch_masking::Secret<String>| value.peek().parse::<u8>().ok();
    let year = |value: &hyperswitch_masking::Secret<String>| {
        value
            .peek()
            .parse::<u16>()
            .ok()
            .map(|v| if v < 100 { v + 2000 } else { v })
    };
    if month(&raw.card_exp_month) != month(&card.card.card_exp_month)
        || year(&raw.card_exp_year) != year(&card.card.card_exp_year)
        || card.card.card_holder_name.as_ref().is_some_and(|name| {
            raw.card_holder_name.as_ref().map(|n| n.peek()) != Some(name.peek())
        })
        || card
            .card
            .nick_name
            .as_ref()
            .is_some_and(|name| raw.nick_name.as_ref().map(|n| n.peek()) != Some(name.peek()))
        || card
            .card
            .card_network
            .as_ref()
            .is_some_and(|network| raw.card_network.as_ref() != Some(network))
        || card
            .subtype
            .is_some_and(|expected| subtype != Some(expected))
        || card
            .billing
            .as_ref()
            .is_some_and(|expected| !billing_matches(expected, billing))
    {
        return Err(CardError::PayloadConflict);
    }
    Ok(())
}

fn billing_matches(
    expected: &api_models::payments::Address,
    actual: Option<&api_models::payments::Address>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    fn matches<T: PartialEq>(expected: &Option<T>, actual: &Option<T>) -> bool {
        expected
            .as_ref()
            .is_none_or(|expected| actual.as_ref() == Some(expected))
    }
    if !matches(&expected.email, &actual.email) || !matches(&expected.phone, &actual.phone) {
        return false;
    }
    let Some(expected) = expected.address.as_ref() else {
        return true;
    };
    let Some(actual) = actual.address.as_ref() else {
        return false;
    };
    // Enrichment of absent fields is allowed; every supplied field must still match.
    matches(&expected.city, &actual.city)
        && matches(&expected.country, &actual.country)
        && matches(&expected.line1, &actual.line1)
        && matches(&expected.line2, &actual.line2)
        && matches(&expected.line3, &actual.line3)
        && matches(&expected.zip, &actual.zip)
        && matches(&expected.state, &actual.state)
        && matches(&expected.first_name, &actual.first_name)
        && matches(&expected.last_name, &actual.last_name)
        && matches(&expected.origin_zip, &actual.origin_zip)
}

#[async_trait::async_trait]
impl CardBackend for ServiceCardBackend<'_> {
    async fn forget_rejected(&self, key: &str) -> Result<(), CardError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CardError::JournalUnavailable)?;
        match redis
            .delete_key(&redis_interface::RedisKey::from(key))
            .await
            .map_err(|_| CardError::JournalUnavailable)?
        {
            redis_interface::types::DelReply::KeyDeleted => Ok(()),
            redis_interface::types::DelReply::KeyNotDeleted => Err(CardError::JournalUnavailable),
        }
    }
    async fn lock(&self, key: &str) -> Result<(), CardError> {
        self.lock_action(key)
            .perform_locking_action(self.state, self.merchant.clone())
            .await
            .map_err(|_| CardError::LockUnavailable)
    }
    async fn unlock(&self, key: &str) -> Result<(), CardError> {
        self.lock_action(key)
            .free_lock_action(self.state, self.merchant.clone())
            .await
            .map_err(|_| CardError::LockUnavailable)
    }
    async fn attempt(&self, key: &str) -> Result<Option<Attempt>, CardError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CardError::JournalUnavailable)?;
        let value: Option<String> = redis
            .get_key(&redis_interface::RedisKey::from(key))
            .await
            .map_err(|_| CardError::JournalUnavailable)?;
        value
            .map(|value| serde_json::from_str(&value).map_err(|_| CardError::JournalUnavailable))
            .transpose()
    }
    async fn remember(&self, key: &str, attempt: Attempt) -> Result<(), CardError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CardError::JournalUnavailable)?;
        let value = serde_json::to_string(&attempt).map_err(|_| CardError::JournalUnavailable)?;
        // KEEPTTL creates new keys without expiry. Do not use set_key: it adds the default TTL.
        redis
            .set_key_without_modifying_ttl(&redis_interface::RedisKey::from(key), value)
            .await
            .map_err(|_| CardError::JournalUnavailable)
    }
    async fn find(
        &self,
        profile: &id_type::ProfileId,
        customer: &id_type::GlobalCustomerId,
        card: &MigrationCardInput,
    ) -> Result<Option<StoredCard>, CardError> {
        self.find_stored(profile, customer, card).await
    }
    async fn create(
        &self,
        profile: &id_type::ProfileId,
        customer: &id_type::GlobalCustomerId,
        card: &MigrationCardInput,
    ) -> Result<StoredCard, CardError> {
        let headers = headers(self.state, self.merchant, profile);
        let client = PaymentMethodClient::new(
            &self.state.conf.micro_services.payment_methods_base_url,
            &headers,
            &self.state.conf.trace_header.header_name,
        );
        // Reuse the migration converter and ordinary modular create transport.
        let created = CreateMigrationPaymentMethod::call(
            self.state,
            &client,
            CreateMigrationPaymentMethodRequest {
                card: card.clone(),
                customer_id: customer.clone(),
                modular_service_prefix: self
                    .state
                    .conf
                    .micro_services
                    .payment_methods_prefix
                    .0
                    .clone(),
            },
        )
        .await
        .map_err(|error| match error.kind {
            hyperswitch_interfaces::micro_service::MicroserviceClientErrorKind::Upstream {
                status: 401 | 403,
                ..
            } => CardError::AuthRejected,
            _ => CardError::ServiceUnavailable,
        })?;
        if !is_modular_id(&created.payment_method_id, "pm")
            || created.merchant_id != *self.merchant
            || created.customer_id.as_ref().map(|id| id.get_string_repr())
                != Some(customer.get_string_repr())
            || created.payment_method != Some(common_enums::PaymentMethod::Card)
            || created.storage_type != Some(common_enums::StorageType::Persistent)
        {
            return Err(CardError::InvalidResponse);
        }
        Ok(StoredCard {
            payment_method_id: created.payment_method_id,
        })
    }
}

#[cfg(test)]
pub(super) mod tests;
