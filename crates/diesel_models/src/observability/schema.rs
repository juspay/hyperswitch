// @generated automatically by Diesel CLI.

diesel::table! {
    alerts_dicts (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 255]
        key_ -> Varchar,
        product -> Nullable<Json>,
        values_ -> Nullable<Json>,
        ts_created -> Nullable<Timestamp>,
        is_enabled -> Nullable<Bool>,
        #[max_length = 64]
        username -> Nullable<Varchar>,
        metadata -> Nullable<Json>,
    }
}

diesel::table! {
    alerts_info (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 255]
        dimensions -> Nullable<Varchar>,
        period -> Nullable<Int4>,
        #[max_length = 64]
        default_channel -> Nullable<Varchar>,
        default_critical -> Nullable<Bool>,
        blacklist -> Nullable<Json>,
        snooze -> Nullable<Json>,
        history_window -> Nullable<Int4>,
        thresholds -> Nullable<Json>,
        metadata -> Nullable<Json>,
        is_enabled -> Nullable<Bool>,
        comments -> Nullable<Json>,
        call_period -> Nullable<Int4>,
        #[max_length = 64]
        author -> Nullable<Varchar>,
        #[max_length = 64]
        approver -> Nullable<Varchar>,
        last_updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alerts_intermediate (id_intermediate) {
        id_intermediate -> Uuid,
        id -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Nullable<Jsonb>,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Nullable<Timestamp>,
        latest_ts_alert -> Nullable<Timestamp>,
        max_duration -> Nullable<Int4>,
        other_metrics -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        rca_metadata -> Nullable<Jsonb>,
        #[max_length = 64]
        group_id -> Varchar,
        #[max_length = 64]
        priority -> Nullable<Varchar>,
        last_updated_at -> Nullable<Timestamp>,
        recovered_ts -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alerts_intermediate_xyne (id_intermediate) {
        id_intermediate -> Uuid,
        id -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Nullable<Jsonb>,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Nullable<Timestamp>,
        latest_ts_alert -> Nullable<Timestamp>,
        max_duration -> Nullable<Int4>,
        other_metrics -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        rca_metadata -> Nullable<Jsonb>,
        #[max_length = 64]
        group_id -> Varchar,
        #[max_length = 64]
        priority -> Nullable<Varchar>,
        last_updated_at -> Nullable<Timestamp>,
        recovered_ts -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alerts_main (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Nullable<Json>,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Nullable<Timestamp>,
        duration -> Nullable<Int4>,
        sent -> Nullable<Bool>,
        critical -> Nullable<Bool>,
        rca_metadata -> Nullable<Jsonb>,
        metadata -> Nullable<Json>,
        last_updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alerts_main_xyne (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        dimensions -> Nullable<Json>,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Nullable<Timestamp>,
        duration -> Nullable<Int4>,
        sent -> Nullable<Bool>,
        critical -> Nullable<Bool>,
        rca_metadata -> Nullable<Jsonb>,
        metadata -> Nullable<Json>,
        last_updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    merchant_thresholds (id) {
        id -> Uuid,
        #[max_length = 64]
        name -> Nullable<Varchar>,
        #[max_length = 64]
        product -> Nullable<Varchar>,
        #[max_length = 64]
        merchant_id -> Nullable<Varchar>,
        #[max_length = 64]
        profile_id -> Varchar,
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
        author -> Nullable<Varchar>,
        is_enabled -> Nullable<Bool>,
        last_updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    merchants_alert_external (id_merchant_table) {
        id -> Nullable<Uuid>,
        id_merchant_table -> Uuid,
        id_intermediate -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        dimensions -> Nullable<Jsonb>,
        auxiliary_dimensions -> Nullable<Jsonb>,
        current_metric -> Float8,
        expected_metric -> Float8,
        #[max_length = 255]
        attribution -> Nullable<Varchar>,
        max_duration -> Int4,
        start_time -> Timestamp,
        is_visible -> Bool,
        recovered_ts -> Nullable<Timestamp>,
        #[max_length = 255]
        ts_slack -> Varchar,
        ts_alert -> Timestamp,
        latest_ts_alert -> Nullable<Timestamp>,
        last_updated_at -> Nullable<Timestamp>,
        slack_info -> Nullable<Jsonb>,
        communication_info -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        #[max_length = 64]
        priority -> Nullable<Varchar>,
        #[max_length = 64]
        tenant_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    merchants_alert_external_config (name, product) {
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        category -> Nullable<Varchar>,
        is_enabled -> Nullable<Bool>,
        metadata -> Nullable<Jsonb>,
        last_updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    merchants_alert_external_dimension (id_merchant_table) {
        id -> Nullable<Uuid>,
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
        dimensions -> Nullable<Jsonb>,
        auxiliary_dimensions -> Nullable<Jsonb>,
        current_metric -> Float8,
        expected_metric -> Float8,
        #[max_length = 255]
        attribution -> Nullable<Varchar>,
        max_duration -> Int4,
        is_visible -> Bool,
        start_time -> Timestamp,
        recovered_ts -> Nullable<Timestamp>,
        #[max_length = 255]
        ts_slack -> Nullable<Varchar>,
        ts_alert -> Timestamp,
        latest_ts_alert -> Nullable<Timestamp>,
        last_updated_at -> Nullable<Timestamp>,
        slack_info -> Nullable<Jsonb>,
        communication_info -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        #[max_length = 64]
        priority -> Nullable<Varchar>,
        #[max_length = 64]
        tenant_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    merchants_alert_external_xyne (id_merchant_table) {
        id -> Nullable<Uuid>,
        id_merchant_table -> Uuid,
        id_intermediate -> Nullable<Uuid>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 64]
        product -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        dimensions -> Nullable<Jsonb>,
        auxiliary_dimensions -> Nullable<Jsonb>,
        current_metric -> Float8,
        expected_metric -> Float8,
        #[max_length = 255]
        attribution -> Nullable<Varchar>,
        max_duration -> Int4,
        start_time -> Timestamp,
        is_visible -> Bool,
        recovered_ts -> Nullable<Timestamp>,
        #[max_length = 255]
        ts_slack -> Varchar,
        ts_alert -> Timestamp,
        latest_ts_alert -> Nullable<Timestamp>,
        last_updated_at -> Nullable<Timestamp>,
        slack_info -> Nullable<Jsonb>,
        communication_info -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        metadata_alert_details -> Nullable<Jsonb>,
        #[max_length = 64]
        priority -> Nullable<Varchar>,
        #[max_length = 64]
        tenant_id -> Nullable<Varchar>,
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
diesel::joinable!(alerts_intermediate_xyne -> alerts_main_xyne (id));
diesel::joinable!(merchants_alert_external -> alerts_main (id));
diesel::joinable!(merchants_alert_external_dimension -> alerts_main (id));
diesel::joinable!(merchants_alert_external_xyne -> alerts_main_xyne (id));

diesel::allow_tables_to_appear_in_same_query!(
    alerts_dicts,
    alerts_info,
    alerts_intermediate,
    alerts_intermediate_xyne,
    alerts_main,
    alerts_main_xyne,
    merchant_thresholds,
    merchants_alert_external,
    merchants_alert_external_config,
    merchants_alert_external_dimension,
    merchants_alert_external_xyne,
    notification_reads,
);
