-- This file should undo anything in `up.sql`
DELETE FROM pg_enum
WHERE enumlabel = 'refunds'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventClass'
);

DELETE FROM pg_enum
WHERE enumlabel = 'refund_details'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventObjectType'
);

DELETE FROM pg_enum
WHERE enumlabel = 'refund_succeeded'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
);

DELETE FROM pg_enum
WHERE enumlabel = 'refund_failed'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'EventType'
);