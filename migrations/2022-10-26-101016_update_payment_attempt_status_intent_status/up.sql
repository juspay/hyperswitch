-- Your SQL goes here
ALTER TABLE payment_attempt ADD IF NOT EXISTS amount_to_capture INTEGER;
ALTER TYPE "CaptureMethod" ADD VALUE 'manual_multiple' AFTER 'manual';
ALTER TYPE "IntentStatus" ADD VALUE 'requires_capture';