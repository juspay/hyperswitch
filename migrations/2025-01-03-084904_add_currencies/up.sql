DO $$
  DECLARE currency TEXT;
  BEGIN
    FOR currency IN
      SELECT
        unnest(
          ARRAY ['AFN', 'BTN', 'CDF', 'ERN', 'IRR', 'ISK', 'KPW', 'SDG', 'SYP', 'TJS', 'TMT', 'ZWL']
        ) AS currency
      LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM pg_enum
            WHERE enumlabel = currency
              AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'Currency')
          ) THEN EXECUTE format('ALTER TYPE "Currency" ADD VALUE %L', currency);
        END IF;
      END LOOP;
END $$;