-- Your SQL goes here
ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'cancelled_post_capture';

ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'voided_post_charge';

ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payment_cancelled_post_capture';