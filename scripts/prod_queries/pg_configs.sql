-- ============================================================================
-- Bucket 3 config features — Postgres: configs key-value table
-- ============================================================================
-- Output: feature, enabled_count, last_modified (always NULL — configs table
--         has no timestamp columns, only key and config)
-- Export as CSV: ~/Downloads/b3_pg_configs.csv
--
-- The `configs` table stores arbitrary key-value pairs with schema:
--   key (varchar), config (text)
-- No modified_at column. Features are detected by the presence of keys
-- matching a pattern.
-- ============================================================================

SELECT feature, enabled_count, last_modified FROM (

SELECT 'Gateway Status Map (GSM)' AS feature,
       COUNT(*) AS enabled_count,
       NULL::text AS last_modified
FROM configs WHERE key LIKE 'should_call_gsm%'

UNION ALL
SELECT 'PM Modular Service', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_call_pm_modular_service%'

UNION ALL
SELECT 'Authentication Service Eligibility', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'authentication_service_eligible%'

UNION ALL
SELECT 'Eligibility Check', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_perform_eligibility%'

UNION ALL
SELECT 'Eligibility Data Storage For Auth', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_store_eligibility_check_data_for_authentication%'

UNION ALL
SELECT '3DS Routing Region UAS', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'threeds_routing_region_uas%'

UNION ALL
SELECT 'Vault Tokenization Disable', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_disable_vault_tokenization%'

UNION ALL
SELECT 'Conditional Routing DSL', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'dsl_%' OR key = 'dsl'

UNION ALL
SELECT 'Surcharge DSL', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'surcharge_dsl%'

UNION ALL
SELECT 'Connector API Version Override', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'connector_api_version%'

UNION ALL
SELECT 'Implicit Customer Update', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'implicit_customer_update%'

UNION ALL
SELECT 'MIT With Limited Card Data', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_enable_mit_with_limited_card_data%'

UNION ALL
SELECT 'Requires CVV', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'requires_cvv%'

UNION ALL
SELECT 'Extended Card BIN', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'enable_extended_card_bin%'

UNION ALL
SELECT 'Raw PM Details Return', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'should_return_raw_payment_method_details%'

UNION ALL
SELECT 'Process Tracker Mapping', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'pt_mapping%'

UNION ALL
SELECT 'Poll Config', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'poll_config%'

UNION ALL
SELECT 'PM Filters CGraph', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'pm_filters_cgraph%'

UNION ALL
SELECT 'Routing Result Source', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'routing_result_source%'

UNION ALL
SELECT 'Payment Update Via Client Auth', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'payment_update_enabled_for_client_auth%'

UNION ALL
SELECT 'Split Transactions Enabled', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'split_txns_enabled%'

UNION ALL
SELECT 'Webhook Config Disabled Events', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'whconf_disabled_events%'

UNION ALL
SELECT 'Connector Onboarding Config', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'onboarding_%'

UNION ALL
SELECT 'Credentials Identifier Mapping', COUNT(*), NULL::text
FROM configs WHERE key LIKE 'mcd_%'

) t ORDER BY feature;
