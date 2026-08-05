-- Add surcharge event types for connector notifications
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'surcharge_payment_succeeded';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'surcharge_refund_succeeded';
