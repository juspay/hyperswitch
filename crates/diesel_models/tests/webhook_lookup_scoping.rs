#![cfg(feature = "v1")]
#![allow(clippy::panic_in_result_fn)]

use std::{borrow::Cow, error::Error};

use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::{
    date_time,
    id_type::{MerchantConnectorAccountId, MerchantId, OrganizationId, PaymentId, ProfileId},
    types::{ConnectorTransactionId, MinorUnit},
};
use diesel::{delete, pg::PgConnection, prelude::*, update};
use diesel_models::{
    enums::{AttemptStatus, Currency, RefundStatus, RefundType},
    schema::{payment_attempt::dsl as pa_dsl, refund::dsl as refund_dsl},
    PaymentAttempt, PaymentAttemptNew, PgPooledConn, Refund, RefundNew,
};

const CONNECTOR: &str = "stripe";

async fn build_pool(
    database_url: String,
) -> Result<
    bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>,
    Box<dyn Error + Send + Sync>,
> {
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let pool = bb8::Pool::builder()
        .max_size(5)
        .connection_timeout(std::time::Duration::from_secs(30))
        .build(manager)
        .await?;
    Ok(pool)
}

fn merchant_id(suffix: &str) -> Result<MerchantId, Box<dyn Error + Send + Sync>> {
    Ok(MerchantId::try_from(Cow::from(format!("mer_wb_{suffix}")))?)
}

fn merchant_connector_account_id(
    suffix: &str,
) -> Result<MerchantConnectorAccountId, Box<dyn Error + Send + Sync>> {
    Ok(MerchantConnectorAccountId::try_from(Cow::from(
        suffix.to_owned(),
    ))?)
}

