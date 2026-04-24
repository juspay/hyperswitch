-- ============================================================================
-- Bucket 3 / config-level features — Postgres: business_profile table
-- ============================================================================
-- Output: feature, enabled_count, last_modified
--
-- `enabled_count` > 0 means at least one business_profile has the feature
-- enabled → mark prod_used = yes in bucket 3.
--
-- Run once against your prod Postgres and export as CSV:
--   \copy (<query>) TO '~/Downloads/b3_pg_business_profile.csv' WITH CSV HEADER
-- ============================================================================

SELECT feature, enabled_count, last_modified FROM (

SELECT 'Auto Retries' AS feature,
       COUNT(*) FILTER (WHERE is_auto_retries_enabled IS TRUE) AS enabled_count,
       MAX(modified_at) FILTER (WHERE is_auto_retries_enabled IS TRUE)::text AS last_modified
FROM business_profile

UNION ALL
SELECT 'Manual Retry',
       COUNT(*) FILTER (WHERE is_manual_retry_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_manual_retry_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Clear PAN Retries',
       COUNT(*) FILTER (WHERE is_clear_pan_retries_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_clear_pan_retries_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Iframe Redirection',
       COUNT(*) FILTER (WHERE is_iframe_redirection_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_iframe_redirection_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'External Vault',
       COUNT(*) FILTER (WHERE is_external_vault_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_external_vault_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Connector Agnostic MIT',
       COUNT(*) FILTER (WHERE is_connector_agnostic_mit_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_connector_agnostic_mit_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Tax Connector',
       COUNT(*) FILTER (WHERE is_tax_connector_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_tax_connector_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Extended Card Info',
       COUNT(*) FILTER (WHERE is_extended_card_info_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_extended_card_info_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Use Billing As PM Billing',
       COUNT(*) FILTER (WHERE use_billing_as_payment_method_billing IS TRUE),
       MAX(modified_at) FILTER (WHERE use_billing_as_payment_method_billing IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Reconciliation',
       COUNT(*) FILTER (WHERE is_recon_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_recon_enabled IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Payment Response Hash',
       COUNT(*) FILTER (WHERE enable_payment_response_hash IS TRUE),
       MAX(modified_at) FILTER (WHERE enable_payment_response_hash IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Redirect Method',
       COUNT(*) FILTER (WHERE redirect_to_merchant_with_http_post IS TRUE),
       MAX(modified_at) FILTER (WHERE redirect_to_merchant_with_http_post IS TRUE)::text
FROM business_profile

UNION ALL
SELECT 'Card Testing Guard',
       COUNT(*) FILTER (WHERE card_testing_guard_config IS NOT NULL),
       MAX(modified_at) FILTER (WHERE card_testing_guard_config IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Session Expiry',
       COUNT(*) FILTER (WHERE session_expiry IS NOT NULL),
       MAX(modified_at) FILTER (WHERE session_expiry IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Dispute Polling Interval',
       COUNT(*) FILTER (WHERE dispute_polling_interval IS NOT NULL),
       MAX(modified_at) FILTER (WHERE dispute_polling_interval IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Merchant Category Code',
       COUNT(*) FILTER (WHERE merchant_category_code IS NOT NULL),
       MAX(modified_at) FILTER (WHERE merchant_category_code IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Merchant Country Code',
       COUNT(*) FILTER (WHERE merchant_country_code IS NOT NULL),
       MAX(modified_at) FILTER (WHERE merchant_country_code IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Outgoing Webhook Custom Headers',
       COUNT(*) FILTER (WHERE outgoing_webhook_custom_http_headers IS NOT NULL),
       MAX(modified_at) FILTER (WHERE outgoing_webhook_custom_http_headers IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Network Tokenization Credentials',
       COUNT(*) FILTER (WHERE network_tokenization_credentials IS NOT NULL),
       MAX(modified_at) FILTER (WHERE network_tokenization_credentials IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Acquirer Config Map',
       COUNT(*) FILTER (WHERE acquirer_config_map IS NOT NULL),
       MAX(modified_at) FILTER (WHERE acquirer_config_map IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Dynamic Routing',
       COUNT(*) FILTER (WHERE dynamic_routing_algorithm IS NOT NULL),
       MAX(modified_at) FILTER (WHERE dynamic_routing_algorithm IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'FRM Routing Algorithm',
       COUNT(*) FILTER (WHERE frm_routing_algorithm IS NOT NULL),
       MAX(modified_at) FILTER (WHERE frm_routing_algorithm IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Payout Routing Algorithm',
       COUNT(*) FILTER (WHERE payout_routing_algorithm IS NOT NULL),
       MAX(modified_at) FILTER (WHERE payout_routing_algorithm IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT '3DS Decision Rule Algorithm',
       COUNT(*) FILTER (WHERE three_ds_decision_rule_algorithm IS NOT NULL),
       MAX(modified_at) FILTER (WHERE three_ds_decision_rule_algorithm IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Default Fallback Routing',
       COUNT(*) FILTER (WHERE default_fallback_routing IS NOT NULL),
       MAX(modified_at) FILTER (WHERE default_fallback_routing IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'External 3DS Authentication (config)',
       COUNT(*) FILTER (WHERE authentication_connector_details IS NOT NULL),
       MAX(modified_at) FILTER (WHERE authentication_connector_details IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Payment Link',
       COUNT(*) FILTER (WHERE payment_link_config IS NOT NULL),
       MAX(modified_at) FILTER (WHERE payment_link_config IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Payout Link',
       COUNT(*) FILTER (WHERE payout_link_config IS NOT NULL),
       MAX(modified_at) FILTER (WHERE payout_link_config IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'Webhook Details (config)',
       COUNT(*) FILTER (WHERE webhook_details IS NOT NULL),
       MAX(modified_at) FILTER (WHERE webhook_details IS NOT NULL)::text
FROM business_profile

UNION ALL
SELECT 'L2/L3 Data Processing',
       COUNT(*) FILTER (WHERE is_l2_l3_enabled IS TRUE),
       MAX(modified_at) FILTER (WHERE is_l2_l3_enabled IS TRUE)::text
FROM business_profile

) t ORDER BY feature;
