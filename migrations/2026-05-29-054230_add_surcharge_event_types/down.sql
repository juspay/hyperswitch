-- Note: PostgreSQL does not support removing values from ENUM types
-- To downgrade, you would need to recreate the ENUM type
DELETE FROM pg_enum
WHERE enumlabel in ('surcharge_payment_succeeded','surcharge_refund_succeeded')
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
); 
