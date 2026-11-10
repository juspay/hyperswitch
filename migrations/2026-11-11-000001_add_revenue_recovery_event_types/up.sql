-- Your SQL goes here
ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'recovery_retry_scheduled';

ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'recovery_limits_exhausted';

ALTER TYPE "EventObjectType"
ADD VALUE IF NOT EXISTS 'recovery_retry_details';
