-- ============================================================================
-- Bucket 3 config features — Postgres: merchant_account table
-- ============================================================================
-- Output: feature, enabled_count, last_modified
-- Export as CSV: ~/Downloads/b3_pg_merchant_account.csv
-- ============================================================================

SELECT feature, enabled_count, last_modified FROM (

SELECT 'Sub-Merchants' AS feature,
       COUNT(*) FILTER (WHERE sub_merchants_enabled IS TRUE) AS enabled_count,
       MAX(modified_at) FILTER (WHERE sub_merchants_enabled IS TRUE)::text AS last_modified
FROM merchant_account

UNION ALL
SELECT 'Platform Account',
       COUNT(*) FILTER (WHERE is_platform_account IS TRUE),
       MAX(modified_at) FILTER (WHERE is_platform_account IS TRUE)::text
FROM merchant_account

UNION ALL
SELECT 'Product Type',
       COUNT(*) FILTER (WHERE product_type IS NOT NULL),
       MAX(modified_at) FILTER (WHERE product_type IS NOT NULL)::text
FROM merchant_account

UNION ALL
SELECT 'PM Collect Link',
       COUNT(*) FILTER (WHERE pm_collect_link_config IS NOT NULL),
       MAX(modified_at) FILTER (WHERE pm_collect_link_config IS NOT NULL)::text
FROM merchant_account

) t ORDER BY feature;
