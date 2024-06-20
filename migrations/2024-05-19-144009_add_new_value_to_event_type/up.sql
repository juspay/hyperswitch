-- Your SQL goes here
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_success';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_failed';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_processing';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_cancelled';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_initiated';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_expired';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_reversed';

ALTER TYPE "EventObjectType" ADD VALUE IF NOT EXISTS 'payout_details';

ALTER TYPE "EventClass" ADD VALUE IF NOT EXISTS 'payouts';