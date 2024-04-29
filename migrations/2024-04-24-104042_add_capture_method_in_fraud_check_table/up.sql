ALTER TABLE fraud_check 
ADD COLUMN IF NOT EXISTS payment_capture_method "CaptureMethod" NULL;