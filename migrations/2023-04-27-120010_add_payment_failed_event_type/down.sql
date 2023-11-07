DELETE FROM pg_enum
WHERE enumlabel = 'payment_failed'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
);