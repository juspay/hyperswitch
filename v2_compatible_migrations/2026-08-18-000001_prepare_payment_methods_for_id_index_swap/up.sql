UPDATE payment_methods
SET locker_fingerprint_id = NULL
WHERE locker_fingerprint_id = 'FINGERPRINT_ID_REDACTED'
  AND status = 'redacted';
