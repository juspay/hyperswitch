// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    address (address_id) {
        id -> Nullable<Int4>,
        address_id -> Varchar,
        city -> Nullable<Varchar>,
        country -> Nullable<CountryAlpha2>,
        line1 -> Nullable<Bytea>,
        line2 -> Nullable<Bytea>,
        line3 -> Nullable<Bytea>,
        state -> Nullable<Bytea>,
        zip -> Nullable<Bytea>,
        first_name -> Nullable<Bytea>,
        last_name -> Nullable<Bytea>,
        phone_number -> Nullable<Bytea>,
        country_code -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        customer_id -> Nullable<Varchar>,
        merchant_id -> Varchar,
        payment_id -> Nullable<Varchar>,
        updated_by -> Varchar,
        email -> Nullable<Bytea>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    api_keys (key_id) {
        key_id -> Varchar,
        merchant_id -> Varchar,
        name -> Varchar,
        description -> Nullable<Varchar>,
        hashed_api_key -> Varchar,
        prefix -> Varchar,
        created_at -> Timestamp,
        expires_at -> Nullable<Timestamp>,
        last_used -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    authentication (authentication_id) {
        authentication_id -> Varchar,
        merchant_id -> Varchar,
        authentication_connector -> Varchar,
        connector_authentication_id -> Nullable<Varchar>,
        authentication_data -> Nullable<Jsonb>,
        payment_method_id -> Varchar,
        authentication_type -> Nullable<Varchar>,
        authentication_status -> Varchar,
        authentication_lifecycle_status -> Varchar,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        error_message -> Nullable<Varchar>,
        error_code -> Nullable<Varchar>,
        connector_metadata -> Nullable<Jsonb>,
        maximum_supported_version -> Nullable<Jsonb>,
        threeds_server_transaction_id -> Nullable<Varchar>,
        cavv -> Nullable<Varchar>,
        authentication_flow_type -> Nullable<Varchar>,
        message_version -> Nullable<Jsonb>,
        eci -> Nullable<Varchar>,
        trans_status -> Nullable<Varchar>,
        acquirer_bin -> Nullable<Varchar>,
        acquirer_merchant_id -> Nullable<Varchar>,
        three_ds_method_data -> Nullable<Varchar>,
        three_ds_method_url -> Nullable<Varchar>,
        acs_url -> Nullable<Varchar>,
        challenge_request -> Nullable<Varchar>,
        acs_reference_number -> Nullable<Varchar>,
        acs_trans_id -> Nullable<Varchar>,
        three_dsserver_trans_id -> Nullable<Varchar>,
        acs_signed_content -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    blocklist (id) {
        id -> Int4,
        merchant_id -> Varchar,
        fingerprint_id -> Varchar,
        data_kind -> BlocklistDataKind,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    blocklist_fingerprint (id) {
        id -> Int4,
        merchant_id -> Varchar,
        fingerprint_id -> Varchar,
        data_kind -> BlocklistDataKind,
        encrypted_fingerprint -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    blocklist_lookup (id) {
        id -> Int4,
        merchant_id -> Varchar,
        fingerprint -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    business_profile (profile_id) {
        profile_id -> Varchar,
        merchant_id -> Varchar,
        profile_name -> Varchar,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        return_url -> Nullable<Text>,
        enable_payment_response_hash -> Bool,
        payment_response_hash_key -> Nullable<Varchar>,
        redirect_to_merchant_with_http_post -> Bool,
        webhook_details -> Nullable<Json>,
        metadata -> Nullable<Json>,
        routing_algorithm -> Nullable<Json>,
        intent_fulfillment_time -> Nullable<Int8>,
        frm_routing_algorithm -> Nullable<Jsonb>,
        payout_routing_algorithm -> Nullable<Jsonb>,
        is_recon_enabled -> Bool,
        applepay_verified_domains -> Nullable<Array<Nullable<Text>>>,
        payment_link_config -> Nullable<Jsonb>,
        session_expiry -> Nullable<Int8>,
        authentication_connector_details -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    captures (capture_id) {
        capture_id -> Varchar,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        status -> CaptureStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        connector -> Varchar,
        error_message -> Nullable<Varchar>,
        error_code -> Nullable<Varchar>,
        error_reason -> Nullable<Varchar>,
        tax_amount -> Nullable<Int8>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        authorized_attempt_id -> Varchar,
        connector_capture_id -> Nullable<Varchar>,
        capture_sequence -> Int2,
        connector_response_reference_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    cards_info (card_iin) {
        card_iin -> Varchar,
        card_issuer -> Nullable<Text>,
        card_network -> Nullable<Text>,
        card_type -> Nullable<Text>,
        card_subtype -> Nullable<Text>,
        card_issuing_country -> Nullable<Text>,
        bank_code_id -> Nullable<Varchar>,
        bank_code -> Nullable<Varchar>,
        country_code -> Nullable<Varchar>,
        date_created -> Timestamp,
        last_updated -> Nullable<Timestamp>,
        last_updated_provider -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    configs (key) {
        id -> Int4,
        key -> Varchar,
        config -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    customers (customer_id, merchant_id) {
        id -> Int4,
        customer_id -> Varchar,
        merchant_id -> Varchar,
        name -> Nullable<Bytea>,
        email -> Nullable<Bytea>,
        phone -> Nullable<Bytea>,
        phone_country_code -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        metadata -> Nullable<Json>,
        connector_customer -> Nullable<Jsonb>,
        modified_at -> Timestamp,
        address_id -> Nullable<Varchar>,
        default_payment_method_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    dashboard_metadata (id) {
        id -> Int4,
        user_id -> Nullable<Varchar>,
        merchant_id -> Varchar,
        org_id -> Varchar,
        data_key -> DashboardMetadata,
        data_value -> Json,
        created_by -> Varchar,
        created_at -> Timestamp,
        last_modified_by -> Varchar,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    dispute (id) {
        id -> Int4,
        dispute_id -> Varchar,
        amount -> Varchar,
        currency -> Varchar,
        dispute_stage -> DisputeStage,
        dispute_status -> DisputeStatus,
        payment_id -> Varchar,
        attempt_id -> Varchar,
        merchant_id -> Varchar,
        connector_status -> Varchar,
        connector_dispute_id -> Varchar,
        connector_reason -> Nullable<Varchar>,
        connector_reason_code -> Nullable<Varchar>,
        challenge_required_by -> Nullable<Timestamp>,
        connector_created_at -> Nullable<Timestamp>,
        connector_updated_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        connector -> Varchar,
        evidence -> Jsonb,
        profile_id -> Nullable<Varchar>,
        merchant_connector_id -> Nullable<Varchar>,
        dispute_amount -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    events (event_id) {
        event_id -> Varchar,
        event_type -> EventType,
        event_class -> EventClass,
        is_webhook_notified -> Bool,
        primary_object_id -> Varchar,
        primary_object_type -> EventObjectType,
        created_at -> Timestamp,
        merchant_id -> Nullable<Varchar>,
        business_profile_id -> Nullable<Varchar>,
        primary_object_created_at -> Nullable<Timestamp>,
        idempotent_event_id -> Nullable<Varchar>,
        initial_attempt_id -> Nullable<Varchar>,
        request -> Nullable<Bytea>,
        response -> Nullable<Bytea>,
        delivery_attempt -> Nullable<WebhookDeliveryAttempt>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    file_metadata (file_id, merchant_id) {
        file_id -> Varchar,
        merchant_id -> Varchar,
        file_name -> Nullable<Varchar>,
        file_size -> Int4,
        file_type -> Varchar,
        provider_file_id -> Nullable<Varchar>,
        file_upload_provider -> Nullable<Varchar>,
        available -> Bool,
        created_at -> Timestamp,
        connector_label -> Nullable<Varchar>,
        profile_id -> Nullable<Varchar>,
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    fraud_check (frm_id, attempt_id, payment_id, merchant_id) {
        frm_id -> Varchar,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        attempt_id -> Varchar,
        created_at -> Timestamp,
        frm_name -> Varchar,
        frm_transaction_id -> Nullable<Varchar>,
        frm_transaction_type -> FraudCheckType,
        frm_status -> FraudCheckStatus,
        frm_score -> Nullable<Int4>,
        frm_reason -> Nullable<Jsonb>,
        frm_error -> Nullable<Varchar>,
        payment_details -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        modified_at -> Timestamp,
        last_step -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    gateway_status_map (connector, flow, sub_flow, code, message) {
        connector -> Varchar,
        flow -> Varchar,
        sub_flow -> Varchar,
        code -> Varchar,
        message -> Varchar,
        status -> Varchar,
        router_error -> Nullable<Varchar>,
        decision -> Varchar,
        created_at -> Timestamp,
        last_modified -> Timestamp,
        step_up_possible -> Bool,
        unified_code -> Nullable<Varchar>,
        unified_message -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    incremental_authorization (authorization_id, merchant_id) {
        authorization_id -> Varchar,
        merchant_id -> Varchar,
        payment_id -> Varchar,
        amount -> Int8,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        status -> Varchar,
        error_code -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        connector_authorization_id -> Nullable<Varchar>,
        previously_authorized_amount -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    locker_mock_up (id) {
        id -> Int4,
        card_id -> Varchar,
        external_id -> Varchar,
        card_fingerprint -> Varchar,
        card_global_fingerprint -> Varchar,
        merchant_id -> Varchar,
        card_number -> Varchar,
        card_exp_year -> Varchar,
        card_exp_month -> Varchar,
        name_on_card -> Nullable<Varchar>,
        nickname -> Nullable<Varchar>,
        customer_id -> Nullable<Varchar>,
        duplicate -> Nullable<Bool>,
        card_cvc -> Nullable<Varchar>,
        payment_method_id -> Nullable<Varchar>,
        enc_card_data -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    mandate (id) {
        id -> Int4,
        mandate_id -> Varchar,
        customer_id -> Varchar,
        merchant_id -> Varchar,
        payment_method_id -> Varchar,
        mandate_status -> MandateStatus,
        mandate_type -> MandateType,
        customer_accepted_at -> Nullable<Timestamp>,
        customer_ip_address -> Nullable<Varchar>,
        customer_user_agent -> Nullable<Varchar>,
        network_transaction_id -> Nullable<Varchar>,
        previous_attempt_id -> Nullable<Varchar>,
        created_at -> Timestamp,
        mandate_amount -> Nullable<Int8>,
        mandate_currency -> Nullable<Currency>,
        amount_captured -> Nullable<Int8>,
        connector -> Varchar,
        connector_mandate_id -> Nullable<Varchar>,
        start_date -> Nullable<Timestamp>,
        end_date -> Nullable<Timestamp>,
        metadata -> Nullable<Jsonb>,
        connector_mandate_ids -> Nullable<Jsonb>,
        original_payment_id -> Nullable<Varchar>,
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_account (id) {
        id -> Int4,
        merchant_id -> Varchar,
        return_url -> Nullable<Varchar>,
        enable_payment_response_hash -> Bool,
        payment_response_hash_key -> Nullable<Varchar>,
        redirect_to_merchant_with_http_post -> Bool,
        merchant_name -> Nullable<Bytea>,
        merchant_details -> Nullable<Bytea>,
        webhook_details -> Nullable<Json>,
        sub_merchants_enabled -> Nullable<Bool>,
        parent_merchant_id -> Nullable<Varchar>,
        publishable_key -> Nullable<Varchar>,
        storage_scheme -> MerchantStorageScheme,
        locker_id -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        routing_algorithm -> Nullable<Json>,
        primary_business_details -> Json,
        intent_fulfillment_time -> Nullable<Int8>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        frm_routing_algorithm -> Nullable<Jsonb>,
        payout_routing_algorithm -> Nullable<Jsonb>,
        organization_id -> Varchar,
        is_recon_enabled -> Bool,
        default_profile -> Nullable<Varchar>,
        recon_status -> ReconStatus,
        payment_link_config -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_connector_account (id) {
        id -> Int4,
        merchant_id -> Varchar,
        connector_name -> Varchar,
        connector_account_details -> Bytea,
        test_mode -> Nullable<Bool>,
        disabled -> Nullable<Bool>,
        merchant_connector_id -> Varchar,
        payment_methods_enabled -> Nullable<Array<Nullable<Json>>>,
        connector_type -> ConnectorType,
        metadata -> Nullable<Jsonb>,
        connector_label -> Nullable<Varchar>,
        business_country -> Nullable<CountryAlpha2>,
        business_label -> Nullable<Varchar>,
        business_sub_label -> Nullable<Varchar>,
        frm_configs -> Nullable<Jsonb>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        connector_webhook_details -> Nullable<Jsonb>,
        frm_config -> Nullable<Array<Nullable<Jsonb>>>,
        profile_id -> Nullable<Varchar>,
        applepay_verified_domains -> Nullable<Array<Nullable<Text>>>,
        pm_auth_config -> Nullable<Jsonb>,
        status -> ConnectorStatus,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_key_store (merchant_id) {
        merchant_id -> Varchar,
        key -> Bytea,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    organization (org_id) {
        org_id -> Varchar,
        org_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_attempt (id) {
        id -> Int4,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        attempt_id -> Varchar,
        status -> AttemptStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        save_to_locker -> Nullable<Bool>,
        connector -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        offer_amount -> Nullable<Int8>,
        surcharge_amount -> Nullable<Int8>,
        tax_amount -> Nullable<Int8>,
        payment_method_id -> Nullable<Varchar>,
        payment_method -> Nullable<Varchar>,
        connector_transaction_id -> Nullable<Varchar>,
        capture_method -> Nullable<CaptureMethod>,
        capture_on -> Nullable<Timestamp>,
        confirm -> Bool,
        authentication_type -> Nullable<AuthenticationType>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        last_synced -> Nullable<Timestamp>,
        cancellation_reason -> Nullable<Varchar>,
        amount_to_capture -> Nullable<Int8>,
        mandate_id -> Nullable<Varchar>,
        browser_info -> Nullable<Jsonb>,
        error_code -> Nullable<Varchar>,
        payment_token -> Nullable<Varchar>,
        connector_metadata -> Nullable<Jsonb>,
        payment_experience -> Nullable<Varchar>,
        payment_method_type -> Nullable<Varchar>,
        payment_method_data -> Nullable<Jsonb>,
        business_sub_label -> Nullable<Varchar>,
        straight_through_algorithm -> Nullable<Jsonb>,
        preprocessing_step_id -> Nullable<Varchar>,
        mandate_details -> Nullable<Jsonb>,
        error_reason -> Nullable<Text>,
        multiple_capture_count -> Nullable<Int2>,
        connector_response_reference_id -> Nullable<Varchar>,
        amount_capturable -> Int8,
        updated_by -> Varchar,
        merchant_connector_id -> Nullable<Varchar>,
        authentication_data -> Nullable<Json>,
        encoded_data -> Nullable<Text>,
        unified_code -> Nullable<Varchar>,
        unified_message -> Nullable<Varchar>,
        net_amount -> Nullable<Int8>,
        external_three_ds_authentication_attempted -> Nullable<Bool>,
        authentication_connector -> Nullable<Varchar>,
        authentication_id -> Nullable<Varchar>,
        mandate_data -> Nullable<Jsonb>,
        fingerprint_id -> Nullable<Varchar>,
        payment_method_billing_address_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_intent (id) {
        id -> Int4,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        status -> IntentStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        amount_captured -> Nullable<Int8>,
        customer_id -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
        return_url -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        connector_id -> Nullable<Varchar>,
        shipping_address_id -> Nullable<Varchar>,
        billing_address_id -> Nullable<Varchar>,
        statement_descriptor_name -> Nullable<Varchar>,
        statement_descriptor_suffix -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        last_synced -> Nullable<Timestamp>,
        setup_future_usage -> Nullable<FutureUsage>,
        off_session -> Nullable<Bool>,
        client_secret -> Nullable<Varchar>,
        active_attempt_id -> Varchar,
        business_country -> Nullable<CountryAlpha2>,
        business_label -> Nullable<Varchar>,
        order_details -> Nullable<Array<Nullable<Jsonb>>>,
        allowed_payment_method_types -> Nullable<Json>,
        connector_metadata -> Nullable<Json>,
        feature_metadata -> Nullable<Json>,
        attempt_count -> Int2,
        profile_id -> Nullable<Varchar>,
        merchant_decision -> Nullable<Varchar>,
        payment_link_id -> Nullable<Varchar>,
        payment_confirm_source -> Nullable<PaymentSource>,
        updated_by -> Varchar,
        surcharge_applicable -> Nullable<Bool>,
        request_incremental_authorization -> Nullable<RequestIncrementalAuthorization>,
        incremental_authorization_allowed -> Nullable<Bool>,
        authorization_count -> Nullable<Int4>,
        session_expiry -> Nullable<Timestamp>,
        fingerprint_id -> Nullable<Varchar>,
        request_external_three_ds_authentication -> Nullable<Bool>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_link (payment_link_id) {
        payment_link_id -> Varchar,
        payment_id -> Varchar,
        link_to_pay -> Varchar,
        merchant_id -> Varchar,
        amount -> Int8,
        currency -> Nullable<Currency>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        fulfilment_time -> Nullable<Timestamp>,
        custom_merchant_name -> Nullable<Varchar>,
        payment_link_config -> Nullable<Jsonb>,
        description -> Nullable<Varchar>,
        profile_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_methods (id) {
        id -> Int4,
        customer_id -> Varchar,
        merchant_id -> Varchar,
        payment_method_id -> Varchar,
        accepted_currency -> Nullable<Array<Nullable<Currency>>>,
        scheme -> Nullable<Varchar>,
        token -> Nullable<Varchar>,
        cardholder_name -> Nullable<Varchar>,
        issuer_name -> Nullable<Varchar>,
        issuer_country -> Nullable<Varchar>,
        payer_country -> Nullable<Array<Nullable<Text>>>,
        is_stored -> Nullable<Bool>,
        swift_code -> Nullable<Varchar>,
        direct_debit_token -> Nullable<Varchar>,
        created_at -> Timestamp,
        last_modified -> Timestamp,
        payment_method -> Varchar,
        payment_method_type -> Nullable<Varchar>,
        payment_method_issuer -> Nullable<Varchar>,
        payment_method_issuer_code -> Nullable<PaymentMethodIssuerCode>,
        metadata -> Nullable<Json>,
        payment_method_data -> Nullable<Bytea>,
        locker_id -> Nullable<Varchar>,
        last_used_at -> Timestamp,
        connector_mandate_details -> Nullable<Jsonb>,
        customer_acceptance -> Nullable<Jsonb>,
        status -> Varchar,
        network_transaction_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payout_attempt (payout_attempt_id) {
        payout_attempt_id -> Varchar,
        payout_id -> Varchar,
        customer_id -> Varchar,
        merchant_id -> Varchar,
        address_id -> Varchar,
        connector -> Nullable<Varchar>,
        connector_payout_id -> Varchar,
        payout_token -> Nullable<Varchar>,
        status -> PayoutStatus,
        is_eligible -> Nullable<Bool>,
        error_message -> Nullable<Text>,
        error_code -> Nullable<Varchar>,
        business_country -> Nullable<CountryAlpha2>,
        business_label -> Nullable<Varchar>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        profile_id -> Varchar,
        merchant_connector_id -> Nullable<Varchar>,
        routing_info -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payouts (payout_id) {
        payout_id -> Varchar,
        merchant_id -> Varchar,
        customer_id -> Varchar,
        address_id -> Varchar,
        payout_type -> PayoutType,
        payout_method_id -> Nullable<Varchar>,
        amount -> Int8,
        destination_currency -> Currency,
        source_currency -> Currency,
        description -> Nullable<Varchar>,
        recurring -> Bool,
        auto_fulfill -> Bool,
        return_url -> Nullable<Varchar>,
        entity_type -> Varchar,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        attempt_count -> Int2,
        profile_id -> Varchar,
        status -> PayoutStatus,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    process_tracker (id) {
        id -> Varchar,
        name -> Nullable<Varchar>,
        tag -> Array<Nullable<Text>>,
        runner -> Nullable<Varchar>,
        retry_count -> Int4,
        schedule_time -> Nullable<Timestamp>,
        rule -> Varchar,
        tracking_data -> Json,
        business_status -> Varchar,
        status -> ProcessTrackerStatus,
        event -> Array<Nullable<Text>>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    refund (id) {
        id -> Int4,
        internal_reference_id -> Varchar,
        refund_id -> Varchar,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        connector_transaction_id -> Varchar,
        connector -> Varchar,
        connector_refund_id -> Nullable<Varchar>,
        external_reference_id -> Nullable<Varchar>,
        refund_type -> RefundType,
        total_amount -> Int8,
        currency -> Currency,
        refund_amount -> Int8,
        refund_status -> RefundStatus,
        sent_to_gateway -> Bool,
        refund_error_message -> Nullable<Text>,
        metadata -> Nullable<Json>,
        refund_arn -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        description -> Nullable<Varchar>,
        attempt_id -> Varchar,
        refund_reason -> Nullable<Varchar>,
        refund_error_code -> Nullable<Text>,
        profile_id -> Nullable<Varchar>,
        updated_by -> Varchar,
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    reverse_lookup (lookup_id) {
        lookup_id -> Varchar,
        sk_id -> Varchar,
        pk_id -> Varchar,
        source -> Varchar,
        updated_by -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    roles (id) {
        id -> Int4,
        role_name -> Varchar,
        role_id -> Varchar,
        merchant_id -> Varchar,
        org_id -> Varchar,
        groups -> Array<Nullable<Text>>,
        scope -> RoleScope,
        created_at -> Timestamp,
        created_by -> Varchar,
        last_modified_at -> Timestamp,
        last_modified_by -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    routing_algorithm (algorithm_id) {
        algorithm_id -> Varchar,
        profile_id -> Varchar,
        merchant_id -> Varchar,
        name -> Varchar,
        description -> Nullable<Varchar>,
        kind -> RoutingAlgorithmKind,
        algorithm_data -> Jsonb,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        algorithm_for -> TransactionType,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    user_roles (id) {
        id -> Int4,
        user_id -> Varchar,
        merchant_id -> Varchar,
        role_id -> Varchar,
        org_id -> Varchar,
        status -> UserStatus,
        created_by -> Varchar,
        last_modified_by -> Varchar,
        created_at -> Timestamp,
        last_modified -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    users (id) {
        id -> Int4,
        user_id -> Varchar,
        email -> Varchar,
        name -> Varchar,
        password -> Varchar,
        is_verified -> Bool,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        preferred_merchant_id -> Nullable<Varchar>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    address,
    api_keys,
    authentication,
    blocklist,
    blocklist_fingerprint,
    blocklist_lookup,
    business_profile,
    captures,
    cards_info,
    configs,
    customers,
    dashboard_metadata,
    dispute,
    events,
    file_metadata,
    fraud_check,
    gateway_status_map,
    incremental_authorization,
    locker_mock_up,
    mandate,
    merchant_account,
    merchant_connector_account,
    merchant_key_store,
    organization,
    payment_attempt,
    payment_intent,
    payment_link,
    payment_methods,
    payout_attempt,
    payouts,
    process_tracker,
    refund,
    reverse_lookup,
    roles,
    routing_algorithm,
    user_roles,
    users,
);
