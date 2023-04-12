DELETE FROM pg_enum
WHERE enumlabel = 'unresolved'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'AttemptStatus'
);
DELETE FROM pg_enum
WHERE enumlabel = 'requires_merchant_action'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'IntentStatus'
);
DELETE FROM pg_enum
WHERE enumlabel = 'action_required'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
);
DELETE FROM pg_enum
WHERE enumlabel = 'payment_processing'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
);