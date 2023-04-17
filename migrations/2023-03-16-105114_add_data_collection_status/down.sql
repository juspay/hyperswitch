DELETE FROM pg_enum
WHERE enumlabel = 'device_data_collection_pending'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'AttemptStatus'
)