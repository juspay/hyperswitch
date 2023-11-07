-- This file should undo anything in `up.sql`
DELETE FROM pg_enum
WHERE enumlabel = 'partially_captured_and_capturable'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'IntentStatus'
);

DELETE FROM pg_enum
WHERE enumlabel = 'partial_charged_and_chargeable'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'AttemptStatus'
)