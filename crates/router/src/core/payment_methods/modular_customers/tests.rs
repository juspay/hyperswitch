use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::*;

// Preserve JSON indexing's missing-field-as-null behavior using checked access.
fn json_at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
    value.pointer(pointer).cloned().unwrap_or_default()
}

fn merchant(id: &str) -> id_type::MerchantId {
    id_type::MerchantId::wrap(id.to_string()).expect("test merchant")
}

fn profile(id: &str) -> id_type::ProfileId {
    id_type::ProfileId::try_from(std::borrow::Cow::Owned(id.to_string())).expect("test profile")
}

fn record() -> PaymentMethodRecord {
    serde_json::from_value(serde_json::json!({
        "customer_id": "45630_user_one", "name": " Customer Name ",
        "email": "customer@example.com", "card_holder_name": "Different Cardholder",
        "merchant_connector_ids": "mca_one", "line_number": 1,
        "raw_card_number": "4242424242424242", "card_expiry_month": "12", "card_expiry_year": "2099"
    }))
    .expect("test record")
}

pub(super) fn input() -> PreparedInput {
    PreparedInput {
        line_number: Some(1),
        profile_id: profile("pro_one"),
        customer: customer_request(&record()).expect("customer details"),
        card: MigrationCardInput::try_from(&record()),
    }
}

fn response() -> CustomerResponse {
    serde_json::from_value(serde_json::json!({
        "id": "0a_cus_01926c58bc6e77c09e809964e72af8c8",
        "merchant_reference_id": "45630_user_one", "name": "Customer Name", "email": "customer@example.com"
    }))
    .expect("modular response")
}

struct FakeBackend {
    lists: tokio::sync::Mutex<VecDeque<Result<Vec<CustomerResponse>, CustomerError>>>,
    create_result: Result<CustomerResponse, CustomerError>,
    creates: AtomicUsize,
    locked: AtomicBool,
    releases: AtomicUsize,
    pending: tokio::sync::Mutex<Option<CustomerAttempt>>,
    journal_failure: bool,
    create_delay: Option<std::time::Duration>,
    journal_fail_at: usize,
    journal_writes: AtomicUsize,
}

