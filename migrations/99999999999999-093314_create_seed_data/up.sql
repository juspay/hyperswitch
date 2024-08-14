INSERT INTO merchant_account (
        id,
        merchant_name,
        merchant_details,
        publishable_key,
        created_at,
        modified_at,
        organization_id
    )
VALUES (
        'juspay_merchant',
        'Juspay Merchant',
        '{ "primary_email": "merchant@juspay.in" }',
        'pk_MyPublicApiKey',
        '2024-08-12 07:59:13.000000',
        '2024-08-12 07:59:13.000000',
        'organization_id'
    );

INSERT INTO merchant_connector_account (
        merchant_id,
        connector_name,
        connector_account_details,
        profile_id,
        created_at,
        modified_at,
        id
    )
VALUES (
        'juspay_merchant',
        'stripe',
        '{ "auth_type": "HeaderKey", "api_key": "Basic MyStripeApiKey" }',
        'profile_id',
        '2024-08-12 07:59:13.000000',
        '2024-08-12 07:59:13.000000',
        'mca_123'
    );
