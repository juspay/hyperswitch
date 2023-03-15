-- Alter column type to json
-- as well as the connector.
ALTER TABLE payment_attempt
ALTER COLUMN connector TYPE JSONB
USING jsonb_build_object(
    'routed_through', connector,
    'algorithm',      CASE WHEN connector IS NULL THEN
        NULL
    ELSE
        jsonb_build_object(
            'type', 'single',
            'data', connector
        )
    END
);