impl FakeBackend {
    fn new(
        lists: Vec<Result<Vec<CustomerResponse>, CustomerError>>,
        create_result: Result<CustomerResponse, CustomerError>,
    ) -> Self {
        Self {
            lists: tokio::sync::Mutex::new(lists.into()),
            create_result,
            creates: AtomicUsize::new(0),
            locked: AtomicBool::new(false),
            releases: AtomicUsize::new(0),
            pending: tokio::sync::Mutex::new(None),
            journal_failure: false,
            create_delay: None,
            journal_fail_at: 0,
            journal_writes: AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl CustomerBackend for FakeBackend {
    async fn attempt(&self, _: &PreparedInput) -> Result<Option<CustomerAttempt>, CustomerError> {
        Ok(self.pending.lock().await.clone())
    }
    async fn remember(
        &self,
        _: &PreparedInput,
        attempt: CustomerAttempt,
    ) -> Result<(), CustomerError> {
        if self.journal_failure
            || self.journal_writes.fetch_add(1, Ordering::SeqCst) + 1 == self.journal_fail_at
        {
            return Err(CustomerError::JournalUnavailable);
        }
        *self.pending.lock().await = Some(attempt);
        Ok(())
    }
    async fn forget_rejected(&self, _: &PreparedInput) -> Result<(), CustomerError> {
        *self.pending.lock().await = None;
        Ok(())
    }
    async fn lock(&self, _: &id_type::CustomerId) -> Result<(), CustomerError> {
        self.locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| CustomerError::LockUnavailable)
    }
    async fn unlock(&self, _: &id_type::CustomerId) -> Result<(), CustomerError> {
        self.locked.store(false, Ordering::SeqCst);
        self.releases.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn list(&self, _: &PreparedInput) -> Result<Vec<CustomerResponse>, CustomerError> {
        tokio::task::yield_now().await;
        self.lists
            .lock()
            .await
            .pop_front()
            .expect("expected list call")
    }
    async fn create(&self, _: &PreparedInput) -> Result<CustomerResponse, CustomerError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        if let Some(delay) = self.create_delay {
            tokio::time::sleep(delay).await;
        }
        self.create_result.clone()
    }
}

#[tokio::test]
async fn uncertain_customer_post_is_not_repeated_after_lock_expiry() {
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![]), Ok(vec![]), Ok(vec![response()])],
        Err(CustomerError::ServiceUnavailable),
    );
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    backend.locked.store(false, Ordering::SeqCst);
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    assert!(resolve_customer(&backend, &input()).await.is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn customer_journal_failure_prevents_post() {
    let mut backend = FakeBackend::new(vec![Ok(vec![])], Ok(response()));
    backend.journal_failure = true;
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::JournalUnavailable)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    assert!(!backend.locked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn explicit_customer_auth_rejection_can_retry_after_key_replacement() {
    let mut backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![]), Ok(vec![response()])],
        Err(CustomerError::InvalidCredential),
    );
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::InvalidCredential)
    ));
    assert!(backend.pending.lock().await.is_none());
    assert!(!backend.locked.load(Ordering::SeqCst));
    backend.create_result = Ok(response());
    assert!(resolve_customer(&backend, &input()).await.is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn existing_customer_details_must_match_the_supplied_customer() {
    for field in ["name", "email"] {
        let mut customer = response();
        if field == "name" {
            customer.name = Some(Secret::new("Another Customer".to_string()));
        } else {
            customer.email = Some(
                serde_json::from_value(serde_json::json!("another@example.com")).expect("email"),
            );
        }
        let backend = FakeBackend::new(vec![Ok(vec![customer])], Ok(response()));
        assert!(matches!(
            resolve_customer(&backend, &input()).await,
            Err(CustomerError::PayloadConflict)
        ));
        assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn cancelled_customer_creation_keeps_pending_marker_after_lease_expiry() {
    let mut backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![])], Ok(response()));
    backend.create_delay = Some(std::time::Duration::from_secs(30));
    let backend = std::sync::Arc::new(backend);
    let cloned = backend.clone();
    let task = tokio::spawn(async move { resolve_customer(cloned.as_ref(), &input()).await });
    while backend.creates.load(Ordering::SeqCst) == 0 {
        tokio::task::yield_now().await;
    }
    task.abort();
    let _ = task.await;
    assert!(backend.pending.lock().await.is_some());
    backend.locked.store(false, Ordering::SeqCst);
    assert!(matches!(
        resolve_customer(backend.as_ref(), &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[test]
fn customer_attempt_keys_are_stable_scoped_and_contain_no_reference_or_credentials() {
    let reference = record().customer_id;
    let mid = merchant("merchant_one");
    let key = customer_journal_key("tenant_one", &mid, &reference, &[7; 32]);
    assert_eq!(
        key,
        customer_journal_key("tenant_one", &mid, &reference, &[7; 32])
    );
    assert_ne!(
        key,
        customer_journal_key("tenant_two", &mid, &reference, &[7; 32])
    );
    assert_ne!(
        key,
        customer_journal_key(
            "tenant_one",
            &merchant("merchant_two"),
            &reference,
            &[7; 32]
        )
    );
    assert!(!key.contains(reference.get_string_repr()));
    assert!(key.starts_with("MODULAR_CUSTOMER_MIGRATION_"));
}

#[tokio::test]
async fn changed_customer_identity_is_rejected_before_card_creation() {
    let mut changed = response();
    changed.id = id_type::GlobalCustomerId::new_unchecked(
        "0a_cus_ffffffffffffffffffffffffffffffff".to_string(),
    );
    let backend = FakeBackend::new(
        vec![Ok(vec![response()]), Ok(vec![changed])],
        Ok(response()),
    );
    assert!(resolve_customer(&backend, &input()).await.is_ok());
    let cards = cards::tests::FakeCards::new(vec![], Ok(cards::tests::stored()));
    let rows = resolve_batch(
        &backend,
        &cards,
        vec![input()],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let rows = serde_json::to_value(rows).expect("results");
    assert_eq!(
        json_at(&rows, "/0/migration_error"),
        "migration_customer_reference_ambiguous"
    );
    assert_eq!(cards.creates.load(Ordering::SeqCst), 0);
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn customer_journal_failure_after_post_is_uncertain_and_reconciles_without_reposting() {
    let mut backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![response()]), Ok(vec![response()])],
        Ok(response()),
    );
    backend.journal_fail_at = 2;
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    backend.locked.store(false, Ordering::SeqCst);
    assert!(resolve_customer(&backend, &input()).await.is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert_eq!(
        backend
            .pending
            .lock()
            .await
            .as_ref()
            .expect("journal")
            .customer_id
            .as_ref(),
        Some(&response().id)
    );
}

#[tokio::test]
async fn created_customer_id_survives_delayed_confirmation_and_cannot_be_rebound() {
    let mut changed = response();
    changed.id = id_type::GlobalCustomerId::new_unchecked(
        "0a_cus_ffffffffffffffffffffffffffffffff".to_string(),
    );
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![]), Ok(vec![changed])],
        Ok(response()),
    );
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    assert_eq!(
        backend
            .pending
            .lock()
            .await
            .as_ref()
            .expect("journal")
            .customer_id
            .as_ref(),
        Some(&response().id)
    );
    backend.locked.store(false, Ordering::SeqCst);
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::AmbiguousReference)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn synthetic_wire_contract_exports_real_rust_results_for_the_tool() {
    use actix_multipart::form::{bytes, text};
    use payment_methods::core::migration::{CustomerMigrationOptions, PaymentMethodsMigrateForm};
    let fixture = std::env::var("DERIVCO_CONTRACT_INPUT")
        .map(|path| std::fs::read_to_string(path).expect("tool-generated synthetic CSV"))
        .unwrap_or_else(|_| include_str!("fixtures/contract_row.csv").to_string());
    let mut form = PaymentMethodsMigrateForm {
        file: bytes::Bytes {
            data: actix_web::web::Bytes::copy_from_slice(fixture.as_bytes()),
            content_type: None,
            file_name: Some("row.csv".to_string()),
        },
        merchant_id: text::Text(merchant("merchant_one")),
        merchant_connector_id: None,
        merchant_connector_ids: Some(text::Text("mca_one".to_string())),
        use_modular_customer_creation: Some(text::Text(true)),
        customer_api_key: Some(text::Text(StrongSecret::new(
            "synthetic_temporary_secret".to_string(),
        ))),
    };
    assert!(!format!("{form:?}").contains("synthetic_temporary_secret"));
    assert!(matches!(
        form.take_customer_migration_options().expect("options"),
        CustomerMigrationOptions::Modular { .. }
    ));
    form.validate_modular_csv_headers().expect("CSV headers");
    let (mid, records, mcas) = form
        .validate_and_get_payment_method_records()
        .expect("real form/CSV validation");
    assert_eq!(
        mcas.expect("MCAs").first().expect("MCA").get_string_repr(),
        "mca_one"
    );
    let record = records.first().expect("source record");
    let request = customer_request(record).expect("customer");
    let input = PreparedInput {
        line_number: record.line_number,
        profile_id: profile("pro_one"),
        customer: request.clone(),
        card: MigrationCardInput::try_from(record),
    };
    let customer: CustomerResponse = serde_json::from_value(serde_json::json!({
        "id": "0a_cus_01926c58bc6e77c09e809964e72af8c8", "merchant_reference_id": request.merchant_reference_id,
        "name": request.name, "email": request.email
    })).expect("customer response");
    let payload = input
        .card
        .as_ref()
        .expect("card")
        .payload(customer.id.clone());
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![customer.clone()])],
        Ok(customer.clone()),
    );
    let cards = cards::tests::FakeCards::new(
        vec![Ok(None), Ok(Some(cards::tests::stored()))],
        Ok(cards::tests::stored()),
    );
    let success = resolve_batch(
        &backend,
        &cards,
        vec![input.clone()],
        5,
        "contract_tenant",
        &mid,
        &[1; 32],
    )
    .await;
    let backend = FakeBackend::new(
        vec![Ok(vec![customer])],
        Err(CustomerError::ServiceUnavailable),
    );
    let cards = cards::tests::FakeCards::new(
        vec![Err(cards::CardError::ServiceUnavailable)],
        Err(cards::CardError::ServiceUnavailable),
    );
    let partial = resolve_batch(
        &backend,
        &cards,
        vec![input.clone()],
        5,
        "contract_tenant",
        &mid,
        &[1; 32],
    )
    .await;
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![])],
        Err(CustomerError::ServiceUnavailable),
    );
    let uncertain = resolve_batch(
        &backend,
        &cards,
        vec![input],
        5,
        "contract_tenant",
        &mid,
        &[1; 32],
    )
    .await;
    let success = serde_json::to_value(success).expect("success");
    assert_eq!(
        json_at(&success, "/0/merchant_reference_id"),
        "45630_qa_contract"
    );
    assert_eq!(json_at(&success, "/0/line_number"), 1);
    assert_eq!(json_at(&success, "/0/migration_status"), "Success");
    assert!(json_at(
        &serde_json::to_value(&payload).expect("payload"),
        "/payment_method_data/card/card_cvc"
    )
    .is_null());
    if let Ok(directory) = std::env::var("DERIVCO_CONTRACT_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).expect("contract directory");
        std::fs::write(directory.join("input.csv"), &fixture).expect("input fixture");
        for (name, value) in [
            (
                "capabilities.json",
                serde_json::to_value(capabilities()).expect("capabilities"),
            ),
            ("success.json", success),
            (
                "card_failed.json",
                serde_json::to_value(partial).expect("partial"),
            ),
            (
                "customer_uncertain.json",
                serde_json::to_value(uncertain).expect("uncertain"),
            ),
            (
                "card_request.json",
                serde_json::to_value(payload).expect("card"),
            ),
            (
                "customer_request.json",
                serde_json::to_value(request).expect("customer"),
            ),
        ] {
            std::fs::write(
                directory.join(name),
                serde_json::to_vec_pretty(&value).expect("JSON"),
            )
            .expect("contract fixture");
        }
    }
}

