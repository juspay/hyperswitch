ALTER TABLE mandate 
    ALTER COLUMN connector_mandate_id TYPE VARCHAR(255) USING connector_mandate_id->>'mandate_id',
    ALTER COLUMN connector_mandate_id SET DEFAULT NULL;