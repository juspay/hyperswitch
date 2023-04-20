ALTER TABLE mandate
    ALTER COLUMN connector_mandate_id DROP DEFAULT,
    ALTER COLUMN connector_mandate_id TYPE jsonb 
        USING jsonb_build_object(
            'mandate_id', connector_mandate_id,
            'payment_method_id', NULL
        );