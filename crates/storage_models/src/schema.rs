// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    address (address_id) {
        id -> Int4,
        address_id -> Varchar,
        city -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
        line1 -> Nullable<Varchar>,
        line2 -> Nullable<Varchar>,
        line3 -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        zip -> Nullable<Varchar>,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        phone_number -> Nullable<Varchar>,
        country_code -> Nullable<Varchar>,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        customer_id -> Varchar,
        merchant_id -> Varchar,
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
        hash_key -> Varchar,
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

    configs (key) {
        id -> Int4,
        key -> Varchar,
        config -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    connector_response (id) {
        id -> Int4,
        payment_id -> Varchar,
        merchant_id -> Varchar,
        attempt_id -> Varchar,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        connector_name -> Nullable<Varchar>,
        connector_transaction_id -> Nullable<Varchar>,
        authentication_data -> Nullable<Json>,
        encoded_data -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    customers (customer_id, merchant_id) {
        id -> Int4,
        customer_id -> Varchar,
        merchant_id -> Varchar,
        name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        phone_country_code -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        metadata -> Nullable<Json>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    events (id) {
        id -> Int4,
        event_id -> Varchar,
        event_type -> EventType,
        event_class -> EventClass,
        is_webhook_notified -> Bool,
        intent_reference_id -> Nullable<Varchar>,
        primary_object_id -> Varchar,
        primary_object_type -> EventObjectType,
        created_at -> Timestamp,
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
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_account (id) {
        id -> Int4,
        merchant_id -> Varchar,
        api_key -> Nullable<Varchar>,
        return_url -> Nullable<Varchar>,
        enable_payment_response_hash -> Bool,
        payment_response_hash_key -> Nullable<Varchar>,
        redirect_to_merchant_with_http_post -> Bool,
        merchant_name -> Nullable<Varchar>,
        merchant_details -> Nullable<Json>,
        webhook_details -> Nullable<Json>,
        sub_merchants_enabled -> Nullable<Bool>,
        parent_merchant_id -> Nullable<Varchar>,
        publishable_key -> Nullable<Varchar>,
        storage_scheme -> MerchantStorageScheme,
        locker_id -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        routing_algorithm -> Nullable<Json>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::diesel_exports::*;

    merchant_connector_account (id) {
        id -> Int4,
        merchant_id -> Varchar,
        connector_name -> Varchar,
        connector_account_details -> Json,
        test_mode -> Nullable<Bool>,
        disabled -> Nullable<Bool>,
        merchant_connector_id -> Varchar,
        payment_methods_enabled -> Nullable<Array<Nullable<Json>>>,
        connector_type -> ConnectorType,
        metadata -> Nullable<Jsonb>,
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
        payment_method -> Nullable<PaymentMethodType>,
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
        payment_issuer -> Nullable<Varchar>,
        payment_experience -> Nullable<Varchar>,
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
        payment_method -> PaymentMethodType,
        payment_method_type -> Nullable<PaymentMethodSubType>,
        payment_method_issuer -> Nullable<Varchar>,
        payment_method_issuer_code -> Nullable<PaymentMethodIssuerCode>,
        metadata -> Nullable<Json>,
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
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    address,
    api_keys,
    configs,
    connector_response,
    customers,
    events,
    locker_mock_up,
    mandate,
    merchant_account,
    merchant_connector_account,
    payment_attempt,
    payment_intent,
    payment_methods,
    process_tracker,
    refund,
    reverse_lookup,
);
