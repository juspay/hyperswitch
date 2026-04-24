-- ============================================================================
-- Bucket 3 (Part B) — api_events features (single-scan, avoids timeout)
-- ============================================================================
-- Output: feature, call_count, last_seen
--
-- Scans api_events ONCE and classifies each row into a feature using multiIf.
-- Much faster than N separate UNION ALL sub-queries (api_events has ~100M rows).
--
-- If this still times out, reduce the lookback window (change 90 DAY → 30 DAY).
-- ============================================================================

SELECT
    feature,
    count()                   AS call_count,
    toString(max(created_at)) AS last_seen
FROM (
    SELECT
        created_at,
        multiIf(
            api_flow = 'DeepHealthCheck',                                    'Health Check',
            api_flow IN ('PaymentsRetrieve', 'PaymentsRetrieveForceSync'),   'Payment Sync',
            api_flow = 'PaymentsCancel',                                      'Void/Cancel Payment',
            api_flow = 'PaymentsSessionToken',                                'SDK Client Token Generation',
            api_flow = 'PaymentsStart',                                       'Auto Retries',
            api_flow = 'PaymentsRetry',                                       'Manual Retry',
            api_flow LIKE 'Customers%',                                       'Customer Management',
            api_flow LIKE 'PaymentMethods%'
                OR api_flow = 'CustomerPaymentMethodsList',                   'Payment Method Operations',
            api_flow LIKE 'MerchantConnectors%',                              'MCA Management',
            api_flow LIKE 'MerchantsAccount%',                                'Merchant Account Management',
            api_flow IN ('ProfileRetrieve', 'ListProfileForUserInOrgAndMerchant', 'SwitchProfile'),
                                                                               'Business Profile Management',
            api_flow LIKE 'ListOrg%' OR api_flow LIKE 'Organization%',        'Organization Management',
            api_flow LIKE 'CardsInfo%',                                       'Card Issuer Management',
            api_flow LIKE 'Subscription%',                                    'Subscription Management',
            api_flow IN ('ListBlocklist', 'AddToBlocklist', 'DeleteFromBlocklist'),
                                                                               'Blocklist',
            api_flow LIKE 'Oidc%' OR api_flow LIKE '%Sso%',                   'OIDC Authentication',
            api_flow LIKE 'GsmRule%',                                         'Gateway Status Map (GSM)',
            api_flow IN ('GeneratePaymentReport', 'GenerateRefundReport'),    'Reconciliation',
            api_flow LIKE 'RoutingRetrieve%',                                 'Routing Evaluate',
            api_flow LIKE 'Relay%',                                           'Relay Operations',
            ''
        ) AS feature
    FROM api_events
    WHERE created_at > now() - INTERVAL 90 DAY
)
WHERE feature != ''
GROUP BY feature
ORDER BY feature;
