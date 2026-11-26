-- 2024-08-28-081757 dropped customers_pkey and made customer_id nullable so the v2
-- application could key on id, adding idx_customers_merchant_id_customer_id to
-- compensate. This reverts both; the compensating index is dropped separately.
UPDATE customers SET customer_id = id WHERE customer_id IS NULL AND id IS NOT NULL;

ALTER TABLE customers ALTER COLUMN customer_id SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'customers'::regclass AND contype = 'p'
    ) THEN
        ALTER TABLE customers
            ADD CONSTRAINT customers_pkey PRIMARY KEY USING INDEX customers_pkey;
    END IF;
END $$;
