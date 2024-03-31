use api_models::{
    enums::Connector::{DummyConnector4, DummyConnector7},
    user::sample_data::SampleDataRequest,
};
use data_models::payments::payment_intent::PaymentIntentNew;
use diesel_models::{user::sample_data::PaymentAttemptBatchNew, RefundNew};
use error_stack::{IntoReport, ResultExt};
use rand::{prelude::SliceRandom, thread_rng, Rng};
use time::OffsetDateTime;

use crate::{
    consts,
    core::errors::sample_data::{SampleDataError, SampleDataResult},
    AppState,
};

#[allow(clippy::type_complexity)]
pub async fn generate_sample_data(
    state: &AppState,
    req: SampleDataRequest,
    merchant_id: &str,
) -> SampleDataResult<Vec<(PaymentIntentNew, PaymentAttemptBatchNew, Option<RefundNew>)>> {
    let merchant_id = merchant_id.to_string();
    let sample_data_size: usize = req.record.unwrap_or(100);

    if !(10..=100).contains(&sample_data_size) {
        return Err(SampleDataError::InvalidRange.into());
    }

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id.as_str(),
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)?;

    let merchant_from_db = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id.as_str(), &key_store)
        .await
        .change_context::<SampleDataError>(SampleDataError::DataDoesNotExist)?;

    let merchant_parsed_details: Vec<api_models::admin::PrimaryBusinessDetails> =
        serde_json::from_value(merchant_from_db.primary_business_details.clone())
            .into_report()
            .change_context(SampleDataError::InternalServerError)
            .attach_printable("Error while parsing primary business details")?;

    let business_country_default = merchant_parsed_details.first().map(|x| x.country);

    let business_label_default = merchant_parsed_details.first().map(|x| x.business.clone());

    let profile_id = match crate::core::utils::get_profile_id_from_business_details(
        business_country_default,
        business_label_default.as_ref(),
        &merchant_from_db,
        req.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await
    {
        Ok(id) => id,
        Err(error) => {
            router_env::logger::error!(
                "Profile ID not found in business details. Attempting to fetch from the database {error:?}"
            );

            state
                .store
                .list_business_profile_by_merchant_id(&merchant_id)
                .await
                .change_context(SampleDataError::InternalServerError)
                .attach_printable("Failed to get business profile")?
                .first()
                .ok_or(SampleDataError::InternalServerError)?
                .profile_id
                .clone()
        }
    };

    // 10 percent payments should be failed
    #[allow(clippy::as_conversions)]
    let failure_attempts = usize::try_from((sample_data_size as f32 / 10.0).round() as i64)
        .into_report()
        .change_context(SampleDataError::InvalidParameters)?;

    let failure_after_attempts = sample_data_size / failure_attempts;

    // 20 percent refunds for payments
    #[allow(clippy::as_conversions)]
    let number_of_refunds = usize::try_from((sample_data_size as f32 / 5.0).round() as i64)
        .into_report()
        .change_context(SampleDataError::InvalidParameters)?;

    let mut refunds_count = 0;

    let mut random_array: Vec<usize> = (1..=sample_data_size).collect();

    // Shuffle the array
    let mut rng = thread_rng();
    random_array.shuffle(&mut rng);

    let mut res: Vec<(PaymentIntentNew, PaymentAttemptBatchNew, Option<RefundNew>)> = Vec::new();
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

    for num in 1..=sample_data_size {
        let payment_id = common_utils::generate_id_with_default_len("test");
        let attempt_id = crate::utils::get_payment_attempt_id(&payment_id, 1);
        let client_secret = common_utils::generate_id(
            consts::ID_LENGTH,
            format!("{}_secret", payment_id.clone()).as_str(),
        );
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

        let payment_intent = PaymentIntentNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            status: match is_failed_payment {
                true => common_enums::IntentStatus::Failed,
                _ => common_enums::IntentStatus::Succeeded,
            },
            amount: amount * 100,
            currency: Some(
                *currency_vec
                    .get((num - 1) % currency_vec_len)
                    .unwrap_or(&common_enums::Currency::USD),
            ),
            description: Some("This is a sample payment".to_string()),
            created_at: Some(created_at),
            modified_at: Some(modified_at),
            last_synced: Some(last_synced),
            client_secret: Some(client_secret),
            business_country: business_country_default,
            business_label: business_label_default.clone(),
            active_attempt: data_models::RemoteStorageObject::ForeignID(attempt_id.clone()),
            attempt_count: 1,
            customer_id: Some("hs-dashboard-user".to_string()),
            amount_captured: Some(amount * 100),
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
        };
        let payment_attempt = PaymentAttemptBatchNew {
            attempt_id: attempt_id.clone(),
            payment_id: payment_id.clone(),
            connector_transaction_id: Some(attempt_id.clone()),
            merchant_id: merchant_id.clone(),
            status: match is_failed_payment {
                true => common_enums::AttemptStatus::Failure,
                _ => common_enums::AttemptStatus::Charged,
            },
            amount: amount * 100,
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
            created_at: Some(created_at),
            modified_at: Some(modified_at),
            last_synced: Some(last_synced),
            amount_to_capture: Some(amount * 100),
            connector_response_reference_id: Some(attempt_id.clone()),
            updated_by: merchant_from_db.storage_scheme.to_string(),

            ..Default::default()
        };

        let refund = if refunds_count < number_of_refunds && !is_failed_payment {
            refunds_count += 1;
            Some(RefundNew {
                refund_id: common_utils::generate_id_with_default_len("test"),
                internal_reference_id: common_utils::generate_id_with_default_len("test"),
                external_reference_id: None,
                payment_id: payment_id.clone(),
                attempt_id: attempt_id.clone(),
                merchant_id: merchant_id.clone(),
                connector_transaction_id: attempt_id.clone(),
                connector_refund_id: None,
                description: Some("This is a sample refund".to_string()),
                created_at: Some(created_at),
                modified_at: Some(modified_at),
                refund_reason: Some("Sample Refund".to_string()),
                connector: payment_attempt
                    .connector
                    .clone()
                    .unwrap_or(DummyConnector4.to_string()),
                currency: *currency_vec
                    .get((num - 1) % currency_vec_len)
                    .unwrap_or(&common_enums::Currency::USD),
                total_amount: amount * 100,
                refund_amount: amount * 100,
                refund_status: common_enums::RefundStatus::Success,
                sent_to_gateway: true,
                refund_type: diesel_models::enums::RefundType::InstantRefund,
                metadata: None,
                refund_arn: None,
                profile_id: payment_intent.profile_id.clone(),
                updated_by: merchant_from_db.storage_scheme.to_string(),
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
            })
        } else {
            None
        };

        res.push((payment_intent, payment_attempt, refund));
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
