-- Your SQL goes here
-- Appends 'payment_failed' enum variant to the list
ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'payment_failed';

ALTER TYPE "EventType" RENAME VALUE 'action_required' TO 'merchant_action_required';

ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'customer_action_required';
