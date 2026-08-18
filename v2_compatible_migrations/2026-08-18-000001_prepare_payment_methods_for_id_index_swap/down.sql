UPDATE payment_methods
SET locker_fingerprint_id = 'FINGERPRINT_ID_REDACTED'
WHERE locker_fingerprint_id IS NULL
  AND locker_id IS NOT NULL
  AND status = 'redacted';
