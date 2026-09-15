-- Add payment_created event type for webhook notifications
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payment_created';
