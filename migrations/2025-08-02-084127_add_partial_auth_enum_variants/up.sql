-- Your SQL goes here
ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'partially_authorized_and_requires_capture';

ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'partially_authorized';

ALTER TYPE "EventType" ADD VALUE 'payment_partially_authorized';