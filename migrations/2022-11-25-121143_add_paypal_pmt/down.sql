-- This file should undo anything in `up.sql`
DELETE FROM pg_enum
WHERE enumlabel = 'paypal'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'PaymentMethodType'
)