#[test]
fn merchant_key_owner_and_expiry_are_enforced() {
    let now = date_time::now();
    let mid = merchant("merchant_one");
    assert_eq!(
        validate_key(&mid, Some(now + time::Duration::hours(1)), &mid, now),
        Ok(())
    );
    assert_eq!(
        validate_key(
            &merchant("merchant_two"),
            Some(now + time::Duration::hours(1)),
            &mid,
            now
        ),
        Err(CustomerError::InvalidCredential)
    );
    assert_eq!(
        validate_key(&mid, Some(now), &mid, now),
        Err(CustomerError::ExpiredCredential)
    );
    assert_eq!(
        validate_key(&mid, Some(now - time::Duration::seconds(1)), &mid, now),
        Err(CustomerError::ExpiredCredential)
    );
    assert_eq!(
        validate_key(&mid, None, &mid, now),
        Err(CustomerError::InvalidCredential)
    );
}

#[test]
fn customer_name_is_separate_from_cardholder_and_required() {
    let mut row = record();
    assert_eq!(
        customer_request(&row).expect("valid").name.peek(),
        "Customer Name"
    );
    row.name = None;
    assert!(matches!(
        customer_request(&row),
        Err(CustomerError::InvalidInput)
    ));
    row.name = Some(Secret::new("  ".to_string()));
    assert!(matches!(
        customer_request(&row),
        Err(CustomerError::InvalidInput)
    ));
    row.name = Some(Secret::new("Customer Name".to_string()));
    row.email = None;
    assert!(matches!(
        customer_request(&row),
        Err(CustomerError::InvalidInput)
    ));
}