fn payment_attempt_new(
    merchant_id: &MerchantId,
    payment_id: &PaymentId,
    profile_id: &ProfileId,
    organization_id: &OrganizationId,
    attempt_id: String,
    merchant_connector_id: Option<MerchantConnectorAccountId>,
) -> PaymentAttemptNew {
    PaymentAttemptNew {
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        attempt_id,
        status: AttemptStatus::Charged,
        amount: MinorUnit::new(100),
        currency: Some(Currency::USD),
        save_to_locker: None,
        connector: Some(CONNECTOR.to_string()),
        error_message: None,
        offer_amount: None,
        surcharge_amount: None,
        tax_amount: None,
        payment_method_id: None,
        payment_method: None,
        capture_method: None,
        capture_on: None,
        confirm: true,
        authentication_type: None,
        created_at: date_time::now(),
        modified_at: date_time::now(),
        last_synced: None,
        cancellation_reason: None,
        amount_to_capture: None,
        mandate_id: None,
        browser_info: None,
        payment_token: None,
        error_code: None,
        connector_metadata: None,
        payment_experience: None,
        payment_method_type: None,
        payment_method_data: None,
        business_sub_label: None,
        straight_through_algorithm: None,
        preprocessing_step_id: None,
        mandate_details: None,
        error_reason: None,
        connector_response_reference_id: None,
        multiple_capture_count: None,
        amount_capturable: MinorUnit::new(0),
        updated_by: "webhook_lookup_scoping_test".to_string(),
        merchant_connector_id,
        authentication_data: None,
        encoded_data: None,
        unified_code: None,
        unified_message: None,
        net_amount: None,
        external_three_ds_authentication_attempted: None,
        authentication_connector: None,
        authentication_id: None,
        mandate_data: None,
        fingerprint_id: None,
        payment_method_billing_address_id: None,
        client_source: None,
        client_version: None,
        customer_acceptance: None,
        profile_id: profile_id.clone(),
        organization_id: organization_id.clone(),
        card_network: None,
        shipping_cost: None,
        order_tax_amount: None,
        connector_mandate_detail: None,
        request_extended_authorization: None,
        extended_authorization_applied: None,
        capture_before: None,
        card_discovery: None,
        processor_merchant_id: Some(merchant_id.clone()),
        created_by: None,
        setup_future_usage_applied: None,
        routing_approach: None,
        connector_request_reference_id: None,
        network_transaction_id: None,
        network_details: None,
        is_stored_credential: None,
        authorized_amount: None,
        extended_authorization_last_applied_at: None,
        tokenization: None,
        encrypted_payment_method_data: None,
        error_details: None,
        retry_type: None,
        installment_data: None,
        external_surcharge_details: None,
        network_transaction_link_id: None,
        sender_payment_instrument_id: None,
        external_threeds_authentication_type: None,
        applied_offer_details: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn refund_new(
    merchant_id: &MerchantId,
    payment_id: &PaymentId,
    organization_id: &OrganizationId,
    refund_id: String,
    internal_reference_id: String,
    attempt_id: String,
    connector_transaction_id: String,
    connector_refund_id: String,
    merchant_connector_id: Option<MerchantConnectorAccountId>,
) -> RefundNew {
    RefundNew {
        refund_id,
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        internal_reference_id,
        external_reference_id: None,
        connector_transaction_id: ConnectorTransactionId::from(connector_transaction_id),
        connector: CONNECTOR.to_string(),
        connector_refund_id: Some(ConnectorTransactionId::from(connector_refund_id)),
        refund_type: RefundType::InstantRefund,
        total_amount: MinorUnit::new(100),
        currency: Currency::USD,
        refund_amount: MinorUnit::new(100),
        refund_status: RefundStatus::Success,
        sent_to_gateway: true,
        metadata: None,
        refund_arn: None,
        created_at: date_time::now(),
        modified_at: date_time::now(),
        description: None,
        attempt_id,
        refund_reason: None,
        profile_id: None,
        updated_by: "webhook_lookup_scoping_test".to_string(),
        merchant_connector_id,
        charges: None,
        organization_id: organization_id.clone(),
        split_refunds: None,
        processor_refund_data: None,
        processor_transaction_data: None,
        processor_merchant_id: Some(merchant_id.clone()),
        created_by: None,
    }
}

#[tokio::test]
async fn payment_lookup_is_scoped_by_merchant_connector_id(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipped: DATABASE_URL not set");
        return Ok(());
    };
    let pool = build_pool(database_url).await?;
    let pooled = pool.get().await?;
    let conn: &PgPooledConn = &pooled;

    let suffix = std::process::id().to_string();
    let merchant_id = merchant_id(&suffix)?;
    let profile_id = ProfileId::try_from(Cow::from("prof_wb"))?;
    let organization_id = OrganizationId::try_from(Cow::from("org_wb"))?;
    let mca1 = merchant_connector_account_id(&format!("mca_wb_{suffix}_1"))?;
    let mca2 = merchant_connector_account_id(&format!("mca_wb_{suffix}_2"))?;
    let connector_transaction_id = format!("conn_txn_wb_{suffix}");

    let payment_id_1 = PaymentId::try_from(Cow::from(format!("pay_wb_{suffix}_1")))?;
    let payment_id_2 = PaymentId::try_from(Cow::from(format!("pay_wb_{suffix}_2")))?;
    let attempt_id_1 = format!("pa_wb_1_{suffix}");
    let attempt_id_2 = format!("pa_wb_2_{suffix}");

    let _attempt_1 = payment_attempt_new(
        &merchant_id,
        &payment_id_1,
        &profile_id,
        &organization_id,
        attempt_id_1.clone(),
        Some(mca1.clone()),
    )
    .insert(conn)
    .await?;

    let _attempt_2 = payment_attempt_new(
        &merchant_id,
        &payment_id_2,
        &profile_id,
        &organization_id,
        attempt_id_2.clone(),
        Some(mca2.clone()),
    )
    .insert(conn)
    .await?;

    for attempt_id in [attempt_id_1, attempt_id_2] {
        update(
            pa_dsl::payment_attempt.filter(
                pa_dsl::attempt_id
                    .eq(attempt_id)
                    .and(pa_dsl::merchant_id.eq(merchant_id.clone())),
            ),
        )
        .set(pa_dsl::connector_transaction_id.eq(Some(connector_transaction_id.clone())))
        .execute_async(conn)
        .await?;
    }

    let result_mca1 = PaymentAttempt::find_by_processor_merchant_id_connector_txn_id(
        conn,
        &merchant_id,
        &connector_transaction_id,
        Some(&mca1),
    )
    .await?;
    assert_eq!(result_mca1.merchant_connector_id, Some(mca1.clone()));

    let result_mca2 = PaymentAttempt::find_by_processor_merchant_id_connector_txn_id(
        conn,
        &merchant_id,
        &connector_transaction_id,
        Some(&mca2),
    )
    .await?;
    assert_eq!(result_mca2.merchant_connector_id, Some(mca2.clone()));

    let result_any = PaymentAttempt::find_by_processor_merchant_id_connector_txn_id(
        conn,
        &merchant_id,
        &connector_transaction_id,
        None,
    )
    .await?;
    assert!(result_any.merchant_connector_id.is_some());

    delete(pa_dsl::payment_attempt.filter(pa_dsl::merchant_id.eq(merchant_id)))
        .execute_async(conn)
        .await?;

    Ok(())
}

#[tokio::test]
async fn refund_lookup_is_scoped_by_merchant_connector_id(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipped: DATABASE_URL not set");
        return Ok(());
    };
    let pool = build_pool(database_url).await?;
    let pooled = pool.get().await?;
    let conn: &PgPooledConn = &pooled;

    let suffix = std::process::id().to_string();
    let merchant_id = merchant_id(&format!("ref_{suffix}"))?;
    let organization_id = OrganizationId::try_from(Cow::from("org_wb_ref"))?;
    let mca1 = merchant_connector_account_id(&format!("mca_wb_ref_{suffix}_1"))?;
    let mca2 = merchant_connector_account_id(&format!("mca_wb_ref_{suffix}_2"))?;
    let connector_refund_id = format!("connector_refund_id_wb_{suffix}");

    let payment_id_1 = PaymentId::try_from(Cow::from(format!("pay_wb_ref_{suffix}_1")))?;
    let payment_id_2 = PaymentId::try_from(Cow::from(format!("pay_wb_ref_{suffix}_2")))?;
    let refund_id_1 = format!("refund_wb_1_{suffix}");
    let refund_id_2 = format!("refund_wb_2_{suffix}");
    let attempt_id_1 = format!("pa_wb_ref_1_{suffix}");
    let attempt_id_2 = format!("pa_wb_ref_2_{suffix}");
    let connector_transaction_id = format!("conn_txn_wb_ref_{suffix}");

    let _refund_1 = refund_new(
        &merchant_id,
        &payment_id_1,
        &organization_id,
        refund_id_1,
        format!("internal_ref_1_{suffix}"),
        attempt_id_1,
        connector_transaction_id.clone(),
        connector_refund_id.clone(),
        Some(mca1.clone()),
    )
    .insert(conn)
    .await?;

    let _refund_2 = refund_new(
        &merchant_id,
        &payment_id_2,
        &organization_id,
        refund_id_2,
        format!("internal_ref_2_{suffix}"),
        attempt_id_2,
        connector_transaction_id.clone(),
        connector_refund_id.clone(),
        Some(mca2.clone()),
    )
    .insert(conn)
    .await?;

    let result_mca1 = Refund::find_by_processor_merchant_id_connector_refund_id_connector(
        conn,
        &merchant_id,
        &connector_refund_id,
        CONNECTOR,
        Some(&mca1),
    )
    .await?;
    assert_eq!(result_mca1.merchant_connector_id, Some(mca1.clone()));

    let result_mca2 = Refund::find_by_processor_merchant_id_connector_refund_id_connector(
        conn,
        &merchant_id,
        &connector_refund_id,
        CONNECTOR,
        Some(&mca2),
    )
    .await?;
    assert_eq!(result_mca2.merchant_connector_id, Some(mca2.clone()));

    let result_any = Refund::find_by_processor_merchant_id_connector_refund_id_connector(
        conn,
        &merchant_id,
        &connector_refund_id,
        CONNECTOR,
        None,
    )
    .await?;
    assert!(result_any.merchant_connector_id.is_some());

    delete(refund_dsl::refund.filter(refund_dsl::merchant_id.eq(merchant_id)))
        .execute_async(conn)
        .await?;

    Ok(())
}
