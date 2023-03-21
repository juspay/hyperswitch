-- This file should undo anything in `up.sql`
DELETE FROM pg_enum
WHERE enumlabel = 'bank_redirect'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'PaymentMethodType'
)
