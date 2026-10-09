-- Add Review variant to IntentStatus enum
-- This status is used when a connector sends anomalous response
-- (e.g., Adyen CAPTURE_FAILED webhook after successful CAPTURE)
ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'review';
