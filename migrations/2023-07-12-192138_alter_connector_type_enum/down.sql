DELETE FROM pg_enum
WHERE enumlabel = 'payout_processor'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'ConnectorType'
)
