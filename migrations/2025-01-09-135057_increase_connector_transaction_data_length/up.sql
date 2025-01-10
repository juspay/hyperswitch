DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'payment_attempt'
          AND column_name = 'connector_transaction_data'
    ) THEN
        ALTER TABLE payment_attempt 
        ALTER COLUMN connector_transaction_data TYPE VARCHAR(1024);
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'refund'
          AND column_name = 'connector_refund_data'
    ) THEN
        ALTER TABLE refund 
        ALTER COLUMN connector_refund_data TYPE VARCHAR(1024);
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'refund'
          AND column_name = 'connector_transaction_data'
    ) THEN
        ALTER TABLE refund 
        ALTER COLUMN connector_transaction_data TYPE VARCHAR(1024);
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'captures'
          AND column_name = 'connector_capture_data'
    ) THEN
        ALTER TABLE captures 
        ALTER COLUMN connector_capture_data TYPE VARCHAR(1024);
    END IF;
END $$;