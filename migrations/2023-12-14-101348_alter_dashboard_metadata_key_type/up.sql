-- Your SQL goes here
CREATE TYPE "DashboardMetadata" AS ENUM (
    'production_agreement',
    'setup_processor',
    'configure_endpoint',
    'setup_complete',
    'first_processor_connected',
    'second_processor_connected',
    'configured_routing',
    'test_payment',
    'integration_method',
    'stripe_connected',
    'paypal_connected',
    'sp_routing_configured',
    'sp_test_payment',
    'download_woocom',
    'configure_woocom',
    'setup_woocom_webhook',
    'is_multiple_configuration',
    'configuration_type',
    'feedback',
    'prod_intent'
);

ALTER TABLE dashboard_metadata ALTER COLUMN data_key TYPE "DashboardMetadata" USING (data_key::"DashboardMetadata");