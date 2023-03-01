-- Your SQL goes here
ALTER TYPE "EventClass" ADD VALUE 'refunds';

ALTER TYPE "EventObjectType" ADD VALUE 'refund_details';

ALTER TYPE "EventType" ADD VALUE 'refund_succeeded';

ALTER TYPE "EventType" ADD VALUE 'refund_failed';