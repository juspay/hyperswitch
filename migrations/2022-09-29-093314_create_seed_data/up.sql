INSERT INTO merchant_account (
        merchant_id,
        api_key,
        merchant_name,
        merchant_details,
        custom_routing_rules,
        publishable_key
    )
VALUES (
        'juspay_merchant',
        'MySecretApiKey',
        'Juspay Merchant',
        '{ "primary_email": "merchant@juspay.in" }',
        '[ { "connectors_pecking_order": [ "stripe" ] } ]',
        'pk_MyPublicApiKey'
    );

INSERT INTO merchant_connector_account (
        merchant_id,
        connector_name,
        connector_account_details
    )
VALUES (
        'juspay_merchant',
        'stripe',
        '{ "auth_type": "HeaderKey", "api_key": "Basic MyStripeApiKey" }'
    );