#[test]
fn mca_bindings_must_belong_to_one_merchant_and_profile() {
    let mid = merchant("merchant_one");
    let bindings = [
        (mid.clone(), profile("pro_one")),
        (mid.clone(), profile("pro_one")),
    ];
    assert_eq!(single_profile(&bindings, &mid), Ok(profile("pro_one")));
    assert_eq!(
        single_profile(
            &[
                (mid.clone(), profile("pro_one")),
                (mid.clone(), profile("pro_two"))
            ],
            &mid
        ),
        Err(CustomerError::InvalidProfile)
    );
    assert_eq!(
        single_profile(&[(merchant("merchant_two"), profile("pro_one"))], &mid),
        Err(CustomerError::InvalidProfile)
    );
    assert_eq!(
        single_profile(&[], &mid),
        Err(CustomerError::InvalidProfile)
    );
}

#[test]
fn row_mca_and_fallback_rules_are_explicit() {
    let mut row = record();
    let fallback =
        vec![id_type::MerchantConnectorAccountId::wrap("mca_fallback".into()).expect("MCA")];
    assert_eq!(
        connector_ids(&row, Some(&fallback))
            .expect("row MCA")
            .first()
            .expect("row MCA entry")
            .get_string_repr(),
        "mca_one"
    );
    row.merchant_connector_ids = None;
    assert_eq!(
        connector_ids(&row, Some(&fallback)).expect("fallback"),
        fallback
    );
    assert_eq!(
        connector_ids(&row, None),
        Err(CustomerError::InvalidProfile)
    );
    row.merchant_connector_ids = Some(" , ".into());
    assert_eq!(
        connector_ids(&row, Some(&fallback)),
        Err(CustomerError::InvalidProfile)
    );
    row.merchant_connector_ids = Some("mca_one".into());
    row.merchant_connector_id = Some(fallback.first().expect("fallback MCA").clone());
    assert_eq!(
        connector_ids(&row, None),
        Err(CustomerError::InvalidProfile)
    );
}

