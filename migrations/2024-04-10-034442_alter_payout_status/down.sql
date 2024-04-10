DELETE FROM pg_enum
WHERE enumlabel = 'requires_vendor_account_creation'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'PayoutStatus'
)