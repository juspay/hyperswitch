-- ============================================================================
-- Bucket 3 — Request-JSON and API-endpoint features (recovers 7 unknowns)
-- ============================================================================
-- Output: feature, call_count, last_seen
-- Export as CSV: ~/Downloads/bucket_3_json_features.csv
--
-- These queries use LIKE patterns on the `request` JSON blob in api_events.
-- Use a smaller window (7 days) because LIKE on a String column scans every
-- row in the window — running this over 90 days on 100M+ rows will time out.
-- If 7 days is enough to see usage, the answer is "yes".
-- ============================================================================

SELECT * FROM (

-- ---- API endpoint features (fast — api_flow is LowCardinality) ----

SELECT 'Payment Manual Update' AS feature, count() AS call_count, toString(max(created_at)) AS last_seen
FROM api_events
WHERE api_flow LIKE '%ManualUpdate%' AND api_flow LIKE 'Payments%'
  AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Refund Manual Update', count(), toString(max(created_at))
FROM api_events
WHERE api_flow LIKE '%ManualUpdate%' AND api_flow LIKE 'Refunds%'
  AND created_at > now() - INTERVAL 90 DAY

-- ---- Request-JSON features (slow — 7 day window) ----

UNION ALL
SELECT 'Order Details', count(), toString(max(created_at))
FROM api_events
WHERE api_flow IN ('PaymentsCreate', 'PaymentsConfirm', 'PaymentsUpdate')
  AND request LIKE '%"order_details"%'
  AND created_at > now() - INTERVAL 7 DAY

UNION ALL
SELECT 'Feature Metadata', count(), toString(max(created_at))
FROM api_events
WHERE api_flow IN ('PaymentsCreate', 'PaymentsConfirm', 'PaymentsUpdate')
  AND request LIKE '%"feature_metadata"%'
  AND created_at > now() - INTERVAL 7 DAY

UNION ALL
SELECT 'Payout Entity Type', count(), toString(max(created_at))
FROM api_events
WHERE api_flow = 'PayoutsCreate'
  AND request LIKE '%"entity_type"%'
  AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Payout Priority', count(), toString(max(created_at))
FROM api_events
WHERE api_flow = 'PayoutsCreate'
  AND request LIKE '%"priority"%'
  AND created_at > now() - INTERVAL 90 DAY

UNION ALL
SELECT 'Refund Type', count(), toString(max(created_at))
FROM api_events
WHERE api_flow = 'RefundsCreate'
  AND request LIKE '%"refund_type"%'
  AND created_at > now() - INTERVAL 90 DAY

) ORDER BY feature;