#[test]
fn requests_use_modular_auth_profile_tenant_and_encoded_reference() {
    let input = input();
    let key = StrongSecret::new("synthetic_customer_secret".to_string());
    let request = customer_http_request(
        &url::Url::parse("http://modular.invalid").expect("URL"),
        "v1",
        &input.profile_id,
        &key,
        "test_tenant",
        &input.customer,
        false,
    )
    .expect("request");
    assert_eq!(request.method, Method::Get);
    assert!(request
        .url
        .contains("/v1/customers/list?customer_id=45630_user_one&limit=2&offset=0"));
    assert!(request.headers.iter().any(|(name, value)| name == "Authorization" && matches!(value, Maskable::Masked(secret) if secret.peek() == "api-key=synthetic_customer_secret")));
    assert!(request
        .headers
        .iter()
        .any(|(name, value)| name == "x-profile-id"
            && matches!(value, Maskable::Normal(value) if value == "pro_one")));
    assert!(request
        .headers
        .iter()
        .any(|(name, value)| name == "x-tenant-id"
            && matches!(value, Maskable::Normal(value) if value == "test_tenant")));
    assert!(!request.headers.iter().any(|(name, _)| name == "api-key"));
    assert!(!format!("{:?}", request.headers).contains("synthetic_customer_secret"));
    let create = customer_http_request(
        &url::Url::parse("http://modular.invalid").expect("URL"),
        "v1",
        &input.profile_id,
        &key,
        "test_tenant",
        &input.customer,
        true,
    )
    .expect("POST");
    assert_eq!(create.method, Method::Post);
    assert!(create.disable_connection_closed_retry);
    assert!(!request.disable_connection_closed_retry);
    assert_eq!(create.url, "http://modular.invalid/v1/customers");
    assert!(create.body.is_some());
}

#[tokio::test]
async fn transport_retry_opt_out_preserves_legacy_policy_and_sends_mutations_once() {
    use std::{
        io::{Read, Write},
        net::{Shutdown, TcpListener},
        time::{Duration, Instant},
    };
    for disable in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        listener.set_nonblocking(true).expect("nonblocking");
        let address = listener.local_addr().expect("address");
        let expected = if disable { 1 } else { 2 };
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut accepted = 0;
            while accepted < expected {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "expected transport request was not received"
                        );
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("accept failed: {error}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("timeout");
                let mut bytes = Vec::new();
                loop {
                    let mut chunk = [0; 4096];
                    let size = stream.read(&mut chunk).expect("request");
                    assert!(size > 0);
                    bytes.extend_from_slice(chunk.get(..size).expect("read length within buffer"));
                    if let Some(end) = bytes.windows(4).position(|slice| slice == b"\r\n\r\n") {
                        if bytes.len() >= end + 6 {
                            break;
                        }
                    }
                }
                accepted += 1;
                if accepted == 1 {
                    // The synthetic mutation has committed, but no response is returned.
                    stream.shutdown(Shutdown::Both).expect("close");
                } else {
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        )
                        .expect("response");
                }
            }
            accepted
        });
        let mut request = Request::new(
            Method::Post,
            &format!("http://{address}/synthetic-mutation"),
        );
        request.set_body(RequestContent::RawBytes(b"qa".to_vec()));
        request.disable_connection_closed_retry = disable;
        let response = external_services::http_client::send_request(
            &hyperswitch_interfaces::types::Proxy::default(),
            request,
            Some(3),
        )
        .await;
        if disable {
            assert_eq!(
                response.expect_err("do not resend").current_context(),
                &hyperswitch_interfaces::errors::HttpClientError::ConnectionClosedIncompleteMessage
            );
        } else {
            assert_eq!(
                response.expect("legacy retry").status(),
                reqwest::StatusCode::OK
            );
        }
        assert_eq!(server.join().expect("server"), expected);
    }
}

