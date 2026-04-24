-- ============================================================================
-- Bucket 3 (Part A) — Event-table features (fast)
-- ============================================================================
-- Output: feature, call_count, last_seen
-- Covers all Bucket 3 features detected via event tables OTHER than api_events.
-- Run this independently of bucket_3_api_events.sql.
-- ============================================================================

SELECT * FROM (

-- ---- Dedicated event tables ----

SELECT 'Routing Algorithm' AS feature, count() AS call_count, toString(max(created_at)) AS last_seen
FROM routing_events WHERE created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'External 3DS Authentication', count(), toString(max(created_at))
FROM authentications WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Dispute Management', count(), toString(max(created_at))
FROM dispute WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'FRM (Fraud Risk Management)', count(), toString(max(created_at))
FROM fraud_check WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Webhook Details', count(), toString(max(created_at))
FROM outgoing_webhook_events WHERE created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'External Vault', count(), toString(max(created_at))
FROM connector_events
WHERE flow = 'ExternalVaultInsertFlow' AND created_at > now() - INTERVAL 90 DAY

-- ---- payout table ----

UNION ALL
SELECT 'Payout Type', count(), toString(max(created_at))
FROM payout WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Payout Auto Fulfill', count(), toString(max(created_at))
FROM payout WHERE sign_flag = 1 AND auto_fulfill = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Payout Recurring', count(), toString(max(created_at))
FROM payout WHERE sign_flag = 1 AND recurring = 1 AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Payout Link', count(), toString(max(created_at))
FROM payout WHERE sign_flag = 1 AND return_url IS NOT NULL AND created_at > now() - INTERVAL 90 DAY

-- ---- payment_intents / payment_attempts flags (single scan per table) ----
-- Using multiIf to get all features from one table scan instead of N scans

UNION ALL
SELECT feature, count() AS call_count, toString(max(created_at)) AS last_seen
FROM (
    SELECT created_at, multiIf(
        setup_future_usage IS NOT NULL,                         'Save Card Flow',
        off_session = 1,                                         'Off Session Payments',
        return_url IS NOT NULL,                                  'Payment Link',
        ''
    ) AS feature
    FROM payment_intents
    WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY
)
WHERE feature != ''
GROUP BY feature

UNION ALL
SELECT feature, count() AS call_count, toString(max(created_at)) AS last_seen
FROM (
    SELECT created_at, arrayJoin(arrayFilter(x -> x != '', [
        if(mandate_id IS NOT NULL AND mandate_id != '', 'Mandate Management', ''),
        if(multiple_capture_count > 1, 'Multiple Capture', ''),
        if(browser_info IS NOT NULL AND browser_info != '', 'Browser Info Collection', ''),
        if(connector_metadata IS NOT NULL AND connector_metadata != '', 'Connector Metadata', ''),
        if(mandate_data IS NOT NULL AND mandate_data != '', 'Customer Acceptance', ''),
        if(mandate_id IS NOT NULL AND mandate_id != ''
            AND payment_method_data LIKE '%network_transaction_id%', 'Connector Agnostic MIT', '')
    ])) AS feature
    FROM payment_attempts
    WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY
)
GROUP BY feature

) ORDER BY feature;
