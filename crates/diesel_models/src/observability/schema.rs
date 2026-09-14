// @generated automatically by Diesel CLI.

diesel::table! {
    alerts_dicts (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 255]
        key_ -> Varchar,
        product -> Json,
        values_ -> Json,
        ts_created -> Timestamp,
        is_enabled -> Bool,
        #[max_length = 64]
        username -> Varchar,
        metadata -> Json,
    }
}

diesel::table! {
    alerts_info (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 255]
        dimensions -> Varchar,
        period -> Int4,
        #[max_length = 64]
        default_channel -> Nullable<Varchar>,
        default_critical -> Bool,
        blacklist -> Nullable<Json>,
        snooze -> Nullable<Json>,
        history_window -> Nullable<Int4>,
        thresholds -> Nullable<Json>,
        metadata -> Nullable<Json>,
        is_enabled -> Bool,
        comments -> Nullable<Json>,
        call_period -> Nullable<Int4>,
        #[max_length = 64]
        author -> Varchar,
        #[max_length = 64]
        approver -> Nullable<Varchar>,
        last_updated_at -> Timestamp,
    }
}

diesel::table! {
    alerts_intermediate (id_intermediate) {
        id_intermediate -> Uuid,
        #[max_length = 64]
        channel -> Varchar,
        id -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Jsonb,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Timestamp,
        latest_ts_alert -> Timestamp,
        max_duration -> Int4,
        other_metrics -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        rca_metadata -> Jsonb,
        #[max_length = 64]
        group_id -> Varchar,
        #[max_length = 64]
        priority -> Varchar,
        last_updated_at -> Timestamp,
        recovered_ts -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alerts_main (id) {
        id -> Uuid,
        #[max_length = 64]
        channel -> Varchar,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Json,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Timestamp,
        duration -> Int4,
        sent -> Bool,
        critical -> Bool,
        rca_metadata -> Jsonb,
        metadata -> Nullable<Json>,
        last_updated_at -> Timestamp,
    }
}

diesel::table! {
    merchant_thresholds (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        thresholds_min_volume -> Nullable<Float8>,
        thresholds_min_impacted_volume -> Nullable<Float8>,
        thresholds_tolerance -> Nullable<Float8>,
        thresholds_diff_threshold -> Nullable<Float8>,
        thresholds_merchant_impact -> Nullable<Float8>,
        thresholds_alert_period -> Nullable<Float8>,
        thresholds_min_observations -> Nullable<Float8>,
        thresholds_min_history_volume -> Nullable<Float8>,
        thresholds_filter_percentile -> Nullable<Float8>,
        thresholds_current_min_volume -> Nullable<Float8>,
        metadata -> Nullable<Jsonb>,
        #[max_length = 64]
        author -> Varchar,
        is_enabled -> Bool,
        last_updated_at -> Timestamp,
    }
}

diesel::table! {
    merchants_alert_external (id_merchant_table) {
        id -> Uuid,
        #[max_length = 64]
        channel -> Varchar,
        id_merchant_table -> Uuid,
        id_intermediate -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        dimensions -> Jsonb,
        auxiliary_dimensions -> Jsonb,
        current_metric -> Nullable<Float8>,
        expected_metric -> Nullable<Float8>,
        #[max_length = 255]
        attribution -> Varchar,
        max_duration -> Nullable<Int4>,
        start_time -> Nullable<Timestamp>,
        is_visible -> Bool,
        recovered_ts -> Nullable<Timestamp>,
        #[max_length = 255]
        ts_slack -> Varchar,
        ts_alert -> Timestamp,
        latest_ts_alert -> Nullable<Timestamp>,
        last_updated_at -> Timestamp,
        slack_info -> Jsonb,
        communication_info -> Jsonb,
        metadata -> Jsonb,
        metadata_alert_details -> Jsonb,
        #[max_length = 64]
        priority -> Varchar,
        #[max_length = 64]
        tenant_id -> Varchar,
    }
}

diesel::table! {
    merchants_alert_external_config (name, product) {
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        category -> Varchar,
        is_enabled -> Bool,
        metadata -> Jsonb,
        last_updated_at -> Timestamp,
    }
}

diesel::table! {
    merchants_alert_external_dimension (id_merchant_table) {
        id -> Uuid,
        #[max_length = 64]
        channel -> Varchar,
        id_merchant_table -> Uuid,
        id_intermediate -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        dimension_key -> Varchar,
        #[max_length = 255]
        dimension_value -> Varchar,
        dimensions -> Jsonb,
        auxiliary_dimensions -> Jsonb,
        current_metric -> Nullable<Float8>,
        expected_metric -> Nullable<Float8>,
        #[max_length = 255]
        attribution -> Varchar,
        max_duration -> Nullable<Int4>,
        is_visible -> Bool,
        start_time -> Nullable<Timestamp>,
        recovered_ts -> Nullable<Timestamp>,
        #[max_length = 255]
        ts_slack -> Varchar,
        ts_alert -> Timestamp,
        latest_ts_alert -> Nullable<Timestamp>,
        last_updated_at -> Timestamp,
        slack_info -> Jsonb,
        communication_info -> Jsonb,
        metadata -> Jsonb,
        metadata_alert_details -> Jsonb,
        #[max_length = 64]
        priority -> Varchar,
        #[max_length = 64]
        tenant_id -> Varchar,
    }
}

diesel::table! {
    notification_reads (user_name) {
        #[max_length = 255]
        user_name -> Varchar,
        last_read_at -> Timestamp,
    }
}

diesel::joinable!(alerts_intermediate -> alerts_main (id));
diesel::joinable!(merchants_alert_external -> alerts_main (id));
diesel::joinable!(merchants_alert_external_dimension -> alerts_main (id));

diesel::allow_tables_to_appear_in_same_query!(
    alerts_dicts,
    alerts_info,
    alerts_intermediate,
    alerts_main,
    merchant_thresholds,
    merchants_alert_external,
    merchants_alert_external_config,
    merchants_alert_external_dimension,
    notification_reads,
);
