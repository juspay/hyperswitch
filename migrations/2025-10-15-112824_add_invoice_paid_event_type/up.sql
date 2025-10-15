-- Your SQL goes here
ALTER TYPE "EventType"
ADD VALUE 'invoice_paid';

ALTER TYPE "EventObjectType"
ADD VALUE 'subscription_details';

ALTER TYPE "EventClass"
ADD VALUE 'subscriptions';