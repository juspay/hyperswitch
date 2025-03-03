use api_models::{
    enums::Connector::{DummyConnector4, DummyConnector7},
    user::sample_data::SampleDataRequest,
};
use common_utils::{
    id_type,
    types::{ConnectorTransactionId, MinorUnit},
};
#[cfg(feature = "v1")]
use diesel_models::user::sample_data::PaymentAttemptBatchNew;
use diesel_models::{enums as storage_enums, DisputeNew, RefundNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentIntent;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use time::OffsetDateTime;

use crate::{
    consts,
    core::errors::sample_data::{SampleDataError, SampleDataResult},
    SessionState,
};

#[cfg(feature = "v1")]
#[allow(clippy::type_complexity)]
pub async fn generate_sample_data(
    state: &SessionState,
    req: SampleDataRequest,
    merchant_id: &id_type::MerchantId,
    org_id: &id_type::OrganizationId,
) -> SampleDataResult<
    Vec<(
        PaymentIntent,
        PaymentAttemptBatchNew,
        Option<RefundNew>,
        Option<DisputeNew>,
    )>,
> {
    let sample_data_size: usize = req.record.unwrap_or(100);
    let key_manager_state = &state.into();
    if !(10..=100).contains(&sample_data_size) {
        return Err(SampleDataError::InvalidRange.into());
    }

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)?;

    let merchant_from_db = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .change_context::<SampleDataError>(SampleDataError::DataDoesNotExist)?;

    #[cfg(feature = "v1")]
    let (profile_id_result, business_country_default, business_label_default) = {
        let merchant_parsed_details: Vec<api_models::admin::PrimaryBusinessDetails> =
            serde_json::from_value(merchant_from_db.primary_business_details.clone())
                .change_context(SampleDataError::InternalServerError)
                .attach_printable("Error while parsing primary business details")?;

        let business_country_default = merchant_parsed_details.first().map(|x| x.country);

        let business_label_default = merchant_parsed_details.first().map(|x| x.business.clone());

        let profile_id = crate::core::utils::get_profile_id_from_business_details(
            key_manager_state,
            &key_store,
            business_country_default,
            business_label_default.as_ref(),
            &merchant_from_db,
            req.profile_id.as_ref(),
            &*state.store,
            false,
        )
        .await;
        (profile_id, business_country_default, business_label_default)
    };

    #[cfg(feature = "v2")]
    let (profile_id_result, business_country_default, business_label_default) = {
        let profile_id = req
            .profile_id.clone()
            .ok_or(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::MissingRequiredField {
                field_name: "profile_id",
            });

        (profile_id, None, None)
    };

    let profile_id = match profile_id_result {
        Ok(id) => id.clone(),
        Err(error) => {
            router_env::logger::error!(
                "Profile ID not found in business details. Attempting to fetch from the database {error:?}"
            );

            state
                .store
                .list_profile_by_merchant_id(key_manager_state, &key_store, merchant_id)
                .await
                .change_context(SampleDataError::InternalServerError)
                .attach_printable("Failed to get business profile")?
                .first()
                .ok_or(SampleDataError::InternalServerError)?
                .get_id()
                .to_owned()
        }
    };

    // 10 percent payments should be failed
    #[allow(clippy::as_conversions)]
    let failure_attempts = usize::try_from((sample_data_size as f32 / 10.0).round() as i64)
        .change_context(SampleDataError::InvalidParameters)?;

    let failure_after_attempts = sample_data_size / failure_attempts;

    // 20 percent refunds for payments
    #[allow(clippy::as_conversions)]
    let number_of_refunds = usize::try_from((sample_data_size as f32 / 5.0).round() as i64)
        .change_context(SampleDataError::InvalidParameters)?;

    let mut refunds_count = 0;

    // 2 disputes if generated data size is between 50 and 100, 1 dispute if it is less than 50.
    let number_of_disputes: usize = if sample_data_size >= 50 { 2 } else { 1 };

    let mut disputes_count = 0;

    let mut random_array: Vec<usize> = (1..=sample_data_size).collect();

    // Shuffle the array
    let mut rng = thread_rng();
    random_array.shuffle(&mut rng);

    let mut res: Vec<(
        PaymentIntent,
        PaymentAttemptBatchNew,
        Option<RefundNew>,
        Option<DisputeNew>,
    )> = Vec::new();
    let start_time = req
        .start_time
        .unwrap_or(common_utils::date_time::now() - time::Duration::days(7))
        .assume_utc()
        .unix_timestamp();
    let end_time = req
        .end_time
        .unwrap_or_else(common_utils::date_time::now)
        .assume_utc()
        .unix_timestamp();

    let current_time = common_utils::date_time::now().assume_utc().unix_timestamp();

    let min_amount = req.min_amount.unwrap_or(100);
    let max_amount = req.max_amount.unwrap_or(min_amount + 100);

    if min_amount > max_amount
        || start_time > end_time
        || start_time > current_time
        || end_time > current_time
    {
        return Err(SampleDataError::InvalidParameters.into());
    };

    let currency_vec = req.currency.unwrap_or(vec![common_enums::Currency::USD]);
    let currency_vec_len = currency_vec.len();

    let connector_vec = req
        .connector
        .unwrap_or(vec![DummyConnector4, DummyConnector7]);
    let connector_vec_len = connector_vec.len();

    let auth_type = req.auth_type.unwrap_or(vec![
        common_enums::AuthenticationType::ThreeDs,
        common_enums::AuthenticationType::NoThreeDs,
    ]);
    let auth_type_len = auth_type.len();

    if currency_vec_len == 0 || connector_vec_len == 0 || auth_type_len == 0 {
        return Err(SampleDataError::InvalidParameters.into());
    }

    // This has to be an internal server error because, this function failing means that the intended functionality is not working as expected
    let dashboard_customer_id =
        id_type::CustomerId::try_from(std::borrow::Cow::from("hs-dashboard-user"))
            .change_context(SampleDataError::InternalServerError)?;

    for num in 1..=sample_data_size {
        let payment_id = id_type::PaymentId::generate_test_payment_id_for_sample_data();
        let attempt_id = payment_id.get_attempt_id(1);
        let client_secret = payment_id.generate_client_secret();
        let amount = thread_rng().gen_range(min_amount..=max_amount);

        let created_at @ modified_at @ last_synced =
            OffsetDateTime::from_unix_timestamp(thread_rng().gen_range(start_time..=end_time))
                .map(common_utils::date_time::convert_to_pdt)
                .unwrap_or(
                    req.start_time.unwrap_or_else(|| {
                        common_utils::date_time::now() - time::Duration::days(7)
                    }),
                );
        let session_expiry =
            created_at.saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        // After some set of payments sample data will have a failed attempt
        let is_failed_payment =
            (random_array.get(num - 1).unwrap_or(&0) % failure_after_attempts) == 0;

        let payment_intent = PaymentIntent {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            status: match is_failed_payment {
                true => common_enums::IntentStatus::Failed,
                _ => common_enums::IntentStatus::Succeeded,
            },
            amount: MinorUnit::new(amount * 100),
            currency: Some(
                *currency_vec
                    .get((num - 1) % currency_vec_len)
                    .unwrap_or(&common_enums::Currency::USD),
            ),
            description: Some("This is a sample payment".to_string()),
            created_at,
            modified_at,
            last_synced: Some(last_synced),
            client_secret: Some(client_secret),
            business_country: business_country_default,
            business_label: business_label_default.clone(),
            active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
                attempt_id.clone(),
            ),
            attempt_count: 1,
            customer_id: Some(dashboard_customer_id.clone()),
            amount_captured: Some(MinorUnit::new(amount * 100)),
            profile_id: Some(profile_id.clone()),
            return_url: Default::default(),
            metadata: Default::default(),
            connector_id: Default::default(),
            shipping_address_id: Default::default(),
            billing_address_id: Default::default(),
            statement_descriptor_name: Default::default(),
            statement_descriptor_suffix: Default::default(),
            setup_future_usage: Default::default(),
            off_session: Default::default(),
            order_details: Default::default(),
            allowed_payment_method_types: Default::default(),
            connector_metadata: Default::default(),
            feature_metadata: Default::default(),
            merchant_decision: Default::default(),
            payment_link_id: Default::default(),
            payment_confirm_source: Default::default(),
            updated_by: merchant_from_db.storage_scheme.to_string(),
            surcharge_applicable: Default::default(),
            request_incremental_authorization: Default::default(),
            incremental_authorization_allowed: Default::default(),
            authorization_count: Default::default(),
            fingerprint_id: None,
            session_expiry: Some(session_expiry),
            request_external_three_ds_authentication: None,
            split_payments: None,
            frm_metadata: Default::default(),
            customer_details: None,
            billing_details: None,
            merchant_order_reference_id: Default::default(),
            shipping_details: None,
            is_payment_processor_token_flow: None,
            organization_id: org_id.clone(),
            shipping_cost: None,
            tax_details: None,
            skip_external_tax_calculation: None,
            request_extended_authorization: None,
            psd2_sca_exemption_type: None,
            platform_merchant_id: None,
        };
        let (connector_transaction_id, processor_transaction_data) =
            ConnectorTransactionId::form_id_and_data(attempt_id.clone());
        let payment_attempt = PaymentAttemptBatchNew {
            attempt_id: attempt_id.clone(),
            payment_id: payment_id.clone(),
            connector_transaction_id: Some(connector_transaction_id),
            merchant_id: merchant_id.clone(),
            status: match is_failed_payment {
                true => common_enums::AttemptStatus::Failure,
                _ => common_enums::AttemptStatus::Charged,
            },
            amount: MinorUnit::new(amount * 100),
            currency: payment_intent.currency,
            connector: Some(
                (*connector_vec
                    .get((num - 1) % connector_vec_len)
                    .unwrap_or(&DummyConnector4))
                .to_string(),
            ),
            payment_method: Some(common_enums::PaymentMethod::Card),
            payment_method_type: Some(get_payment_method_type(thread_rng().gen_range(1..=2))),
            authentication_type: Some(
                *auth_type
                    .get((num - 1) % auth_type_len)
                    .unwrap_or(&common_enums::AuthenticationType::NoThreeDs),
            ),
            error_message: match is_failed_payment {
                true => Some("This is a test payment which has a failed status".to_string()),
                _ => None,
            },
            error_code: match is_failed_payment {
                true => Some("HS001".to_string()),
                _ => None,
            },
            confirm: true,
            created_at,
            modified_at,
            last_synced: Some(last_synced),
            amount_to_capture: Some(MinorUnit::new(amount * 100)),
            connector_response_reference_id: Some(attempt_id.clone()),
            updated_by: merchant_from_db.storage_scheme.to_string(),
            save_to_locker: None,
            offer_amount: None,
            surcharge_amount: None,
            tax_amount: None,
            payment_method_id: None,
            capture_method: None,
            capture_on: None,
            cancellation_reason: None,
            mandate_id: None,
            browser_info: None,
            payment_token: None,
            connector_metadata: None,
            payment_experience: None,
            payment_method_data: None,
            business_sub_label: None,
            straight_through_algorithm: None,
            preprocessing_step_id: None,
            mandate_details: None,
            error_reason: None,
            multiple_capture_count: None,
            amount_capturable: MinorUnit::new(i64::default()),
            merchant_connector_id: None,
            authentication_data: None,
            encoded_data: None,
            unified_code: None,
            unified_message: None,
            net_amount: None,
            external_three_ds_authentication_attempted: None,
            authentication_connector: None,
            authentication_id: None,
            mandate_data: None,
            payment_method_billing_address_id: None,
            fingerprint_id: None,
            charge_id: None,
            client_source: None,
            client_version: None,
            customer_acceptance: None,
            profile_id: profile_id.clone(),
            organization_id: org_id.clone(),
            shipping_cost: None,
            order_tax_amount: None,
            processor_transaction_data,
            connector_mandate_detail: None,
            request_extended_authorization: None,
            extended_authorization_applied: None,
            capture_before: None,
            card_discovery: None,
        };

        let refund = if refunds_count < number_of_refunds && !is_failed_payment {
            refunds_count += 1;
            let (connector_transaction_id, processor_transaction_data) =
                ConnectorTransactionId::form_id_and_data(attempt_id.clone());
            Some(RefundNew {
                refund_id: common_utils::generate_id_with_default_len("test"),
                internal_reference_id: common_utils::generate_id_with_default_len("test"),
                external_reference_id: None,
                payment_id: payment_id.clone(),
                attempt_id: attempt_id.clone(),
                merchant_id: merchant_id.clone(),
                connector_transaction_id,
                connector_refund_id: None,
                description: Some("This is a sample refund".to_string()),
                created_at,
                modified_at,
                refund_reason: Some("Sample Refund".to_string()),
                connector: payment_attempt
                    .connector
                    .clone()
                    .unwrap_or(DummyConnector4.to_string()),
                currency: *currency_vec
                    .get((num - 1) % currency_vec_len)
                    .unwrap_or(&common_enums::Currency::USD),
                total_amount: MinorUnit::new(amount * 100),
                refund_amount: MinorUnit::new(amount * 100),
                refund_status: common_enums::RefundStatus::Success,
                sent_to_gateway: true,
                refund_type: diesel_models::enums::RefundType::InstantRefund,
                metadata: None,
                refund_arn: None,
                profile_id: payment_intent.profile_id.clone(),
                updated_by: merchant_from_db.storage_scheme.to_string(),
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                charges: None,
                split_refunds: None,
                organization_id: org_id.clone(),
                processor_refund_data: None,
                processor_transaction_data,
            })
        } else {
            None
        };

        let dispute =
            if disputes_count < number_of_disputes && !is_failed_payment && refund.is_none() {
                disputes_count += 1;
                Some(DisputeNew {
                    dispute_id: common_utils::generate_id_with_default_len("test"),
                    amount: (amount * 100).to_string(),
                    currency: payment_intent
                        .currency
                        .unwrap_or(common_enums::Currency::USD)
                        .to_string(),
                    dispute_stage: storage_enums::DisputeStage::Dispute,
                    dispute_status: storage_enums::DisputeStatus::DisputeOpened,
                    payment_id: payment_id.clone(),
                    attempt_id: attempt_id.clone(),
                    merchant_id: merchant_id.clone(),
                    connector_status: "Sample connector status".into(),
                    connector_dispute_id: common_utils::generate_id_with_default_len("test"),
                    connector_reason: Some("Sample Dispute".into()),
                    connector_reason_code: Some("123".into()),
                    challenge_required_by: None,
                    connector_created_at: None,
                    connector_updated_at: None,
                    connector: payment_attempt
                        .connector
                        .clone()
                        .unwrap_or(DummyConnector4.to_string()),
                    evidence: None,
                    profile_id: payment_intent.profile_id.clone(),
                    merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                    dispute_amount: amount * 100,
                    organization_id: org_id.clone(),
                    dispute_currency: Some(payment_intent.currency.unwrap_or_default()),
                })
            } else {
                None
            };

        res.push((payment_intent, payment_attempt, refund, dispute));
    }
    Ok(res)
}

fn get_payment_method_type(num: u8) -> common_enums::PaymentMethodType {
    let rem: u8 = (num) % 2;
    match rem {
        0 => common_enums::PaymentMethodType::Debit,
        _ => common_enums::PaymentMethodType::Credit,
    }
}
