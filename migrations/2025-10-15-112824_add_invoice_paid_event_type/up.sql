-- Your SQL goes here
ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'invoice_paid';

ALTER TYPE "EventObjectType"
ADD VALUE IF NOT EXISTS 'subscription_details';

ALTER TYPE "EventClass"
ADD VALUE IF NOT EXISTS 'subscriptions';