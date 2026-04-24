-- ============================================================================
-- Diagnostic queries — run these to understand your ClickHouse data shape
-- ============================================================================
-- Run each block separately and share back the results. I'll use them to
-- fix bucket_3.sql so it matches your actual api_flow / flow / engine values.
-- ============================================================================


-- ---- 1. Which tables have any data in last 90 days? ----
-- Tells us if any table is empty or named differently in your prod.

SELECT 'routing_events' AS tbl, count() AS cnt FROM routing_events WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'api_events', count() FROM api_events WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'authentications', count() FROM authentications WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'dispute', count() FROM dispute WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'fraud_check', count() FROM fraud_check WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'payout', count() FROM payout WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'payment_intents', count() FROM payment_intents WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'payment_attempts', count() FROM payment_attempts WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'outgoing_webhook_events', count() FROM outgoing_webhook_events WHERE created_at > now() - INTERVAL 90 DAY
UNION ALL SELECT 'connector_events', count() FROM connector_events WHERE created_at > now() - INTERVAL 90 DAY;


-- ---- 2. Distinct api_flow values (top 100 in last 7 days) ----
-- This is the CRITICAL one — shows the real handler names used in your prod.
-- Many of my bucket_3 queries use guessed values (PaymentsRetrieve, PaymentsCancel,
-- Customer%, etc.). Share this output and I'll fix them.

SELECT api_flow, count() AS cnt
FROM api_events
WHERE created_at > now() - INTERVAL 7 DAY
GROUP BY api_flow
ORDER BY cnt DESC
LIMIT 100;


-- ---- 3. Distinct connector_events.flow values ----
-- Confirms bucket_1 flow values are correct (IncrementalAuthorization, etc.)

SELECT flow, count() AS cnt
FROM connector_events
WHERE created_at > now() - INTERVAL 7 DAY
GROUP BY flow
ORDER BY cnt DESC
LIMIT 100;


-- ---- 4. routing_events breakdown ----
-- Shows what values exist for routing_engine and flow

SELECT routing_engine, count() AS cnt
FROM routing_events
WHERE created_at > now() - INTERVAL 7 DAY
GROUP BY routing_engine
ORDER BY cnt DESC;

SELECT flow, count() AS cnt
FROM routing_events
WHERE created_at > now() - INTERVAL 7 DAY
GROUP BY flow
ORDER BY cnt DESC;


-- ---- 5. payment_attempts column sanity checks ----
-- Quick yes/no checks for columns we rely on

SELECT
    countIf(mandate_id IS NOT NULL AND mandate_id != '')      AS has_mandate,
    countIf(multiple_capture_count > 1)                        AS has_multi_capture,
    countIf(surcharge_amount IS NOT NULL AND surcharge_amount > 0) AS has_surcharge,
    countIf(browser_info IS NOT NULL AND browser_info != '')   AS has_browser_info,
    countIf(connector_metadata IS NOT NULL AND connector_metadata != '') AS has_conn_meta,
    countIf(amount_capturable > amount)                        AS has_overcapture
FROM payment_attempts
WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY;
