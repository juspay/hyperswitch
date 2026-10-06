UPDATE payment_methods
SET locker_fingerprint_id = NULL
WHERE status = 'redacted';
