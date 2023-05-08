ALTER TABLE mandate
    ADD COLUMN connector_mandate_ids jsonb;
UPDATE mandate SET connector_mandate_ids = jsonb_build_object(
            'mandate_id', connector_mandate_id,
            'payment_method_id', NULL
        );