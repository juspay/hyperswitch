DELETE FROM pg_enum
WHERE enumlabel = 'cancelled'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'IntentStatus'
)
