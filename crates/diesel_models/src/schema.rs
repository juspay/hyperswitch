// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    address (address_id) {
        id -> Nullable<Int4>,
        #[max_length = 64]
        address_id -> Varchar,
        #[max_length = 128]
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
        #[max_length = 8]
        country_code -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        #[max_length = 64]
        customer_id -> Nullable<Varchar>,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        payment_id -> Nullable<Varchar>,
        #[max_length = 32]
        updated_by -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    api_keys (key_id) {
        #[max_length = 64]
        key_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 256]
        description -> Nullable<Varchar>,
        #[max_length = 128]
        hashed_api_key -> Varchar,
        #[max_length = 16]
        prefix -> Varchar,
        created_at -> Timestamp,
        expires_at -> Nullable<Timestamp>,
        last_used -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    business_profile (profile_id) {
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        profile_name -> Varchar,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        return_url -> Nullable<Text>,
        enable_payment_response_hash -> Bool,
        #[max_length = 255]
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
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    captures (capture_id) {
        #[max_length = 64]
        capture_id -> Varchar,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        status -> CaptureStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        #[max_length = 255]
        connector -> Varchar,
        #[max_length = 255]
        error_message -> Nullable<Varchar>,
        #[max_length = 255]
        error_code -> Nullable<Varchar>,
        #[max_length = 255]
        error_reason -> Nullable<Varchar>,
        tax_amount -> Nullable<Int8>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        #[max_length = 64]
        authorized_attempt_id -> Varchar,
        #[max_length = 128]
        connector_capture_id -> Nullable<Varchar>,
        capture_sequence -> Int2,
        #[max_length = 128]
        connector_response_reference_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    cards_info (card_iin) {
        #[max_length = 16]
        card_iin -> Varchar,
        card_issuer -> Nullable<Text>,
        card_network -> Nullable<Text>,
        card_type -> Nullable<Text>,
        card_subtype -> Nullable<Text>,
        card_issuing_country -> Nullable<Text>,
        #[max_length = 32]
        bank_code_id -> Nullable<Varchar>,
        #[max_length = 32]
        bank_code -> Nullable<Varchar>,
        #[max_length = 32]
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
        #[max_length = 255]
        key -> Varchar,
        config -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    connector_response (id) {
        id -> Int4,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        attempt_id -> Varchar,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        #[max_length = 64]
        connector_name -> Nullable<Varchar>,
        #[max_length = 128]
        connector_transaction_id -> Nullable<Varchar>,
        authentication_data -> Nullable<Json>,
        encoded_data -> Nullable<Text>,
        #[max_length = 32]
        updated_by -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    customers (customer_id, merchant_id) {
        id -> Int4,
        #[max_length = 64]
        customer_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        name -> Nullable<Bytea>,
        email -> Nullable<Bytea>,
        phone -> Nullable<Bytea>,
        #[max_length = 8]
        phone_country_code -> Nullable<Varchar>,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        metadata -> Nullable<Json>,
        connector_customer -> Nullable<Jsonb>,
        modified_at -> Timestamp,
        #[max_length = 64]
        address_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    dispute (id) {
        id -> Int4,
        #[max_length = 64]
        dispute_id -> Varchar,
        #[max_length = 255]
        amount -> Varchar,
        #[max_length = 255]
        currency -> Varchar,
        dispute_stage -> DisputeStage,
        dispute_status -> DisputeStatus,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        attempt_id -> Varchar,
        #[max_length = 255]
        merchant_id -> Varchar,
        #[max_length = 255]
        connector_status -> Varchar,
        #[max_length = 255]
        connector_dispute_id -> Varchar,
        #[max_length = 255]
        connector_reason -> Nullable<Varchar>,
        #[max_length = 255]
        connector_reason_code -> Nullable<Varchar>,
        challenge_required_by -> Nullable<Timestamp>,
        connector_created_at -> Nullable<Timestamp>,
        connector_updated_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        #[max_length = 255]
        connector -> Varchar,
        evidence -> Jsonb,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    events (id) {
        id -> Int4,
        #[max_length = 64]
        event_id -> Varchar,
        event_type -> EventType,
        event_class -> EventClass,
        is_webhook_notified -> Bool,
        #[max_length = 64]
        intent_reference_id -> Nullable<Varchar>,
        #[max_length = 64]
        primary_object_id -> Varchar,
        primary_object_type -> EventObjectType,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    file_metadata (file_id, merchant_id) {
        #[max_length = 64]
        file_id -> Varchar,
        #[max_length = 255]
        merchant_id -> Varchar,
        #[max_length = 255]
        file_name -> Nullable<Varchar>,
        file_size -> Int4,
        #[max_length = 255]
        file_type -> Varchar,
        #[max_length = 255]
        provider_file_id -> Nullable<Varchar>,
        #[max_length = 255]
        file_upload_provider -> Nullable<Varchar>,
        available -> Bool,
        created_at -> Timestamp,
        #[max_length = 255]
        connector_label -> Nullable<Varchar>,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    fraud_check (frm_id, attempt_id, payment_id, merchant_id) {
        #[max_length = 64]
        frm_id -> Varchar,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        attempt_id -> Varchar,
        created_at -> Timestamp,
        #[max_length = 255]
        frm_name -> Varchar,
        #[max_length = 255]
        frm_transaction_id -> Nullable<Varchar>,
        frm_transaction_type -> FraudCheckType,
        frm_status -> FraudCheckStatus,
        frm_score -> Nullable<Int4>,
        frm_reason -> Nullable<Jsonb>,
        #[max_length = 255]
        frm_error -> Nullable<Varchar>,
        payment_details -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        modified_at -> Timestamp,
        #[max_length = 64]
        last_step -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    locker_mock_up (id) {
        id -> Int4,
        #[max_length = 255]
        card_id -> Varchar,
        #[max_length = 255]
        external_id -> Varchar,
        #[max_length = 255]
        card_fingerprint -> Varchar,
        #[max_length = 255]
        card_global_fingerprint -> Varchar,
        #[max_length = 255]
        merchant_id -> Varchar,
        #[max_length = 255]
        card_number -> Varchar,
        #[max_length = 255]
        card_exp_year -> Varchar,
        #[max_length = 255]
        card_exp_month -> Varchar,
        #[max_length = 255]
        name_on_card -> Nullable<Varchar>,
        #[max_length = 255]
        nickname -> Nullable<Varchar>,
        #[max_length = 255]
        customer_id -> Nullable<Varchar>,
        duplicate -> Nullable<Bool>,
        #[max_length = 8]
        card_cvc -> Nullable<Varchar>,
        #[max_length = 64]
        payment_method_id -> Nullable<Varchar>,
        enc_card_data -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    mandate (id) {
        id -> Int4,
        #[max_length = 64]
        mandate_id -> Varchar,
        #[max_length = 64]
        customer_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        payment_method_id -> Varchar,
        mandate_status -> MandateStatus,
        mandate_type -> MandateType,
        customer_accepted_at -> Nullable<Timestamp>,
        #[max_length = 64]
        customer_ip_address -> Nullable<Varchar>,
        #[max_length = 255]
        customer_user_agent -> Nullable<Varchar>,
        #[max_length = 128]
        network_transaction_id -> Nullable<Varchar>,
        #[max_length = 64]
        previous_attempt_id -> Nullable<Varchar>,
        created_at -> Timestamp,
        mandate_amount -> Nullable<Int8>,
        mandate_currency -> Nullable<Currency>,
        amount_captured -> Nullable<Int8>,
        #[max_length = 64]
        connector -> Varchar,
        #[max_length = 128]
        connector_mandate_id -> Nullable<Varchar>,
        start_date -> Nullable<Timestamp>,
        end_date -> Nullable<Timestamp>,
        metadata -> Nullable<Jsonb>,
        connector_mandate_ids -> Nullable<Jsonb>,
        #[max_length = 64]
        original_payment_id -> Nullable<Varchar>,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_account (id) {
        id -> Int4,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 255]
        return_url -> Nullable<Varchar>,
        enable_payment_response_hash -> Bool,
        #[max_length = 255]
        payment_response_hash_key -> Nullable<Varchar>,
        redirect_to_merchant_with_http_post -> Bool,
        merchant_name -> Nullable<Bytea>,
        merchant_details -> Nullable<Bytea>,
        webhook_details -> Nullable<Json>,
        sub_merchants_enabled -> Nullable<Bool>,
        #[max_length = 64]
        parent_merchant_id -> Nullable<Varchar>,
        #[max_length = 128]
        publishable_key -> Nullable<Varchar>,
        storage_scheme -> MerchantStorageScheme,
        #[max_length = 64]
        locker_id -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        routing_algorithm -> Nullable<Json>,
        primary_business_details -> Json,
        intent_fulfillment_time -> Nullable<Int8>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        frm_routing_algorithm -> Nullable<Jsonb>,
        payout_routing_algorithm -> Nullable<Jsonb>,
        #[max_length = 32]
        organization_id -> Varchar,
        is_recon_enabled -> Bool,
        #[max_length = 64]
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
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        connector_name -> Varchar,
        connector_account_details -> Bytea,
        test_mode -> Nullable<Bool>,
        disabled -> Nullable<Bool>,
        #[max_length = 128]
        merchant_connector_id -> Varchar,
        payment_methods_enabled -> Nullable<Array<Nullable<Json>>>,
        connector_type -> ConnectorType,
        metadata -> Nullable<Jsonb>,
        #[max_length = 255]
        connector_label -> Nullable<Varchar>,
        business_country -> Nullable<CountryAlpha2>,
        #[max_length = 255]
        business_label -> Nullable<Varchar>,
        #[max_length = 64]
        business_sub_label -> Nullable<Varchar>,
        frm_configs -> Nullable<Jsonb>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        connector_webhook_details -> Nullable<Jsonb>,
        frm_config -> Nullable<Array<Nullable<Jsonb>>>,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        applepay_verified_domains -> Nullable<Array<Nullable<Text>>>,
        pm_auth_config -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_key_store (merchant_id) {
        #[max_length = 64]
        merchant_id -> Varchar,
        key -> Bytea,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    organization (org_id) {
        #[max_length = 32]
        org_id -> Varchar,
        org_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_attempt (id) {
        id -> Int4,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        attempt_id -> Varchar,
        status -> AttemptStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        save_to_locker -> Nullable<Bool>,
        #[max_length = 64]
        connector -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        offer_amount -> Nullable<Int8>,
        surcharge_amount -> Nullable<Int8>,
        tax_amount -> Nullable<Int8>,
        #[max_length = 64]
        payment_method_id -> Nullable<Varchar>,
        payment_method -> Nullable<Varchar>,
        #[max_length = 128]
        connector_transaction_id -> Nullable<Varchar>,
        capture_method -> Nullable<CaptureMethod>,
        capture_on -> Nullable<Timestamp>,
        confirm -> Bool,
        authentication_type -> Nullable<AuthenticationType>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        last_synced -> Nullable<Timestamp>,
        #[max_length = 255]
        cancellation_reason -> Nullable<Varchar>,
        amount_to_capture -> Nullable<Int8>,
        #[max_length = 64]
        mandate_id -> Nullable<Varchar>,
        browser_info -> Nullable<Jsonb>,
        #[max_length = 255]
        error_code -> Nullable<Varchar>,
        #[max_length = 128]
        payment_token -> Nullable<Varchar>,
        connector_metadata -> Nullable<Jsonb>,
        #[max_length = 50]
        payment_experience -> Nullable<Varchar>,
        #[max_length = 64]
        payment_method_type -> Nullable<Varchar>,
        payment_method_data -> Nullable<Jsonb>,
        #[max_length = 64]
        business_sub_label -> Nullable<Varchar>,
        straight_through_algorithm -> Nullable<Jsonb>,
        preprocessing_step_id -> Nullable<Varchar>,
        mandate_details -> Nullable<Jsonb>,
        error_reason -> Nullable<Text>,
        multiple_capture_count -> Nullable<Int2>,
        #[max_length = 128]
        connector_response_reference_id -> Nullable<Varchar>,
        amount_capturable -> Int8,
        #[max_length = 32]
        updated_by -> Varchar,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
        authentication_data -> Nullable<Json>,
        encoded_data -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_intent (id) {
        id -> Int4,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        status -> IntentStatus,
        amount -> Int8,
        currency -> Nullable<Currency>,
        amount_captured -> Nullable<Int8>,
        #[max_length = 64]
        customer_id -> Nullable<Varchar>,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        #[max_length = 255]
        return_url -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        #[max_length = 64]
        connector_id -> Nullable<Varchar>,
        #[max_length = 64]
        shipping_address_id -> Nullable<Varchar>,
        #[max_length = 64]
        billing_address_id -> Nullable<Varchar>,
        #[max_length = 255]
        statement_descriptor_name -> Nullable<Varchar>,
        #[max_length = 255]
        statement_descriptor_suffix -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        last_synced -> Nullable<Timestamp>,
        setup_future_usage -> Nullable<FutureUsage>,
        off_session -> Nullable<Bool>,
        #[max_length = 128]
        client_secret -> Nullable<Varchar>,
        #[max_length = 64]
        active_attempt_id -> Varchar,
        business_country -> Nullable<CountryAlpha2>,
        #[max_length = 64]
        business_label -> Nullable<Varchar>,
        order_details -> Nullable<Array<Nullable<Jsonb>>>,
        allowed_payment_method_types -> Nullable<Json>,
        connector_metadata -> Nullable<Json>,
        feature_metadata -> Nullable<Json>,
        attempt_count -> Int2,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        #[max_length = 64]
        merchant_decision -> Nullable<Varchar>,
        #[max_length = 255]
        payment_link_id -> Nullable<Varchar>,
        payment_confirm_source -> Nullable<PaymentSource>,
        #[max_length = 32]
        updated_by -> Varchar,
        surcharge_applicable -> Nullable<Bool>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_link (payment_link_id) {
        #[max_length = 255]
        payment_link_id -> Varchar,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 255]
        link_to_pay -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        amount -> Int8,
        currency -> Nullable<Currency>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        fulfilment_time -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payment_methods (id) {
        id -> Int4,
        #[max_length = 64]
        customer_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        payment_method_id -> Varchar,
        accepted_currency -> Nullable<Array<Nullable<Currency>>>,
        #[max_length = 32]
        scheme -> Nullable<Varchar>,
        #[max_length = 128]
        token -> Nullable<Varchar>,
        #[max_length = 255]
        cardholder_name -> Nullable<Varchar>,
        #[max_length = 64]
        issuer_name -> Nullable<Varchar>,
        #[max_length = 64]
        issuer_country -> Nullable<Varchar>,
        payer_country -> Nullable<Array<Nullable<Text>>>,
        is_stored -> Nullable<Bool>,
        #[max_length = 32]
        swift_code -> Nullable<Varchar>,
        #[max_length = 128]
        direct_debit_token -> Nullable<Varchar>,
        created_at -> Timestamp,
        last_modified -> Timestamp,
        payment_method -> Varchar,
        #[max_length = 64]
        payment_method_type -> Nullable<Varchar>,
        #[max_length = 128]
        payment_method_issuer -> Nullable<Varchar>,
        payment_method_issuer_code -> Nullable<PaymentMethodIssuerCode>,
        metadata -> Nullable<Json>,
        payment_method_data -> Nullable<Bytea>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payout_attempt (payout_attempt_id) {
        #[max_length = 64]
        payout_attempt_id -> Varchar,
        #[max_length = 64]
        payout_id -> Varchar,
        #[max_length = 64]
        customer_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        address_id -> Varchar,
        #[max_length = 64]
        connector -> Varchar,
        #[max_length = 128]
        connector_payout_id -> Varchar,
        #[max_length = 64]
        payout_token -> Nullable<Varchar>,
        status -> PayoutStatus,
        is_eligible -> Nullable<Bool>,
        error_message -> Nullable<Text>,
        #[max_length = 64]
        error_code -> Nullable<Varchar>,
        business_country -> Nullable<CountryAlpha2>,
        #[max_length = 64]
        business_label -> Nullable<Varchar>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    payouts (payout_id) {
        #[max_length = 64]
        payout_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        customer_id -> Varchar,
        #[max_length = 64]
        address_id -> Varchar,
        payout_type -> PayoutType,
        #[max_length = 64]
        payout_method_id -> Nullable<Varchar>,
        amount -> Int8,
        destination_currency -> Currency,
        source_currency -> Currency,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        recurring -> Bool,
        auto_fulfill -> Bool,
        #[max_length = 255]
        return_url -> Nullable<Varchar>,
        #[max_length = 64]
        entity_type -> Varchar,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    process_tracker (id) {
        #[max_length = 127]
        id -> Varchar,
        #[max_length = 64]
        name -> Nullable<Varchar>,
        tag -> Array<Nullable<Text>>,
        #[max_length = 64]
        runner -> Nullable<Varchar>,
        retry_count -> Int4,
        schedule_time -> Nullable<Timestamp>,
        #[max_length = 255]
        rule -> Varchar,
        tracking_data -> Json,
        #[max_length = 255]
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
        #[max_length = 64]
        internal_reference_id -> Varchar,
        #[max_length = 64]
        refund_id -> Varchar,
        #[max_length = 64]
        payment_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 128]
        connector_transaction_id -> Varchar,
        #[max_length = 64]
        connector -> Varchar,
        #[max_length = 128]
        connector_refund_id -> Nullable<Varchar>,
        #[max_length = 64]
        external_reference_id -> Nullable<Varchar>,
        refund_type -> RefundType,
        total_amount -> Int8,
        currency -> Currency,
        refund_amount -> Int8,
        refund_status -> RefundStatus,
        sent_to_gateway -> Bool,
        refund_error_message -> Nullable<Text>,
        metadata -> Nullable<Json>,
        #[max_length = 128]
        refund_arn -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        #[max_length = 64]
        attempt_id -> Varchar,
        #[max_length = 255]
        refund_reason -> Nullable<Varchar>,
        refund_error_code -> Nullable<Text>,
        #[max_length = 64]
        profile_id -> Nullable<Varchar>,
        #[max_length = 32]
        updated_by -> Varchar,
        #[max_length = 32]
        merchant_connector_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    reverse_lookup (lookup_id) {
        #[max_length = 128]
        lookup_id -> Varchar,
        #[max_length = 128]
        sk_id -> Varchar,
        #[max_length = 128]
        pk_id -> Varchar,
        #[max_length = 128]
        source -> Varchar,
        #[max_length = 32]
        updated_by -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    routing_algorithm (algorithm_id) {
        #[max_length = 64]
        algorithm_id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 256]
        description -> Nullable<Varchar>,
        kind -> RoutingAlgorithmKind,
        algorithm_data -> Jsonb,
        created_at -> Timestamp,
        modified_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    address,
    api_keys,
    business_profile,
    captures,
    cards_info,
    configs,
    connector_response,
    customers,
    dispute,
    events,
    file_metadata,
    fraud_check,
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
    routing_algorithm,
);
