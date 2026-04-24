-- ============================================================================
-- Bucket 1 — Connector × Feature production usage
-- ============================================================================
-- Output columns: feature, connector, call_count, last_seen
-- Each row represents: feature X on connector Y was used N times, last seen at T
--
-- Run all sub-queries at once, or pull out individual blocks by feature.
-- Lookback window: 90 days (change `INTERVAL 90 DAY` to adjust).
--
-- FEATURES NOT QUERIED (require request-JSON inspection or config-level check):
--   Billing Descriptor, L2/L3 Data Processing, Connector Intent Metadata,
--   Connector Testing Data, Partner Merchant Identifier, Installments,
--   Network Transaction ID, Partial Authorization, Split Payments
-- ============================================================================

SELECT * FROM (

-- ---- connector_events.flow matches (dedicated connector calls) ----

SELECT 'Incremental Authorization'  AS feature, lower(connector_name) AS connector, count() AS call_count, toString(max(created_at)) AS last_seen
FROM connector_events WHERE flow = 'IncrementalAuthorization' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Extended Authorization', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'ExtendAuthorization' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Preprocessing Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'PreProcessing' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Pre-Authentication Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow IN ('PreAuthenticate', 'PreAuthentication') AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Authentication Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow IN ('Authentication', 'Authenticate') AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Post-Authentication Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow IN ('CompleteAuthorize', 'PostAuthenticate') AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Order Create Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'CreateOrder' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Settlement Split Call', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'SettlementSplitCreate' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'QR Code Generation Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'GenerateQr' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Push Notification Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'PushNotification' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Balance Check Flow', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'GiftCardBalanceCheck' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Dispute Accept', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow = 'Accept' AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

UNION ALL
SELECT 'Dispute Defend', lower(connector_name), count(), toString(max(created_at))
FROM connector_events WHERE flow IN ('Defend', 'Evidence') AND created_at > now() - INTERVAL 90 DAY GROUP BY connector_name

-- ---- Refund (from refunds table, more reliable than connector_events) ----

UNION ALL
SELECT 'Refund', lower(connector), count(), toString(max(created_at))
FROM refunds
WHERE sign_flag = 1 AND connector IS NOT NULL AND created_at > now() - INTERVAL 90 DAY
GROUP BY connector

-- ---- payment_attempts column-based detection ----

UNION ALL
SELECT 'Overcapture', lower(connector), count(), toString(max(created_at))
FROM payment_attempts
WHERE sign_flag = 1
  AND connector IS NOT NULL
  AND amount IS NOT NULL AND amount_capturable IS NOT NULL
  AND amount_capturable > amount
  AND created_at > now() - INTERVAL 90 DAY
GROUP BY connector

UNION ALL
SELECT 'Surcharge', lower(connector), count(), toString(max(created_at))
FROM payment_attempts
WHERE sign_flag = 1
  AND connector IS NOT NULL
  AND surcharge_amount IS NOT NULL AND surcharge_amount > 0
  AND created_at > now() - INTERVAL 90 DAY
GROUP BY connector

-- ---- Step Up from authentications table ----

UNION ALL
SELECT 'Step Up Authentication', lower(authentication_connector), count(), toString(max(created_at))
FROM authentications
WHERE sign_flag = 1
  AND authentication_connector IS NOT NULL
  AND trans_status IS NOT NULL
  AND message_version IS NOT NULL
  AND created_at > now() - INTERVAL 90 DAY
GROUP BY authentication_connector

-- ---- Split Refunds: payments with more than one refund row ----

UNION ALL
SELECT 'Split Refunds', lower(connector), count(), toString(max(created_at))
FROM refunds
WHERE sign_flag = 1 AND connector IS NOT NULL
  AND created_at > now() - INTERVAL 90 DAY
  AND payment_id IN (
      SELECT payment_id FROM refunds
      WHERE sign_flag = 1 AND created_at > now() - INTERVAL 90 DAY
      GROUP BY payment_id HAVING count() > 1
  )
GROUP BY connector

) ORDER BY feature, connector;
