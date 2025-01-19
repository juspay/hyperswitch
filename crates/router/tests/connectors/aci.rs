#![allow(clippy::print_stdout)]

use std::{borrow::Cow, marker::PhantomData, str::FromStr, sync::Arc};

use common_utils::id_type;
use hyperswitch_domain_models::address::{Address, AddressDetails, PhoneDetails};
use masking::Secret;
use router::{
    configs::settings::Settings,
    core::payments,
    db::StorageImpl,
    routes, services,
    types::{self, storage::enums, PaymentAddress},
};
use tokio::sync::oneshot;

use crate::{connector_auth::ConnectorAuthentication, utils};

fn construct_payment_router_data() -> types::PaymentsAuthorizeRouterData {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    let merchant_id = id_type::MerchantId::try_from(Cow::from("aci")).unwrap();

    types::RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: Some(id_type::CustomerId::try_from(Cow::from("aci")).unwrap()),
        tenant_id: id_type::TenantId::try_from_string("public".to_string()).unwrap(),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethod::Card,
        connector_auth_type: utils::to_connector_auth_type(auth.into()),
        description: Some("This is a test".to_string()),
        payment_method_status: None,
        request: types::PaymentsAuthorizeData {
            amount: 1000,
            currency: enums::Currency::USD,
            payment_method_data: types::domain::PaymentMethodData::Card(types::domain::Card {
                card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_cvc: Secret::new("999".to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(Secret::new("nick_name".into())),
                card_holder_name: Some(Secret::new("card holder name".into())),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            customer_name: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: None,
            customer_id: None,
            surcharge_details: None,
            request_incremental_authorization: false,
            metadata: None,
            authentication_data: None,
            customer_acceptance: None,
            ..utils::PaymentAuthorizeType::default().0
        },
        response: Err(types::ErrorResponse::default()),
        address: PaymentAddress::new(
            None,
            None,
            Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("John".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                    ..Default::default()
                }),
                phone: Some(PhoneDetails {
                    number: Some(Secret::new("9123456789".to_string())),
                    country_code: Some("+351".to_string()),
                }),
                email: None,
            }),
            None,
        ),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        connector_response: None,
        preprocessing_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        apple_pay_flow: None,
        external_latency: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    }
}

fn construct_refund_router_data<F>() -> types::RefundsRouterData<F> {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    let merchant_id = id_type::MerchantId::try_from(Cow::from("aci")).unwrap();

    types::RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: Some(id_type::CustomerId::try_from(Cow::from("aci")).unwrap()),
        tenant_id: id_type::TenantId::try_from_string("public".to_string()).unwrap(),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        payment_method_status: None,
        status: enums::AttemptStatus::default(),
        payment_method: enums::PaymentMethod::Card,
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_auth_type: utils::to_connector_auth_type(auth.into()),
        description: Some("This is a test".to_string()),
        request: types::RefundsData {
            payment_amount: 1000,
            currency: enums::Currency::USD,

            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 100,
            webhook_url: None,
            connector_metadata: None,
            reason: None,
            connector_refund_id: None,
            browser_info: None,
            ..utils::PaymentRefundType::default().0
        },
        response: Err(types::ErrorResponse::default()),
        address: PaymentAddress::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        connector_response: None,
        preprocessing_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        apple_pay_flow: None,
        external_latency: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    }
}

#[actix_web::test]
async fn payments_create_success() {
    let conf = Settings::new().unwrap();
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let app_state = Box::pin(routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();

    use router::connector::Aci;
    let connector = utils::construct_connector_data_old(
        Box::new(Aci::new()),
        types::Connector::Aci,
        types::api::GetToken::Connector,
        None,
    );
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
}

#[actix_web::test]
#[ignore]
async fn payments_create_failure() {
    {
        let conf = Settings::new().unwrap();
        use router::connector::Aci;
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;
        let state = Arc::new(app_state)
            .get_session_state(
                &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
        let connector = utils::construct_connector_data_old(
            Box::new(Aci::new()),
            types::Connector::Aci,
            types::api::GetToken::Connector,
            None,
        );
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            types::api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let mut request = construct_payment_router_data();
        request.request.payment_method_data =
            types::domain::PaymentMethodData::Card(types::domain::Card {
                card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_cvc: Secret::new("99".to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(Secret::new("nick_name".into())),
                card_holder_name: Some(Secret::new("card holder name".into())),
            });

        let response = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .is_err();
        println!("{response:?}");
        assert!(response, "The payment was intended to fail but it passed");
    }
}

#[actix_web::test]
async fn refund_for_successful_payments() {
    let conf = Settings::new().unwrap();
    use router::connector::Aci;
    let connector = utils::construct_connector_data_old(
        Box::new(Aci::new()),
        types::Connector::Aci,
        types::api::GetToken::Connector,
        None,
    );
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let app_state = Box::pin(routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        types::api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut refund_request = construct_refund_router_data();
    refund_request.request.connector_transaction_id = match response.response.unwrap() {
        types::PaymentsResponseData::TransactionResponse { resource_id, .. } => {
            resource_id.get_connector_transaction_id().unwrap()
        }
        _ => panic!("Connector transaction id not found"),
    };
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &refund_request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    println!("{response:?}");
    assert!(
        response.response.unwrap().refund_status == enums::RefundStatus::Success,
        "The refund transaction failed"
    );
}

#[actix_web::test]
#[ignore]
async fn refunds_create_failure() {
    let conf = Settings::new().unwrap();
    use router::connector::Aci;
    let connector = utils::construct_connector_data_old(
        Box::new(Aci::new()),
        types::Connector::Aci,
        types::api::GetToken::Connector,
        None,
    );
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let app_state = Box::pin(routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
    let connector_integration: services::BoxedRefundConnectorIntegrationInterface<
        types::api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut request = construct_refund_router_data();
    request.request.connector_transaction_id = "1234".to_string();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .is_err();
    println!("{response:?}");
    assert!(response, "The refund was intended to fail but it passed");
}
