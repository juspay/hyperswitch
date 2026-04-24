-- ============================================================================
-- Bucket 2 — Connector × PM × PMT × Feature production usage
-- ============================================================================
-- Output columns: feature, connector, pm, pmt, call_count, last_seen
--
-- FEATURES NOT QUERIED:
--   "Payment (Decrypt Flow)" — requires parsing request JSON for
--   PaymentMethodToken::ApplePayDecrypt / GooglePayDecrypt / PazeDecrypt.
--   Too expensive to query at scale; skip and mark prod_used = unknown.
-- ============================================================================

SELECT * FROM (

-- ---- Base Payment usage per (connector, pm, pmt) ----

SELECT 'Payment' AS feature,
       lower(connector)          AS connector,
       payment_method            AS pm,
       payment_method_type       AS pmt,
       count()                   AS call_count,
       toString(max(created_at)) AS last_seen
FROM payment_attempts
WHERE sign_flag = 1
  AND connector IS NOT NULL
  AND payment_method IS NOT NULL
  AND payment_method_type IS NOT NULL
  AND created_at > now() - INTERVAL 90 DAY
GROUP BY connector, payment_method, payment_method_type

-- ---- Mandate usage per (connector, pm, pmt) ----

UNION ALL
SELECT 'Mandate',
       lower(connector),
       payment_method,
       payment_method_type,
       count(),
       toString(max(created_at))
FROM payment_attempts
WHERE sign_flag = 1
  AND mandate_id IS NOT NULL AND mandate_id != ''
  AND connector IS NOT NULL
  AND payment_method IS NOT NULL
  AND payment_method_type IS NOT NULL
  AND created_at > now() - INTERVAL 90 DAY
GROUP BY connector, payment_method, payment_method_type

) ORDER BY feature, connector, pm, pmt;