#[test]
fn capability_advertises_customer_and_permanent_card_migration() {
    let value = serde_json::to_value(capabilities()).expect("capabilities");
    assert_eq!(json_at(&value, "/modular_customer_creation"), true);
    assert_eq!(json_at(&value, "/modular_card_storage"), true);
}

#[tokio::test]
async fn existing_customer_is_reused_without_post() {
    let backend = FakeBackend::new(
        vec![Ok(vec![response()])],
        Err(CustomerError::ServiceUnavailable),
    );
    let (customer, created) = resolve_customer(&backend, &input())
        .await
        .expect("existing");
    assert_eq!(customer.id, response().id);
    assert!(!created);
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn new_customer_is_confirmed_after_write() {
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![response()])], Ok(response()));
    let (customer, created) = resolve_customer(&backend, &input()).await.expect("new");
    assert_eq!(customer.id, response().id);
    assert!(created);
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn lost_create_response_is_reconciled_without_a_second_post() {
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![response()])],
        Err(CustomerError::ServiceUnavailable),
    );
    assert!(resolve_customer(&backend, &input()).await.is_ok());
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invisible_or_uncertain_write_retains_lease_and_blocks_replay() {
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![])], Ok(response()));
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    assert_eq!(backend.releases.load(Ordering::SeqCst), 0);
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::LockUnavailable)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn ambiguous_reference_is_rejected_without_creation() {
    let backend = FakeBackend::new(vec![Ok(vec![response(), response()])], Ok(response()));
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::AmbiguousReference)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn concurrent_resolutions_do_not_create_twice() {
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![response()])], Ok(response()));
    let input = input();
    let (first, second) = tokio::join!(
        resolve_customer(&backend, &input),
        resolve_customer(&backend, &input)
    );
    assert!(first.is_ok());
    assert!(matches!(second, Err(CustomerError::LockUnavailable)));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn wrong_reference_response_is_not_accepted() {
    let mut wrong = response();
    wrong.merchant_reference_id =
        Some(id_type::CustomerId::wrap("another_customer".into()).expect("ID"));
    let backend = FakeBackend::new(vec![Ok(vec![wrong])], Ok(response()));
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::InvalidResponse)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn malformed_global_id_cannot_deserialize_as_customer_response() {
    for id in [
        "45630_user_one",
        "cus_legacy_customer",
        "0a_pm_01926c58bc6e77c09e809964e72af8c8",
        "longcell_cus_01926c58bc6e77c09e809964e72af8c8",
        "0a_cus_not_a_uuid",
        "0a_cus_01926c58bc6e77c09e809964e72af8c8_extra",
    ] {
        assert!(
            serde_json::from_value::<CustomerResponse>(serde_json::json!({
                "id": id, "merchant_reference_id": "45630_user_one"
            }))
            .is_err(),
            "accepted invalid ID {id}"
        );
    }
    let generated =
        id_type::GlobalCustomerId::generate(&id_type::CellId::from_string("0a").expect("cell"));
    assert!(
        serde_json::from_value::<CustomerResponse>(serde_json::json!({
            "id": generated, "merchant_reference_id": "45630_user_one"
        }))
        .is_ok()
    );
}

#[test]
fn batch_details_reject_conflicts_and_cross_merchant_rows() {
    let mid = merchant("merchant_one");
    let mut second = record();
    assert!(validate_customer_details(&[record(), second.clone()], &mid).is_ok());
    second.name = Some(Secret::new("Conflicting Name".into()));
    assert!(matches!(
        validate_customer_details(&[record(), second], &mid),
        Err(CustomerError::InvalidInput)
    ));
    let mut second = record();
    second.email =
        Some(serde_json::from_value(serde_json::json!("different@example.com")).expect("email"));
    assert!(matches!(
        validate_customer_details(&[record(), second], &mid),
        Err(CustomerError::InvalidInput)
    ));
    let mut second = record();
    second.merchant_id = Some(merchant("merchant_two"));
    assert!(matches!(
        validate_customer_details(&[second], &mid),
        Err(CustomerError::InvalidInput)
    ));
}

#[tokio::test]
async fn batch_deduplicates_customers_and_cards_preserving_profiles_and_source_lines() {
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![response()])], Ok(response()));
    let card_backend = cards::tests::FakeCards::new(
        vec![Ok(None), Ok(Some(cards::tests::stored()))],
        Ok(cards::tests::stored()),
    );
    let mut second = input();
    second.line_number = Some(2);
    second.profile_id = profile("pro_two");
    let results = resolve_batch(
        &backend,
        &card_backend,
        vec![input(), second],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    let rows = serde_json::to_value(results).expect("results");
    assert_eq!(
        json_at(&rows, "/0/customer_id"),
        json_at(&rows, "/1/customer_id")
    );
    assert_eq!(json_at(&rows, "/0/customer_created"), true);
    assert_eq!(json_at(&rows, "/1/customer_created"), false);
    assert_eq!(json_at(&rows, "/1/profile_id"), "pro_two");
    assert_eq!(json_at(&rows, "/0/merchant_reference_id"), "45630_user_one");
    assert_eq!(json_at(&rows, "/0/customer_migration_status"), "Success");
    assert_eq!(json_at(&rows, "/0/migration_status"), "Success");
    assert_eq!(json_at(&rows, "/0/card_migrated"), true);
    assert_eq!(
        json_at(&rows, "/0/payment_method_id"),
        json_at(&rows, "/1/payment_method_id")
    );
    assert_eq!(json_at(&rows, "/1/line_number"), 2);
    assert_eq!(json_at(&rows, "/0/customer_creation_flow"), "modular");
    assert_eq!(json_at(&rows, "/0/errors"), serde_json::json!([]));
    assert_eq!(card_backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn customer_created_card_failed_retry_reuses_generated_customer() {
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![response()]), Ok(vec![response()])],
        Ok(response()),
    );
    let card_backend = cards::tests::FakeCards::new(
        vec![
            Err(cards::CardError::ServiceUnavailable),
            Ok(None),
            Ok(Some(cards::tests::stored())),
        ],
        Ok(cards::tests::stored()),
    );
    let first = resolve_batch(
        &backend,
        &card_backend,
        vec![input()],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let first = serde_json::to_value(first).expect("response");
    assert_eq!(json_at(&first, "/0/customer_created"), true);
    assert_eq!(json_at(&first, "/0/card_migrated"), false);
    assert_eq!(
        json_at(&first, "/0/customer_id"),
        response().id.get_string_repr()
    );
    assert_eq!(json_at(&first, "/0/errors/0/stage"), "card");
    assert_eq!(json_at(&first, "/0/errors/0/retryable"), true);
    let retry = resolve_batch(
        &backend,
        &card_backend,
        vec![input()],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let retry = serde_json::to_value(retry).expect("response");
    assert_eq!(json_at(&retry, "/0/card_migrated"), true);
    assert_eq!(json_at(&retry, "/0/customer_created"), false);
    assert_eq!(
        json_at(&retry, "/0/customer_id"),
        json_at(&first, "/0/customer_id")
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert_eq!(card_backend.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn customer_created_card_post_rejected_retry_does_not_recreate_customer() {
    let backend = FakeBackend::new(
        vec![Ok(vec![]), Ok(vec![response()]), Ok(vec![response()])],
        Ok(response()),
    );
    let card_backend = cards::tests::FakeCards::new(
        vec![Ok(None), Ok(None), Ok(Some(cards::tests::stored()))],
        Err(cards::CardError::AuthRejected),
    );
    let first = resolve_batch(
        &backend,
        &card_backend,
        vec![input()],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let first = serde_json::to_value(first).expect("response");
    assert_eq!(json_at(&first, "/0/customer_created"), true);
    assert_eq!(
        json_at(&first, "/0/errors/0/code"),
        "migration_card_auth_rejected"
    );
    assert_eq!(json_at(&first, "/0/errors/0/outcome_uncertain"), false);
    card_backend
        .set_create_result(Ok(cards::tests::stored()))
        .await;
    let retry = resolve_batch(
        &backend,
        &card_backend,
        vec![input()],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let retry = serde_json::to_value(retry).expect("response");
    assert_eq!(json_at(&retry, "/0/migration_status"), "Success");
    assert_eq!(
        json_at(&retry, "/0/customer_id"),
        json_at(&first, "/0/customer_id")
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
    assert_eq!(card_backend.creates.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn unsupported_card_fields_return_structured_errors_without_customer_creation() {
    let backend = FakeBackend::new(vec![], Ok(response()));
    let card_backend = cards::tests::FakeCards::new(vec![], Ok(cards::tests::stored()));
    let mut row = input();
    row.card = Err(MigrationCardInputError {
        code: "migration_card_input_unsupported",
        fields: vec!["original_transaction_id"],
    });
    let result = resolve_batch(
        &backend,
        &card_backend,
        vec![row],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let row = serde_json::to_value(result).expect("response");
    assert_eq!(json_at(&row, "/0/customer_id"), serde_json::Value::Null);
    assert_eq!(
        json_at(&row, "/0/errors/0/fields"),
        serde_json::json!(["original_transaction_id"])
    );
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    assert_eq!(card_backend.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn subtype_preflight_reuses_bin_inference_without_guessing() {
    assert!(!supported_card_subtype(None, None));
    assert!(!supported_card_subtype(None, Some("unknown".into())));
    assert!(supported_card_subtype(None, Some(" CREDIT ".into())));
    assert!(supported_card_subtype(None, Some("debit".into())));
    assert!(supported_card_subtype(
        Some(common_enums::PaymentMethodType::Credit),
        None
    ));
}

#[tokio::test]
async fn unavailable_subtype_is_retryable_before_any_creation() {
    let backend = FakeBackend::new(vec![], Ok(response()));
    let card_backend = cards::tests::FakeCards::new(vec![], Ok(cards::tests::stored()));
    let mut row = input();
    row.card = Err(MigrationCardInputError {
        code: "migration_card_subtype_unavailable",
        fields: vec!["payment_method_type"],
    });
    let result = resolve_batch(
        &backend,
        &card_backend,
        vec![row],
        5,
        "tenant",
        &merchant("merchant_one"),
        &[1; 32],
    )
    .await;
    let result = serde_json::to_value(result).expect("response");
    assert_eq!(json_at(&result, "/0/customer_id"), serde_json::Value::Null);
    assert_eq!(json_at(&result, "/0/errors/0/retryable"), true);
    assert_eq!(json_at(&result, "/0/errors/0/outcome_uncertain"), false);
    assert_eq!(backend.creates.load(Ordering::SeqCst), 0);
    assert_eq!(card_backend.creates.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mismatched_create_reply_retains_lease() {
    let mut wrong = response();
    wrong.merchant_reference_id =
        Some(id_type::CustomerId::wrap("another_customer".into()).expect("ID"));
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![response()])], Ok(wrong));
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::UncertainCreation)
    ));
    assert_eq!(backend.releases.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_post_write_confirmation_retains_lease() {
    let mut wrong = response();
    wrong.merchant_reference_id =
        Some(id_type::CustomerId::wrap("another_customer".into()).expect("ID"));
    let backend = FakeBackend::new(vec![Ok(vec![]), Ok(vec![wrong])], Ok(response()));
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::InvalidResponse)
    ));
    assert_eq!(backend.releases.load(Ordering::SeqCst), 0);
    assert!(matches!(
        resolve_customer(&backend, &input()).await,
        Err(CustomerError::LockUnavailable)
    ));
    assert_eq!(backend.creates.load(Ordering::SeqCst), 1);
}
